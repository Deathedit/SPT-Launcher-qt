//! Steam detection for the installer page (same logic as spt-additions'
//! guided install: a library containing `common/Escape from Tarkov/build`).

use std::path::PathBuf;

use crate::config::home;

pub const EFT_APP_ID: &str = "3932890";

#[derive(Debug, Clone)]
pub struct SteamGame {
    /// `<library>/steamapps`
    pub library: PathBuf,
    pub game_dir: PathBuf,
    pub prefix: PathBuf,
}

pub fn steam_dir() -> PathBuf {
    home().join(".steam/steam")
}

pub fn library_paths() -> Vec<PathBuf> {
    let text = std::fs::read_to_string(steam_dir().join("config/libraryfolders.vdf")).unwrap_or_default();
    text.lines()
        .filter_map(|l| {
            let l = l.trim();
            let rest = l.strip_prefix("\"path\"")?.trim();
            Some(PathBuf::from(rest.trim_matches('"').replace("\\\\", "\\")))
        })
        .collect()
}

pub fn find_eft() -> Option<SteamGame> {
    library_paths().into_iter().find_map(|lib| {
        let steamapps = lib.join("steamapps");
        let game_dir = steamapps.join("common/Escape from Tarkov/build");
        let non_empty = std::fs::read_dir(&game_dir).map(|mut rd| rd.next().is_some()).unwrap_or(false);
        non_empty.then(|| SteamGame {
            prefix: steamapps.join("compatdata").join(EFT_APP_ID).join("pfx"),
            library: steamapps,
            game_dir,
        })
    })
}

/// Whether a Steam client process is running (it rewrites shortcuts.vdf on exit)
pub fn steam_running() -> bool {
    let Ok(rd) = std::fs::read_dir("/proc") else { return false };
    rd.flatten().any(|e| {
        e.file_name().to_str().is_some_and(|s| s.bytes().all(|b| b.is_ascii_digit()))
            && std::fs::read_to_string(e.path().join("comm")).map(|c| c.trim() == "steam").unwrap_or(false)
    })
}
