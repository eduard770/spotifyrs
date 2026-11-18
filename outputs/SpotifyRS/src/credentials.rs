#[cfg(windows)]
mod windows_store {
    use std::{io, ptr, slice};
    use windows_sys::Win32::Security::Credentials::{
        CredDeleteW, CredFree, CredReadW, CredWriteW, CREDENTIALW, CRED_PERSIST_LOCAL_MACHINE,
        CRED_TYPE_GENERIC,
    };

    fn target(name: &str) -> Vec<u16> {
        name.encode_utf16().chain(Some(0)).collect()
    }

    pub fn read(name: &str) -> Option<String> {
        let mut entry = ptr::null_mut();
        // CredReadW owns the returned allocation until CredFree.
        if unsafe { CredReadW(target(name).as_ptr(), CRED_TYPE_GENERIC, 0, &mut entry) } == 0 {
            return None;
        }
        let result = unsafe {
            let entry = &*entry;
            if entry.CredentialBlobSize == 0 || entry.CredentialBlob.is_null() {
                None
            } else {
                let bytes =
                    slice::from_raw_parts(entry.CredentialBlob, entry.CredentialBlobSize as usize);
                std::str::from_utf8(bytes).ok().map(str::to_owned)
            }
        };
        unsafe { CredFree(entry.cast()) };
        result
    }

    pub fn write(name: &str, secret: &str) -> io::Result<()> {
        let mut target = target(name);
        let mut key = secret.as_bytes().to_vec();
        let entry = CREDENTIALW {
            Type: CRED_TYPE_GENERIC,
            TargetName: target.as_mut_ptr(),
            CredentialBlobSize: u32::try_from(key.len())
                .map_err(|_| io::ErrorKind::InvalidInput)?,
            CredentialBlob: key.as_mut_ptr(),
            Persist: CRED_PERSIST_LOCAL_MACHINE,
            ..Default::default()
        };
        let result = if unsafe { CredWriteW(&entry, 0) } != 0 {
            Ok(())
        } else {
            Err(io::Error::last_os_error())
        };
        key.fill(0);
        result
    }

    pub fn delete(name: &str) -> io::Result<()> {
        if unsafe { CredDeleteW(target(name).as_ptr(), CRED_TYPE_GENERIC, 0) } != 0 {
            return Ok(());
        }
        let error = io::Error::last_os_error();
        if error.raw_os_error() == Some(windows_sys::Win32::Foundation::ERROR_NOT_FOUND as i32) {
            Ok(())
        } else {
            Err(error)
        }
    }
}

#[cfg(windows)]
pub use windows_store::{delete, read, write};

#[cfg(target_os = "linux")]
mod linux_store {
    use secret_service::{blocking::SecretService, EncryptionType};
    use std::{collections::HashMap, io};

    fn attributes(name: &str) -> HashMap<&str, &str> {
        HashMap::from([("application", "spotifyrs"), ("credential", name)])
    }

    pub fn read(name: &str) -> Option<String> {
        let service = SecretService::connect(EncryptionType::Dh).ok()?;
        let items = service.search_items(attributes(name)).ok()?;
        let item = items.unlocked.first().or_else(|| items.locked.first())?;
        if item.is_locked().ok()? {
            item.unlock().ok()?;
        }
        String::from_utf8(item.get_secret().ok()?).ok()
    }

    pub fn write(name: &str, secret: &str) -> io::Result<()> {
        let service = SecretService::connect(EncryptionType::Dh).map_err(io::Error::other)?;
        let collection = service.get_default_collection().map_err(io::Error::other)?;
        if collection.is_locked().map_err(io::Error::other)? {
            collection.unlock().map_err(io::Error::other)?;
        }
        collection
            .create_item(
                name,
                attributes(name),
                secret.as_bytes(),
                true,
                "text/plain",
            )
            .map_err(io::Error::other)?;
        Ok(())
    }

    pub fn delete(name: &str) -> io::Result<()> {
        let service = SecretService::connect(EncryptionType::Dh).map_err(io::Error::other)?;
        let items = service
            .search_items(attributes(name))
            .map_err(io::Error::other)?;
        for item in items.unlocked.into_iter().chain(items.locked) {
            if item.is_locked().map_err(io::Error::other)? {
                item.unlock().map_err(io::Error::other)?;
            }
            item.delete().map_err(io::Error::other)?;
        }
        Ok(())
    }
}

#[cfg(target_os = "linux")]
pub use linux_store::{delete, read, write};

#[cfg(not(any(windows, target_os = "linux")))]
pub fn read(_: &str) -> Option<String> {
    None
}

#[cfg(not(any(windows, target_os = "linux")))]
pub fn write(_: &str, _: &str) -> std::io::Result<()> {
    Err(std::io::ErrorKind::Unsupported.into())
}

#[cfg(not(any(windows, target_os = "linux")))]
pub fn delete(_: &str) -> std::io::Result<()> {
    Err(std::io::ErrorKind::Unsupported.into())
}

#[cfg(all(test, any(windows, target_os = "linux")))]
#[test]
#[cfg_attr(
    target_os = "linux",
    ignore = "requer um cofre Secret Service desbloqueado"
)]
fn reads_and_removes_a_test_credential() {
    let target = format!("SpotifyRS/Test/{}", rand::random::<u64>());
    let secret = "test credential";
    write(&target, secret).unwrap();
    let stored = read(&target);
    let replacement = "updated credential";
    write(&target, replacement).unwrap();
    let updated = read(&target);
    delete(&target).unwrap();
    assert_eq!(stored.as_deref(), Some(secret));
    assert_eq!(updated.as_deref(), Some(replacement));
    assert!(read(&target).is_none());
    delete(&target).unwrap();
}
