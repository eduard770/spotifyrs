use crate::model::{self, Item};
use serde::Deserialize;
use std::sync::Arc;

pub const PAGE_SIZE: usize = 20;
pub const SAVED: &str = "saved";

#[derive(Clone, Deserialize)]
pub struct Playlist {
    pub id: String,
    pub name: String,
    pub images: Option<Vec<CoverUrl>>,
    #[serde(skip)]
    pub cover: Option<Arc<Cover>>,
}

#[derive(Clone, Deserialize)]
pub struct CoverUrl {
    pub url: String,
    pub width: Option<u32>,
}

pub struct Cover {
    pub pixels: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Default)]
pub struct Library {
    pub ready: bool,
    pub playlists: Arc<[Playlist]>,
    pub playlist_offset: usize,
    pub more_playlists: bool,
    pub note: String,
    pub saved_preview: Arc<[Item]>,
    pub selection: String,
    pub tracks: Arc<[Item]>,
    pub track_offset: usize,
    pub more_tracks: bool,
    pub track_note: String,
}

pub fn valid_id(id: &str) -> bool {
    id.len() == 22 && id.bytes().all(|b| b.is_ascii_alphanumeric())
}

pub fn demo() -> Library {
    Library {
        playlists: [
            ("demo-noite", "Depois do anoitecer"),
            ("demo-calma", "Dias tranquilos"),
            ("demo-viagem", "Em viagem"),
        ]
        .map(|(id, name)| Playlist {
            id: id.into(),
            name: name.into(),
            images: None,
            cover: None,
        })
        .into(),
        ..Library::default()
    }
}

pub fn demo_tracks(id: &str) -> Vec<Item> {
    let indices: &[usize] = match id {
        SAVED => &[0, 3, 5],
        "demo-noite" => &[1, 2, 4, 5],
        "demo-calma" => &[0, 4, 2],
        "demo-viagem" => &[3, 5, 1, 0],
        _ => &[],
    };
    indices.iter().copied().map(model::demo_track).collect()
}
