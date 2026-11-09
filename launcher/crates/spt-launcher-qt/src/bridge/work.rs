//! Actions behind the `Launcher` invokables.
//!
//! Rule of thumb: read what is needed under the `Inner` lock, drop the lock,
//! then touch Qt properties (setters emit change signals synchronously and QML
//! may call straight back into an invokable).

use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use cxx_qt::{CxxQtThread, Threading};
use cxx_qt_lib::QString;
use serde_json::json;
use spt_core::api::{self, Client};
use spt_core::bundles::{self, format_bytes};
use spt_core::config::{AppConf, LauncherSettings, SptInstall, UiState};
use spt_core::game::{self, ClientCheck, LaunchOptions};
use spt_core::models::{Editions, MiniProfile, ModPage};
use spt_core::process::{self, ManagedProcess};
use spt_core::text::classify;
use spt_core::{proton, steam};

use super::launcher::qobject::Launcher;

type L = Launcher;
type Qt = CxxQtThread<L>;

#[derive(Default)]
pub struct Inner {
    spt_path: PathBuf,
    ui: UiState,
    server: Option<Arc<ManagedProcess>>,
    server_restart: bool,
    client: Option<Client>,
    conn_gen: u64,
    profiles: Vec<MiniProfile>,
    game: Option<Arc<ManagedProcess>>,
    launch_cancel: Arc<AtomicBool>,
    task: Option<Arc<ManagedProcess>>,
    task_cancelled: bool,
}

fn q(s: &str) -> QString {
    QString::from(s)
}

fn now_ms() -> f64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as f64).unwrap_or(0.0)
}

fn log(qt: &Qt, channel: &'static str, level: &str, text: impl Into<String>) {
    let (level, text) = (level.to_string(), text.into());
    let _ = qt.queue(move |o| o.log_line(q(channel), q(&level), q(&text)));
}

fn log_now(o: Pin<&mut L>, channel: &str, level: &str, text: &str) {
    o.log_line(q(channel), q(level), q(text));
}

fn shell_quote(s: &str) -> String {
    if !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || "-_./=:,+@".contains(c)) {
        s.to_string()
    } else {
        format!("'{}'", s.replace('\'', "'\\''"))
    }
}

fn command_line(cmd: &Command) -> String {
    std::iter::once(cmd.get_program().to_string_lossy().into_owned())
        .chain(cmd.get_args().map(|a| shell_quote(&a.to_string_lossy())))
        .collect::<Vec<_>>()
        .join(" ")
}

fn install_of(o: &L) -> SptInstall {
    SptInstall::new(o.inner().lock().unwrap().spt_path.clone())
}

/// LauncherSettings with prefix / Proton filled in from app.conf when the official
/// launcher never ran (its settings are created on first start)
fn effective_settings(install: &SptInstall, conf: &AppConf) -> LauncherSettings {
    let mut s = LauncherSettings::load(install);
    if s.prefix_path().is_empty() {
        if let Some(p) = conf.get("WINEPREFIX") {
            s.set_prefix_path(&p);
        }
    }
    if s.proton_path().is_empty() {
        if let Some(p) = conf.get("PROTONPATH").filter(|p| p.starts_with('/')) {
            s.set_proton_path(&p);
        }
    }
    s
}

fn host_for(o: &L) -> String {
    if o.target().to_string() == "local" {
        api::LOCAL_HOST.to_string()
    } else {
        api::normalize_host(&o.remote_host().to_string())
    }
}

// ---------------------------------------------------------------------------
// State / config
// ---------------------------------------------------------------------------

pub fn init(mut o: Pin<&mut L>) {
    let ui = UiState::load();
    o.as_mut().set_target(q(&ui.target));
    o.as_mut().set_remote_host(q(&ui.remote_host));
    o.as_mut().set_tray_enabled(ui.tray);
    o.as_mut().set_hide_on_play(ui.hide_on_play);
    o.inner().lock().unwrap().ui = ui;
    refresh(o.as_mut());

    // A server that is already running (started from a terminal / shortcut)
    let qt = o.qt_thread();
    let local = *o.local_enabled();
    let remote = o.remote_host().to_string();
    let target = o.target().to_string();
    std::thread::spawn(move || {
        let up = local && Client::new(api::LOCAL_HOST).ping().is_ok();
        let _ = qt.queue(move |mut o| {
            if up && o.server_state().to_string() == "off" {
                o.as_mut().set_server_state(q("external"));
                o.as_mut().set_server_started_at(now_ms());
                log_now(o.as_mut(), "server", "ok", "A server is already running at 127.0.0.1:6969 (started outside this launcher)");
            }
            if *o.installed() && ((target == "local" && up) || (target == "remote" && !remote.is_empty())) {
                connect(o);
            }
        });
    });
}

pub fn refresh(mut o: Pin<&mut L>) {
    let conf = AppConf::load();
    let path = spt_core::config::resolve_spt_path(&conf);
    let install = SptInstall::new(&path);
    let settings = effective_settings(&install, &conf);

    let version = game::pe_file_version(&install.core_dll()).map(|v| format!("{}.{}.{}", v.0, v.1, v.2)).unwrap_or_default();
    let server_available = process::which("spt-server").is_some();
    let local = conf.server_enabled() && server_available;
    let protons: Vec<_> = proton::list(&settings.extra_proton_paths())
        .into_iter()
        .map(|p| json!({ "name": p.name, "path": p.path.to_string_lossy(), "kind": p.kind }))
        .collect();

    o.inner().lock().unwrap().spt_path = path.clone();

    o.as_mut().set_spt_path(q(&path.to_string_lossy()));
    o.as_mut().set_installed(install.is_installed());
    o.as_mut().set_legacy(install.is_legacy());
    o.as_mut().set_spt_version(q(&version));
    o.as_mut().set_prefix_path(q(&settings.prefix_path()));
    o.as_mut().set_proton_path(q(&settings.proton_path()));
    o.as_mut().set_launch_env(q(&settings.launch_settings()));
    o.as_mut().set_game_mode(settings.game_mode());
    o.as_mut().set_clear_cache(settings.clear_cache_on_launch());
    o.as_mut().set_server_available(server_available);
    o.as_mut().set_local_enabled(local);
    o.as_mut().set_fika_installed(install.fika_installed());
    o.as_mut().set_steam_game_dir(q(&steam::find_eft().map(|g| g.game_dir.to_string_lossy().into_owned()).unwrap_or_default()));
    o.as_mut().set_protons_json(q(&serde_json::to_string(&protons).unwrap_or_default()));

    if !local && o.target().to_string() == "local" {
        set_target(o, "remote".into());
    }
}

pub fn set_tray_options(mut o: Pin<&mut L>, tray: bool, hide_on_play: bool) {
    o.as_mut().set_tray_enabled(tray);
    o.as_mut().set_hide_on_play(hide_on_play);
    {
        let inner = o.inner();
        let mut g = inner.lock().unwrap();
        g.ui.tray = tray;
        g.ui.hide_on_play = hide_on_play;
    }
    save_ui(&o);
}

fn save_ui(o: &L) {
    let ui = o.inner().lock().unwrap().ui.clone();
    let _ = ui.save();
}

fn clear_connection(mut o: Pin<&mut L>) {
    {
        let inner = o.inner();
        let mut g = inner.lock().unwrap();
        g.conn_gen += 1;
        g.client = None;
        g.profiles.clear();
    }
    o.as_mut().set_conn_state(q("none"));
    o.as_mut().set_conn_error(QString::default());
    o.as_mut().set_server_version(QString::default());
    o.as_mut().set_client_check(QString::default());
    o.as_mut().set_client_check_text(QString::default());
    o.as_mut().set_profiles_json(q("[]"));
    o.as_mut().set_mods_json(q("[]"));
    o.as_mut().set_selected_id(QString::default());
}

pub fn set_target(mut o: Pin<&mut L>, target: String) {
    let target = if target == "local" && *o.local_enabled() { "local" } else { "remote" };
    if o.target().to_string() == target {
        return;
    }
    if o.game_state().to_string() != "idle" {
        return;
    }
    o.as_mut().set_target(q(target));
    o.inner().lock().unwrap().ui.target = target.to_string();
    save_ui(&o);
    clear_connection(o.as_mut());
    let st = o.server_state().to_string();
    if target == "local" && (st == "running" || st == "external") {
        connect(o);
    }
}

pub fn set_remote(mut o: Pin<&mut L>, host: String) {
    let host = api::normalize_host(&host);
    if o.remote_host().to_string() == host {
        return;
    }
    o.as_mut().set_remote_host(q(&host));
    o.inner().lock().unwrap().ui.remote_host = host;
    save_ui(&o);
    if o.target().to_string() == "remote" {
        clear_connection(o);
    }
}

pub fn select_proton(mut o: Pin<&mut L>, path: String) {
    let install = install_of(&o);
    let mut conf = AppConf::load();
    let mut s = effective_settings(&install, &conf);
    s.set_proton_path(&path);
    conf.set("PROTONPATH", &path);
    let res = conf.save().and_then(|_| if install.is_installed() { s.save() } else { Ok(()) });
    match res {
        Ok(()) => {
            o.as_mut().set_proton_path(q(&path));
            let name = Path::new(&path).file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or(path);
            o.toast(q("ok"), q(&format!("Using {name}")));
        }
        Err(e) => o.toast(q("error"), q(&format!("Could not save: {e:#}"))),
    }
}

pub fn save_settings(mut o: Pin<&mut L>, spt_path: String, launch_env: String, game_mode: bool, clear_cache: bool) {
    let mut conf = AppConf::load();
    let spt_path = spt_path.trim().trim_end_matches('/').to_string();
    if !spt_path.is_empty() && conf.get("spt-path").as_deref() != Some(spt_path.as_str()) {
        conf.set("spt-path", &spt_path);
        if let Err(e) = conf.save() {
            o.toast(q("error"), q(&format!("Could not save app.conf: {e:#}")));
            return;
        }
    }
    let install = SptInstall::new(spt_core::config::resolve_spt_path(&conf));
    if install.is_installed() {
        let mut s = effective_settings(&install, &conf);
        s.set_launch_settings(launch_env.trim());
        s.set_game_mode(game_mode);
        s.set_clear_cache_on_launch(clear_cache);
        if let Err(e) = s.save() {
            o.toast(q("error"), q(&format!("Could not save LauncherSettings.json: {e:#}")));
            return;
        }
    }
    refresh(o.as_mut());
    o.toast(q("ok"), q("Settings saved"));
}

pub fn set_local_enabled(mut o: Pin<&mut L>, enabled: bool) {
    let mut conf = AppConf::load();
    conf.set("server", if enabled { "1" } else { "0" });
    if let Err(e) = conf.save() {
        o.toast(q("error"), q(&format!("Could not save app.conf: {e:#}")));
        return;
    }
    if !enabled && o.server_state().to_string() != "off" {
        stop_server(o.as_mut(), false);
    }
    refresh(o.as_mut());
    if enabled && !*o.server_available() {
        o.toast(q("warn"), q("spt-server is not installed (client-only package). Install spt-launcher-qt instead of the -client variant."));
    }
}

// ---------------------------------------------------------------------------
// Connection & profiles
// ---------------------------------------------------------------------------

fn profiles_json(profiles: &[MiniProfile]) -> String {
    let v: Vec<_> = profiles
        .iter()
        .map(|p| {
            json!({
                "id": p.profile_id,
                "username": p.username,
                "nick": p.display_name(),
                "side": if p.wipe || p.side.is_empty() { "–".to_string() } else { p.side.clone() },
                "level": p.current_level,
                "xpPct": p.xp_percent(),
                "edition": p.edition,
                "roubles": p.currency.roubles,
                "fresh": p.wipe,
                "invalid": p.invalid,
            })
        })
        .collect();
    serde_json::to_string(&v).unwrap_or_else(|_| "[]".into())
}

fn editions_json(e: &Editions) -> String {
    let v: Vec<_> = e.iter().map(|(name, desc)| json!({ "name": name, "desc": desc })).collect();
    serde_json::to_string(&v).unwrap_or_else(|_| "[]".into())
}

fn mods_json(m: &[ModPage]) -> String {
    let v: Vec<_> = m
        .iter()
        .map(|p| json!({ "name": p.name, "url": p.home_page, "desc": p.description.clone().unwrap_or_default() }))
        .collect();
    serde_json::to_string(&v).unwrap_or_else(|_| "[]".into())
}

/// Updates the profile list and keeps (or picks) a selection
fn apply_profiles(mut o: Pin<&mut L>, profiles: Vec<MiniProfile>, prefer_username: Option<&str>) {
    let host = o.conn_host().to_string();
    let current = o.selected_id().to_string();
    let selected = {
        let inner = o.inner();
        let mut g = inner.lock().unwrap();
        let remembered = g.ui.last_profile.get(&host).cloned().unwrap_or_default();
        let pick = prefer_username
            .and_then(|u| profiles.iter().find(|p| p.username.eq_ignore_ascii_case(u)))
            .or_else(|| profiles.iter().find(|p| p.profile_id == current))
            .or_else(|| profiles.iter().find(|p| p.profile_id == remembered))
            .or_else(|| profiles.iter().find(|p| !p.invalid))
            .map(|p| p.profile_id.clone())
            .unwrap_or_default();
        g.profiles = profiles.clone();
        if !pick.is_empty() {
            g.ui.last_profile.insert(host, pick.clone());
        }
        pick
    };
    save_ui(&o);
    o.as_mut().set_profiles_json(q(&profiles_json(&profiles)));
    o.as_mut().set_selected_id(q(&selected));
}

pub fn connect(mut o: Pin<&mut L>) {
    let host = host_for(&o);
    if host.is_empty() {
        o.toast(q("warn"), q("Enter the server address first (host:port)"));
        return;
    }
    if o.game_state().to_string() != "idle" {
        return;
    }
    let gen = {
        let inner = o.inner();
        let mut g = inner.lock().unwrap();
        g.conn_gen += 1;
        g.conn_gen
    };
    o.as_mut().set_conn_state(q("connecting"));
    o.as_mut().set_conn_host(q(&host));
    o.as_mut().set_conn_error(QString::default());
    log_now(o.as_mut(), "game", "cmd", &format!("GET https://{host}{}", api::urls::PING));

    let qt = o.qt_thread();
    let install = install_of(&o);
    std::thread::spawn(move || {
        let client = Client::new(&host);
        let result = (|| -> anyhow::Result<_> {
            client.ping()?;
            let version = client.version().unwrap_or_default();
            let profiles = client.profiles()?;
            let editions = client.editions().unwrap_or_default();
            let mods = client.mod_pages().unwrap_or_default();
            let check = if install.is_installed() && !version.is_empty() { Some(game::check_client(&install, &version)) } else { None };
            Ok((version, profiles, editions, mods, check))
        })();
        let _ = qt.queue(move |mut o| {
            if o.inner().lock().unwrap().conn_gen != gen {
                return; // superseded by a newer connect / disconnect
            }
            match result {
                Ok((version, profiles, editions, mods, check)) => {
                    o.inner().lock().unwrap().client = Some(client);
                    let (cc, cc_text) = match check {
                        Some(Ok(ClientCheck::Match(c))) => ("match", format!("spt-core.dll {c} matches")),
                        Some(Ok(ClientCheck::Mismatch(c, s))) => ("mismatch", format!("Client {c} ≠ server {s}")),
                        Some(Ok(ClientCheck::Skipped(c))) => ("skipped", format!("Dev build modules ({c})")),
                        Some(Err(e)) => ("error", format!("{e:#}")),
                        None => ("", String::new()),
                    };
                    o.as_mut().set_server_version(q(version.split(" - ").next().unwrap_or("").trim()));
                    o.as_mut().set_client_check(q(cc));
                    o.as_mut().set_client_check_text(q(&cc_text));
                    o.as_mut().set_editions_json(q(&editions_json(&editions)));
                    o.as_mut().set_mods_json(q(&mods_json(&mods)));
                    let n = profiles.len();
                    apply_profiles(o.as_mut(), profiles, None);
                    o.as_mut().set_conn_state(q("ok"));
                    let msg = format!(
                        "Pong! · server {} · {n} profile{}",
                        if version.is_empty() { "?" } else { &version },
                        if n == 1 { "" } else { "s" }
                    );
                    log_now(o.as_mut(), "game", "ok", &msg);
                    if cc == "mismatch" {
                        log_now(o.as_mut(), "game", "warn", &format!("{cc_text}: the game may fail to connect. Update SPT (spt-additions update spt)."));
                    }
                }
                Err(e) => {
                    let msg = format!("{e:#}");
                    o.as_mut().set_conn_state(q("fail"));
                    o.as_mut().set_conn_error(q(&msg));
                    log_now(o.as_mut(), "game", "error", &format!("No answer: {msg}"));
                }
            }
        });
    });
}

pub fn select_profile(mut o: Pin<&mut L>, id: String) {
    if o.game_state().to_string() != "idle" {
        return;
    }
    let host = o.conn_host().to_string();
    o.inner().lock().unwrap().ui.last_profile.insert(host, id.clone());
    save_ui(&o);
    o.as_mut().set_selected_id(q(&id));
}

fn client_or_toast(o: &mut Pin<&mut L>) -> Option<Client> {
    let c = o.inner().lock().unwrap().client.clone();
    if c.is_none() {
        o.as_mut().toast(q("warn"), q("Not connected to a server"));
    }
    c
}

pub fn create_profile(mut o: Pin<&mut L>, username: String, edition: String) {
    let username = username.trim().to_string();
    if username.is_empty() || *o.busy() {
        return;
    }
    let Some(client) = client_or_toast(&mut o) else { return };
    o.as_mut().set_busy(true);
    log_now(o.as_mut(), "game", "cmd", &format!("PUT {} · {username} · {edition}", api::urls::REGISTER));
    let qt = o.qt_thread();
    std::thread::spawn(move || {
        let r = client.register(&username, &edition);
        let _ = qt.queue(move |mut o| {
            o.as_mut().set_busy(false);
            match r {
                Ok(profiles) => {
                    apply_profiles(o.as_mut(), profiles, Some(&username));
                    log_now(o.as_mut(), "game", "ok", &format!("Created profile \"{username}\" ({edition})"));
                    o.toast(q("ok"), q(&format!("Profile \"{username}\" created. Create your character in game.")));
                }
                Err(e) => {
                    log_now(o.as_mut(), "game", "error", &format!("{e:#}"));
                    o.toast(q("error"), q(&format!("{e:#}")));
                }
            }
        });
    });
}

pub enum ProfileAction {
    Wipe,
    Remove,
}

pub fn profile_action(mut o: Pin<&mut L>, id: String, action: ProfileAction) {
    if *o.busy() || o.game_state().to_string() != "idle" {
        return;
    }
    let profile = o.inner().lock().unwrap().profiles.iter().find(|p| p.profile_id == id).cloned();
    let Some(profile) = profile else { return };
    let Some(client) = client_or_toast(&mut o) else { return };
    o.as_mut().set_busy(true);
    let url = match action {
        ProfileAction::Wipe => api::urls::WIPE,
        ProfileAction::Remove => api::urls::REMOVE,
    };
    log_now(o.as_mut(), "game", "cmd", &format!("PUT {url} · {}", profile.username));
    let qt = o.qt_thread();
    std::thread::spawn(move || {
        let r = match action {
            ProfileAction::Wipe => client.wipe(&profile.username, &profile.edition).map(|p| (p, "wiped")),
            ProfileAction::Remove => client.remove(&profile.username).map(|p| (p, "removed")),
        };
        let _ = qt.queue(move |mut o| {
            o.as_mut().set_busy(false);
            match r {
                Ok((profiles, what)) => {
                    apply_profiles(o.as_mut(), profiles, None);
                    log_now(o.as_mut(), "game", "warn", &format!("Profile \"{}\" {what}", profile.username));
                }
                Err(e) => {
                    log_now(o.as_mut(), "game", "error", &format!("{e:#}"));
                    o.toast(q("error"), q(&format!("{e:#}")));
                }
            }
        });
    });
}

// ---------------------------------------------------------------------------
// Launch
// ---------------------------------------------------------------------------

fn set_phase(qt: &Qt, phase: i32, info: impl Into<String>) {
    let info = info.into();
    let _ = qt.queue(move |mut o| {
        o.as_mut().set_launch_phase(phase);
        o.as_mut().set_phase_info(q(&info));
    });
}

pub fn launch(mut o: Pin<&mut L>) {
    if o.game_state().to_string() != "idle" {
        return;
    }
    if !*o.installed() {
        o.toast(q("warn"), q("SPT is not installed"));
        return;
    }
    let id = o.selected_id().to_string();
    let (client, profile, cancel) = {
        let inner = o.inner();
        let mut g = inner.lock().unwrap();
        g.launch_cancel = Arc::new(AtomicBool::new(false));
        (g.client.clone(), g.profiles.iter().find(|p| p.profile_id == id).cloned(), g.launch_cancel.clone())
    };
    let (Some(client), Some(profile)) = (client, profile) else {
        o.toast(q("warn"), q("Connect to a server and pick a profile first"));
        return;
    };
    if profile.invalid {
        o.toast(q("error"), q("This profile can't be loaded by the server"));
        return;
    }

    o.as_mut().set_game_state(q("launching"));
    o.as_mut().set_launch_phase(0);
    o.as_mut().set_phase_info(QString::default());
    o.as_mut().set_bundle_pct(0.0);
    o.as_mut().set_launch_error(QString::default());

    let qt = o.qt_thread();
    let install = install_of(&o);
    let inner = o.inner();
    std::thread::spawn(move || {
        let fail = |msg: String| {
            log(&qt, "game", "error", msg.clone());
            let _ = qt.queue(move |mut o| {
                o.as_mut().set_game_state(q("idle"));
                o.as_mut().set_launch_phase(-1);
                o.as_mut().set_launch_error(q(&msg));
                o.toast(q("error"), q(&msg));
            });
        };
        let cancelled = || cancel.load(Ordering::Relaxed);
        let conf = AppConf::load();
        let settings = effective_settings(&install, &conf);

        // 0 · check client
        log(&qt, "game", "cmd", format!("PUT {} · {}", api::urls::LOGIN, profile.username));
        if let Err(e) = client.login(&profile.username) {
            return fail(format!("Login failed: {e:#}"));
        }
        let version = match client.version() {
            Ok(v) => v,
            Err(e) => return fail(format!("Lost connection to the server: {e:#}")),
        };
        match game::check_client(&install, &version) {
            Ok(ClientCheck::Match(c)) => log(&qt, "game", "ok", format!("spt-core.dll {c} matches server {}", version.trim())),
            Ok(ClientCheck::Mismatch(c, s)) => {
                log(&qt, "game", "warn", format!("spt-core.dll {c} does not match server {s}: the game may fail to connect"))
            }
            Ok(ClientCheck::Skipped(c)) => log(&qt, "game", "warn", format!("Locally built modules ({c}), version check skipped")),
            Err(e) => return fail(format!("{e:#}")),
        }
        let removed = game::setup_game_files(&install, &settings);
        if !removed.is_empty() {
            log(&qt, "game", "info", format!("Removed: {}", removed.join(", ")));
        }
        if settings.clear_cache_on_launch() {
            match game::clear_cache(&install) {
                Ok(()) => log(&qt, "game", "info", "Cleared the game cache (user/sptappdata)"),
                Err(e) => log(&qt, "game", "warn", format!("{e:#}")),
            }
        }
        if cancelled() {
            return fail("Launch cancelled".into());
        }

        // 1 · patch
        set_phase(&qt, 1, "");
        let hpatchz = process::which("hpatchz").map(|p| p.to_string_lossy().into_owned()).unwrap_or_else(|| "hpatchz".into());
        let qt2 = qt.clone();
        match game::patch_game(&install, &hpatchz, |i, n, f| {
            if n > 0 && !f.is_empty() {
                set_phase(&qt2, 1, format!("{}/{n} · {}", i + 1, f.rsplit('/').next().unwrap_or(f)));
            }
        }) {
            Ok(n) => log(&qt, "game", "ok", format!("Game files prepared ({n} patch{})", if n == 1 { "" } else { "es" })),
            Err(e) => return fail(format!("Patching failed: {e:#}")),
        }
        if cancelled() {
            return fail("Launch cancelled".into());
        }

        // 2 · bundles
        set_phase(&qt, 2, "");
        let qt3 = qt.clone();
        match bundles::acquire(&client, &install, &cancel, |p| {
            let info = if p.total_bytes > 0 {
                format!(
                    "{}/{} · {} / {} · {}/s",
                    p.done,
                    p.total,
                    format_bytes(p.bytes as f64),
                    format_bytes(p.total_bytes as f64),
                    format_bytes(p.bytes_per_sec)
                )
            } else {
                format!("{}/{}", p.done, p.total)
            };
            let pct = p.percent();
            let _ = qt3.queue(move |mut o| {
                o.as_mut().set_bundle_pct(pct);
                o.as_mut().set_phase_info(q(&info));
            });
        }) {
            Ok(p) if p.total == 0 => log(&qt, "game", "ok", "The server has no mod bundles"),
            Ok(p) if p.total_bytes == 0 => log(&qt, "game", "ok", format!("Bundles up to date ({} total)", p.total)),
            Ok(p) => log(&qt, "game", "ok", format!("Downloaded {} of bundles", format_bytes(p.total_bytes as f64))),
            Err(e) => return fail(format!("Mod bundles: {e:#}")),
        }
        if cancelled() {
            return fail("Launch cancelled".into());
        }

        // 3 · start
        set_phase(&qt, 3, "umu-run EscapeFromTarkov.exe");
        let umu = process::which("umu-run")
            .map(|p| p.to_string_lossy().into_owned())
            .or_else(|| Some(settings.umu_path()).filter(|p| !p.is_empty()))
            .unwrap_or_else(|| "umu-run".into());
        let opts = LaunchOptions { profile_id: profile.profile_id.clone(), backend_url: client.backend_url(), umu_run: umu };
        let cmd = match game::launch_command(&install, &settings, &opts) {
            Ok(c) => c,
            Err(e) => return fail(format!("{e:#}")),
        };
        log(&qt, "game", "cmd", format!("$ {}", command_line(&cmd)));
        let qt4 = qt.clone();
        let proc = match ManagedProcess::spawn(cmd, move |_, line| {
            let lv = classify(&line);
            log(&qt4, "game", if lv == "ok" { "info" } else { lv }, line);
        }) {
            Ok(p) => Arc::new(p),
            Err(e) => return fail(format!("{e:#}")),
        };
        inner.lock().unwrap().game = Some(proc.clone());
        let _ = qt.queue(|mut o| {
            o.as_mut().set_launch_phase(4);
            o.as_mut().set_phase_info(QString::default());
            o.as_mut().set_game_state(q("running"));
        });
        log(&qt, "game", "ok", format!("Game started as {} · {}", profile.display_name(), client.host()));

        // Wait for umu-run, then for any game process it left behind
        let status = proc.wait();
        std::thread::sleep(Duration::from_secs(3));
        while !game::game_pids().is_empty() {
            std::thread::sleep(Duration::from_secs(3));
        }
        inner.lock().unwrap().game = None;
        let code = status.and_then(|s| s.code()).map(|c| format!(" (exit code {c})")).unwrap_or_default();
        log(&qt, "game", "info", format!("Game closed{code}"));
        let _ = qt.queue(|mut o| {
            o.as_mut().set_game_state(q("idle"));
            o.as_mut().set_launch_phase(-1);
        });
    });
}

pub fn stop_game(mut o: Pin<&mut L>) {
    let (game, cancel) = {
        let inner = o.inner();
        let g = inner.lock().unwrap();
        (g.game.clone(), g.launch_cancel.clone())
    };
    match o.game_state().to_string().as_str() {
        "launching" => {
            cancel.store(true, Ordering::Relaxed);
            log_now(o.as_mut(), "game", "warn", "Cancelling launch…");
        }
        "running" => {
            if let Some(g) = game {
                g.terminate();
            }
            process::terminate_pids(&game::game_pids());
            log_now(o.as_mut(), "game", "warn", "Stopping the game (SIGTERM)");
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// Local server
// ---------------------------------------------------------------------------

pub fn start_server(mut o: Pin<&mut L>) {
    if !*o.local_enabled() {
        o.toast(q("warn"), q("The local server is disabled (client-only mode)"));
        return;
    }
    if o.server_state().to_string() != "off" {
        return;
    }
    let Some(bin) = process::which("spt-server") else {
        o.toast(q("error"), q("spt-server not found on PATH"));
        return;
    };
    let install = install_of(&o);
    if !install.is_installed() {
        o.toast(q("warn"), q("SPT is not installed"));
        return;
    }
    let mut cmd = Command::new(bin);
    cmd.arg(&install.root).env("SPT_SERVER_NO_TERMINAL", "1");
    log_now(o.as_mut(), "server", "cmd", &format!("$ spt-server {}", shell_quote(&install.root.to_string_lossy())));

    let qt = o.qt_thread();
    let qt_lines = qt.clone();
    let proc = match ManagedProcess::spawn(cmd, move |_, line| {
        let lv = classify(&line);
        log(&qt_lines, "server", lv, line);
    }) {
        Ok(p) => Arc::new(p),
        Err(e) => {
            o.toast(q("error"), q(&format!("{e:#}")));
            return;
        }
    };
    o.inner().lock().unwrap().server = Some(proc.clone());
    o.as_mut().set_server_state(q("starting"));

    let inner = o.inner();
    std::thread::spawn(move || {
        let ping = Client::new(api::LOCAL_HOST);
        while proc.try_wait().is_none() {
            if ping.ping().is_ok() {
                let _ = qt.queue(|mut o| {
                    if o.server_state().to_string() == "starting" {
                        o.as_mut().set_server_state(q("running"));
                        o.as_mut().set_server_started_at(now_ms());
                        if o.target().to_string() == "local" {
                            connect(o);
                        }
                    }
                });
                break;
            }
            std::thread::sleep(Duration::from_secs(1));
        }
        let status = proc.wait();
        let restart = {
            let mut g = inner.lock().unwrap();
            g.server = None;
            std::mem::take(&mut g.server_restart)
        };
        let msg = match status.and_then(|s| s.code()) {
            Some(0) | None => "Server stopped".to_string(),
            Some(c) => format!("Server exited with code {c}"),
        };
        log(&qt, "server", "warn", msg);
        let _ = qt.queue(move |mut o| {
            o.as_mut().set_server_state(q("off"));
            o.as_mut().set_server_started_at(0.0);
            if o.target().to_string() == "local" {
                clear_connection(o.as_mut());
            }
            if restart {
                start_server(o);
            }
        });
    });
}

pub fn stop_server(mut o: Pin<&mut L>, restart: bool) {
    let state = o.server_state().to_string();
    if state == "external" {
        o.toast(q("warn"), q("This server was started outside the launcher; stop it where it runs"));
        return;
    }
    let server = {
        let inner = o.inner();
        let mut g = inner.lock().unwrap();
        g.server_restart = restart;
        g.server.clone()
    };
    let Some(server) = server else { return };
    log_now(o.as_mut(), "server", "warn", if restart { "Restarting the server…" } else { "Stopping the server (SIGTERM)…" });
    server.terminate();
    o.as_mut().set_server_state(q("stopping"));
    std::thread::spawn(move || {
        for _ in 0..50 {
            if server.try_wait().is_some() {
                return;
            }
            std::thread::sleep(Duration::from_millis(200));
        }
        server.kill();
    });
}

pub fn send_server_input(mut o: Pin<&mut L>, line: String) {
    let server = o.inner().lock().unwrap().server.clone();
    match server {
        Some(s) => {
            log_now(o.as_mut(), "server", "cmd", &format!("› {line}"));
            if let Err(e) = s.write_line(&line) {
                o.toast(q("error"), q(&format!("{e:#}")));
            }
        }
        None => o.toast(q("warn"), q("The server is not running from this launcher")),
    }
}

// ---------------------------------------------------------------------------
// spt-additions tasks (install, Steam, Fika, Proton update, uninstall)
// ---------------------------------------------------------------------------

/// Runs `spt-additions --no-prompt --no-ansi <args>` for each argument list, in order
pub fn run_tasks(mut o: Pin<&mut L>, name: String, steps: Vec<Vec<String>>) {
    if o.task_state().to_string() == "running" {
        o.toast(q("warn"), q("Another task is still running"));
        return;
    }
    if process::which("spt-additions").is_none() {
        o.toast(q("error"), q("spt-additions not found on PATH"));
        return;
    }
    o.as_mut().set_task_state(q("running"));
    o.as_mut().set_task_name(q(&name));
    o.inner().lock().unwrap().task_cancelled = false;

    let qt = o.qt_thread();
    let inner = o.inner();
    std::thread::spawn(move || {
        let mut ok = true;
        for args in steps {
            let mut cmd = Command::new("spt-additions");
            cmd.args(["--no-prompt", "--no-ansi"]).args(&args).env("NO_ANSI", "1");
            log(&qt, "task", "cmd", format!("$ {}", command_line(&cmd)));
            let qt_lines = qt.clone();
            let proc = match ManagedProcess::spawn(cmd, move |_, line| {
                let lv = classify(&line);
                log(&qt_lines, "task", lv, line);
            }) {
                Ok(p) => Arc::new(p),
                Err(e) => {
                    log(&qt, "task", "error", format!("{e:#}"));
                    ok = false;
                    break;
                }
            };
            inner.lock().unwrap().task = Some(proc.clone());
            let status = proc.wait();
            let cancelled = {
                let mut g = inner.lock().unwrap();
                g.task = None;
                g.task_cancelled
            };
            if cancelled || !status.map(|s| s.success()).unwrap_or(false) {
                ok = false;
                if !cancelled {
                    let code = status.and_then(|s| s.code()).map(|c| c.to_string()).unwrap_or_else(|| "signal".into());
                    log(&qt, "task", "error", format!("spt-additions failed (exit {code}). Log: ~/.cache/spt-additions/installer.log"));
                }
                break;
            }
        }
        let cancelled = inner.lock().unwrap().task_cancelled;
        let _ = qt.queue(move |mut o| {
            let state = if ok { "done" } else if cancelled { "cancelled" } else { "failed" };
            o.as_mut().set_task_state(q(state));
            let name = o.task_name().to_string();
            refresh(o.as_mut());
            match state {
                "done" => o.toast(q("ok"), q(&format!("{name}: done"))),
                "cancelled" => o.toast(q("warn"), q(&format!("{name}: cancelled"))),
                _ => o.toast(q("error"), q(&format!("{name}: failed, see the log"))),
            }
        });
    });
}

pub fn cancel_task(mut o: Pin<&mut L>) {
    let task = {
        let inner = o.inner();
        let mut g = inner.lock().unwrap();
        g.task_cancelled = true;
        g.task.clone()
    };
    if let Some(t) = task {
        t.terminate();
        log_now(o.as_mut(), "task", "warn", "Interrupted · run it again to resume");
    }
}

pub fn run_install(o: Pin<&mut L>, spt_path: String, local: bool, add_steam: bool) {
    let spt_path = spt_path.trim().trim_end_matches('/').to_string();
    if spt_path.is_empty() {
        o.toast(q("warn"), q("Choose an SPT directory"));
        return;
    }
    let Some(game) = steam::find_eft() else {
        o.toast(q("warn"), q("No Steam copy of the game found. Use the guided installer (terminal) instead."));
        return;
    };
    if add_steam && steam::steam_running() {
        o.toast(q("warn"), q("Close Steam first (Steam → Exit): it overwrites non-Steam game entries on exit"));
        return;
    }
    // Same as the guided installer's Steam branch
    let mut conf = AppConf::load();
    conf.set("steam-library-path", &game.library.to_string_lossy());
    if let Err(e) = conf.save() {
        o.toast(q("error"), q(&format!("Could not save app.conf: {e:#}")));
        return;
    }
    let mut steps = vec![vec![
        "-p".to_string(),
        game.prefix.to_string_lossy().into_owned(),
        "-s".to_string(),
        spt_path,
        if local { "--with-server" } else { "--no-server" }.to_string(),
        "install".to_string(),
        "spt".to_string(),
    ]];
    if add_steam {
        steps.push(vec!["steam".to_string()]);
    }
    run_tasks(o, "Install SPT".into(), steps);
}

pub fn open_official_launcher(mut o: Pin<&mut L>) {
    let Some(bin) = process::which("spt-launcher") else {
        o.toast(q("error"), q("spt-launcher not found on PATH"));
        return;
    };
    let install = install_of(&o);
    let mut cmd = Command::new(bin);
    cmd.arg(&install.root);
    log_now(o.as_mut(), "game", "cmd", &format!("$ spt-launcher {}", shell_quote(&install.root.to_string_lossy())));
    if let Err(e) = process::spawn_detached(cmd) {
        o.toast(q("error"), q(&format!("{e:#}")));
    }
}

pub fn open_guided_installer(o: Pin<&mut L>) {
    const TERMINALS: &[&str] = &[
        "alacritty", "ghostty", "foot", "terminator", "ptyxis", "cosmic-terminal", "kgx", "konsole", "gnome-terminal",
        "xfce4-terminal", "kitty", "wezterm", "xterm",
    ];
    let Some(term) = TERMINALS.iter().find(|t| process::which(t).is_some()) else {
        o.toast(q("error"), q("No terminal emulator found. Run spt-additions in a terminal."));
        return;
    };
    let mut cmd = Command::new(term);
    match *term {
        "gnome-terminal" | "kgx" | "ptyxis" => cmd.arg("--"),
        _ => cmd.arg("-e"),
    };
    cmd.arg("spt-additions");
    if let Err(e) = process::spawn_detached(cmd) {
        o.toast(q("error"), q(&format!("{e:#}")));
    } else {
        o.toast(q("ok"), q("Guided installer opened in a terminal. Press Refresh when it's done."));
    }
}
