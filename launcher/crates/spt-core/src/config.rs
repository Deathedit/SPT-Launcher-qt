//! Configuration sources:
//!
//! - `~/.config/spt-additions/app.conf`: written by the spt-additions installer
//!   (`spt-path`, `WINEPREFIX`, `PROTONPATH`, `server`, ...)
//! - `<SPT>/SPT_Runtime/user/Launcher/LauncherSettings.json`: the official
//!   launcher's settings. Shared, so both launchers launch the game the same way;
//!   unknown fields are preserved on save.
//! - `~/.config/spt-additions/launcher-qt.json`: our UI state (last server, profile)

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub fn home() -> PathBuf {
    std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("/"))
}

pub fn config_dir() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .unwrap_or_else(|| home().join(".config"))
        .join("spt-additions")
}

// ---------------------------------------------------------------------------
// app.conf (key="value" lines)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
pub struct AppConf {
    path: PathBuf,
    lines: Vec<String>,
}

impl AppConf {
    pub fn path() -> PathBuf {
        config_dir().join("app.conf")
    }

    pub fn load() -> Self {
        let path = Self::path();
        let lines = fs::read_to_string(&path).map(|s| s.lines().map(str::to_owned).collect()).unwrap_or_default();
        Self { path, lines }
    }

    pub fn get(&self, key: &str) -> Option<String> {
        self.lines.iter().rev().find_map(|l| {
            let (k, v) = l.split_once('=')?;
            (k.trim() == key).then(|| v.trim().trim_matches('"').to_string())
        })
    }

    /// Same format as the script's `set_value`: replaces the key, keeps other lines
    pub fn set(&mut self, key: &str, value: &str) {
        self.lines.retain(|l| l.split_once('=').map(|(k, _)| k.trim() != key).unwrap_or(true));
        self.lines.push(format!("{key}=\"{value}\""));
    }

    pub fn save(&self) -> Result<()> {
        if let Some(dir) = self.path.parent() {
            fs::create_dir_all(dir)?;
        }
        let mut text = self.lines.join("\n");
        text.push('\n');
        write_atomic(&self.path, text.as_bytes())
    }

    /// `server="0"` = client-only install
    pub fn server_enabled(&self) -> bool {
        self.get("server").as_deref() != Some("0") && std::env::var("SPT_NO_SERVER").as_deref() != Ok("1")
    }
}

/// SPT directory: `$SPT_PATH`, then `spt-path` from app.conf, then `~/Games/SPT`
/// (same order as the spt-server / spt-launcher wrappers)
pub fn resolve_spt_path(conf: &AppConf) -> PathBuf {
    if let Some(p) = std::env::var_os("SPT_PATH").filter(|p| !p.is_empty()) {
        return PathBuf::from(p);
    }
    conf.get("spt-path").filter(|p| !p.is_empty()).map(PathBuf::from).unwrap_or_else(|| home().join("Games/SPT"))
}

// ---------------------------------------------------------------------------
// SPT install layout
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct SptInstall {
    /// The SPT directory, also the game directory (EscapeFromTarkov.exe)
    pub root: PathBuf,
}

impl SptInstall {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn runtime_dir(&self) -> PathBuf {
        self.root.join("SPT_Runtime")
    }

    pub fn is_installed(&self) -> bool {
        self.root.join("EscapeFromTarkov.exe").is_file() && self.runtime_dir().is_dir()
    }

    /// SPT 4.0 ("legacy") layout, not supported by this launcher
    pub fn is_legacy(&self) -> bool {
        self.root.join("SPT").join("SPT.Server.Linux").is_file() && !self.runtime_dir().is_dir()
    }

    pub fn launcher_settings_path(&self) -> PathBuf {
        self.runtime_dir().join("user/Launcher/LauncherSettings.json")
    }

    pub fn patches_dir(&self) -> PathBuf {
        self.runtime_dir().join("SPT_Data/Launcher/Patches")
    }

    pub fn bundle_cache_dir(&self) -> PathBuf {
        self.runtime_dir().join("user/cache/bundles")
    }

    pub fn app_data_dir(&self) -> PathBuf {
        self.runtime_dir().join("user/sptappdata")
    }

    pub fn core_dll(&self) -> PathBuf {
        self.root.join("BepInEx/plugins/spt/spt-core.dll")
    }

    pub fn fika_installed(&self) -> bool {
        fs::read_dir(self.root.join("BepInEx/plugins"))
            .map(|rd| rd.flatten().any(|e| e.file_name().to_string_lossy().to_lowercase().starts_with("fika")))
            .unwrap_or(false)
    }
}

// ---------------------------------------------------------------------------
// LauncherSettings.json
// ---------------------------------------------------------------------------

/// A user-added server (`Servers` in LauncherSettings.json)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ServerEntry {
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "IpAddress")]
    pub ip_address: String,
    #[serde(rename = "ServerId")]
    pub server_id: String,
}

#[derive(Debug, Clone)]
pub struct LauncherSettings {
    path: PathBuf,
    doc: Value,
}

impl LauncherSettings {
    /// Loads the file, or the official launcher's defaults if it does not exist yet
    pub fn load(install: &SptInstall) -> Self {
        let path = install.launcher_settings_path();
        let doc = fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str::<Value>(s.trim_start_matches('\u{feff}')).ok())
            .filter(Value::is_object)
            .unwrap_or_else(|| Self::defaults(&install.root));
        Self { path, doc }
    }

    fn defaults(root: &Path) -> Value {
        json!({
            "Servers": [],
            "StartLocation": { "X": 0, "Y": 0 },
            "StartSize": { "Height": 0, "Width": 0 },
            "FirstRun": true,
            "EnableGamePath": false,
            "GamePath": root.to_string_lossy(),
            "CloseToTray": false,
            "MinimizeOnLaunch": true,
            "CloseOnGameStart": false,
            "AlwaysTop": false,
            "UseBackground": true,
            "ClearCacheOnLaunch": false,
            "AutoConnectLastProfile": false,
            "PreferredProfile": null,
            "ExcludeFromCleanup": [],
            "Language": "en",
            "LinuxSettings": {
                "PrefixPath": "",
                "UmuPath": "",
                "LaunchSettings": "",
                "ProtonVersion": "",
                "ProtonPaths": [],
                "GameMode": false,
                "DefaultEnv": "WINEDLLOVERRIDES=\"winhttp=n,b\""
            },
            "DebugLogging": false
        })
    }

    pub fn save(&self) -> Result<()> {
        if let Some(dir) = self.path.parent() {
            fs::create_dir_all(dir)?;
        }
        write_atomic(&self.path, serde_json::to_string_pretty(&self.doc)?.as_bytes())
            .with_context(|| format!("writing {}", self.path.display()))
    }

    fn linux(&self) -> &Value {
        &self.doc["LinuxSettings"]
    }

    fn linux_mut(&mut self) -> &mut serde_json::Map<String, Value> {
        let obj = self.doc.as_object_mut().expect("settings root is an object");
        let entry = obj.entry("LinuxSettings").or_insert_with(|| json!({}));
        if !entry.is_object() {
            *entry = json!({});
        }
        entry.as_object_mut().unwrap()
    }

    fn linux_str(&self, key: &str) -> String {
        self.linux()[key].as_str().unwrap_or_default().to_string()
    }

    pub fn prefix_path(&self) -> String {
        self.linux_str("PrefixPath")
    }
    pub fn proton_path(&self) -> String {
        self.linux_str("ProtonVersion")
    }
    pub fn launch_settings(&self) -> String {
        self.linux_str("LaunchSettings")
    }
    pub fn default_env(&self) -> String {
        match self.linux()["DefaultEnv"].as_str() {
            Some(s) => s.to_string(),
            None => "WINEDLLOVERRIDES=\"winhttp=n,b\"".to_string(),
        }
    }
    pub fn umu_path(&self) -> String {
        self.linux_str("UmuPath")
    }
    pub fn game_mode(&self) -> bool {
        self.linux()["GameMode"].as_bool().unwrap_or(false)
    }
    pub fn extra_proton_paths(&self) -> Vec<PathBuf> {
        self.linux()["ProtonPaths"]
            .as_array()
            .map(|a| a.iter().filter_map(|v| v.as_str()).map(PathBuf::from).collect())
            .unwrap_or_default()
    }

    pub fn set_prefix_path(&mut self, v: &str) {
        self.linux_mut().insert("PrefixPath".into(), v.into());
    }
    pub fn set_proton_path(&mut self, v: &str) {
        self.linux_mut().insert("ProtonVersion".into(), v.into());
    }
    pub fn set_launch_settings(&mut self, v: &str) {
        self.linux_mut().insert("LaunchSettings".into(), v.into());
    }
    pub fn set_umu_path(&mut self, v: &str) {
        self.linux_mut().insert("UmuPath".into(), v.into());
    }
    pub fn set_game_mode(&mut self, v: bool) {
        self.linux_mut().insert("GameMode".into(), v.into());
    }

    pub fn clear_cache_on_launch(&self) -> bool {
        self.doc["ClearCacheOnLaunch"].as_bool().unwrap_or(false)
    }
    pub fn set_clear_cache_on_launch(&mut self, v: bool) {
        self.doc["ClearCacheOnLaunch"] = v.into();
    }

    pub fn exclude_from_cleanup(&self) -> Vec<String> {
        self.doc["ExcludeFromCleanup"]
            .as_array()
            .map(|a| a.iter().filter_map(|v| v.as_str()).map(str::to_owned).collect())
            .unwrap_or_default()
    }

    pub fn servers(&self) -> Vec<ServerEntry> {
        serde_json::from_value(self.doc["Servers"].clone()).unwrap_or_default()
    }

    /// Remembers a remote host in the shared server list (so the official launcher sees it too)
    pub fn remember_server(&mut self, host: &str) {
        let mut servers = self.servers();
        if servers.iter().any(|s| s.ip_address == host) {
            return;
        }
        let id = format!("{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0));
        servers.push(ServerEntry { name: host.to_string(), ip_address: host.to_string(), server_id: id });
        self.doc["Servers"] = serde_json::to_value(servers).unwrap_or_else(|_| json!([]));
    }
}

// ---------------------------------------------------------------------------
// Our UI state
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct UiState {
    /// "local" or "remote"
    pub target: String,
    pub remote_host: String,
    /// host -> last selected profile id
    pub last_profile: BTreeMap<String, String>,
    /// Show a system tray icon; closing the window hides it to the tray
    pub tray: bool,
    /// Hide the window to the tray while the game runs
    pub hide_on_play: bool,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            target: "local".into(),
            remote_host: String::new(),
            last_profile: BTreeMap::new(),
            tray: true,
            hide_on_play: true,
        }
    }
}

impl UiState {
    fn path() -> PathBuf {
        config_dir().join("launcher-qt.json")
    }

    pub fn load() -> Self {
        fs::read_to_string(Self::path()).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::path();
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        write_atomic(&path, serde_json::to_string_pretty(self)?.as_bytes())
    }
}

pub fn write_atomic(path: &Path, data: &[u8]) -> Result<()> {
    let tmp = path.with_extension("tmp-spt");
    fs::write(&tmp, data)?;
    fs::rename(&tmp, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_conf_roundtrip() {
        let mut c = AppConf { path: PathBuf::from("/nonexistent"), lines: vec!["server=\"0\"".into(), "spt-path=\"/a\"".into()] };
        assert!(!c.server_enabled() || std::env::var("SPT_NO_SERVER").is_ok());
        assert_eq!(c.get("spt-path").as_deref(), Some("/a"));
        c.set("server", "1");
        assert_eq!(c.get("server").as_deref(), Some("1"));
        assert_eq!(c.lines.len(), 2);
    }

    #[test]
    fn settings_preserve_unknown_fields() {
        let mut s = LauncherSettings {
            path: PathBuf::from("/nonexistent"),
            doc: json!({"Foo": 1, "LinuxSettings": {"Bar": true, "GameMode": false}}),
        };
        s.set_game_mode(true);
        s.set_proton_path("/p");
        assert_eq!(s.doc["Foo"], 1);
        assert_eq!(s.doc["LinuxSettings"]["Bar"], true);
        assert!(s.game_mode());
        assert_eq!(s.proton_path(), "/p");
        assert_eq!(s.default_env(), "WINEDLLOVERRIDES=\"winhttp=n,b\"");
    }
}
