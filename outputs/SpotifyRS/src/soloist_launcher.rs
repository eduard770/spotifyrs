use crate::{credentials, soloist};
use std::{
    env, fs,
    net::{SocketAddr, TcpStream},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::atomic::{AtomicBool, Ordering},
    thread,
    time::{Duration, Instant},
};

const EXE: &str = if cfg!(windows) {
    "soloist.exe"
} else {
    "soloist"
};

pub struct LocalSoloist(Child);

impl Drop for LocalSoloist {
    fn drop(&mut self) {
        if matches!(self.0.try_wait(), Ok(None)) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
}

fn path_file() -> Option<PathBuf> {
    Some(PathBuf::from(env::var_os("LOCALAPPDATA")?).join("SpotifyRS/soloist-path.txt"))
}

fn is_soloist(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.eq_ignore_ascii_case(EXE))
        && path.is_file()
}

pub fn find_executable() -> Option<PathBuf> {
    let stored = path_file()
        .and_then(|file| fs::read_to_string(file).ok())
        .map(PathBuf::from);
    let beside_app = env::current_exe()
        .ok()
        .and_then(|file| Some(file.parent()?.join(EXE)));
    let profile = env::var_os("USERPROFILE").map(PathBuf::from);
    let common_places = [
        "Documents/spotifyrs/soloist_release_windows_x86_64",
        "Downloads/soloist_release_windows_x86_64",
        "Documents/spotifyrs",
    ];
    stored
        .into_iter()
        .chain(beside_app)
        .chain(
            common_places
                .into_iter()
                .filter_map(|folder| Some(profile.as_ref()?.join(folder).join(EXE))),
        )
        .find(|path| is_soloist(path))
}

pub fn start(
    address: SocketAddr,
    executable: &str,
    key: &str,
    remember: bool,
    cancelled: &AtomicBool,
) -> Result<(Option<LocalSoloist>, Option<String>), String> {
    let entered = key.trim();
    if TcpStream::connect_timeout(&address, Duration::from_millis(250)).is_ok() {
        soloist::request(address, "get_auth_state").map_err(|_| {
            "A porta está ocupada. Confirma o endereço do Soloist em Ligações.".to_string()
        })?;
        let note = if remember && !entered.is_empty() {
            match saved_key() {
                Some(saved) if saved == entered => None,
                Some(_) => Some("Para trocar a chave guardada, usa «Esquecer chave» primeiro."),
                None if store_key(entered).is_ok() => {
                    Some("Chave guardada. Será usada no próximo arranque do Soloist.")
                }
                None => Some("O Windows não guardou a chave."),
            }
        } else {
            None
        };
        return Ok((None, note.map(str::to_owned)));
    }
    let path = Path::new(executable.trim().trim_matches('"'));
    if !path.is_absolute() || !is_soloist(path) {
        return Err(format!(
            "Indica o caminho completo do executável oficial {EXE}."
        ));
    }
    let saved = if entered.is_empty() {
        saved_key()
    } else {
        None
    };
    let key = saved.as_deref().unwrap_or(entered);
    if key.is_empty() {
        return Err("Introduz a tua chave Soloist para iniciar a reprodução local.".into());
    }
    let mut command = Command::new(path);
    command
        .args(["--device-name", "SpotifyRS local", "--api-key", key, "--ws"])
        .arg(address.to_string())
        .args(["--cache-size", "100"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }
    let mut process = LocalSoloist(
        command
            .spawn()
            .map_err(|error| format!("Não foi possível iniciar o Soloist: {error}"))?,
    );
    let deadline = Instant::now() + Duration::from_secs(8);
    while Instant::now() < deadline {
        if cancelled.load(Ordering::Relaxed) {
            return Err("Arranque do Soloist cancelado.".into());
        }
        if let Some(status) = process
            .0
            .try_wait()
            .map_err(|_| "Não foi possível verificar o processo Soloist.")?
        {
            return Err(if status.code() == Some(10) {
                "Esta versão do Soloist expirou. Descarrega a versão oficial mais recente.".into()
            } else {
                format!("O Soloist terminou ao iniciar ({status}). Verifica a chave, a versão e se já existe outra instância.")
            });
        }
        if TcpStream::connect_timeout(&address, Duration::from_millis(150)).is_ok() {
            soloist::request(address, "get_auth_state")?;
            if let Some(file) = path_file() {
                if let Some(parent) = file.parent() {
                    let _ = fs::create_dir_all(parent)
                        .and_then(|_| fs::write(&file, path.to_string_lossy().as_bytes()));
                }
            }
            let warning = (remember && !entered.is_empty() && store_key(key).is_err())
                .then(|| "O Soloist arrancou, mas o Windows não guardou a chave.".to_string());
            return Ok((Some(process), warning));
        }
        thread::sleep(Duration::from_millis(100));
    }
    Err("O Soloist não ficou disponível. Verifica a chave e se a porta está livre.".into())
}

const KEY_TARGET: &str = "SpotifyRS/Soloist";

pub fn saved_key() -> Option<String> {
    credentials::read(KEY_TARGET)
}

fn store_key(key: &str) -> std::io::Result<()> {
    credentials::write(KEY_TARGET, key)
}

pub fn forget_key() -> std::io::Result<()> {
    credentials::delete(KEY_TARGET)
}
