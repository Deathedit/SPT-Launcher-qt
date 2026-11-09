//! `Launcher` QObject declaration and its thin invokable wrappers.
//! The actual work lives in [`super::work`] (worker threads calling spt-core).

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        // install & paths
        #[qproperty(bool, installed)]
        #[qproperty(bool, legacy)]
        #[qproperty(QString, spt_path, cxx_name = "sptPath")]
        #[qproperty(QString, spt_version, cxx_name = "sptVersion")]
        #[qproperty(QString, prefix_path, cxx_name = "prefixPath")]
        #[qproperty(QString, proton_path, cxx_name = "protonPath")]
        #[qproperty(QString, launch_env, cxx_name = "launchEnv")]
        #[qproperty(bool, game_mode, cxx_name = "gameMode")]
        #[qproperty(bool, clear_cache, cxx_name = "clearCache")]
        #[qproperty(bool, local_enabled, cxx_name = "localEnabled")]
        #[qproperty(bool, server_available, cxx_name = "serverAvailable")]
        #[qproperty(bool, fika_installed, cxx_name = "fikaInstalled")]
        #[qproperty(QString, steam_game_dir, cxx_name = "steamGameDir")]
        #[qproperty(QString, protons_json, cxx_name = "protonsJson")]
        #[qproperty(QString, fonts_dir, cxx_name = "fontsDir")]
        // local server
        #[qproperty(QString, server_state, cxx_name = "serverState")]
        #[qproperty(f64, server_started_at, cxx_name = "serverStartedAt")]
        // connection
        #[qproperty(QString, target)]
        #[qproperty(QString, remote_host, cxx_name = "remoteHost")]
        #[qproperty(QString, conn_state, cxx_name = "connState")]
        #[qproperty(QString, conn_host, cxx_name = "connHost")]
        #[qproperty(QString, conn_error, cxx_name = "connError")]
        #[qproperty(QString, server_version, cxx_name = "serverVersion")]
        #[qproperty(QString, client_check, cxx_name = "clientCheck")]
        #[qproperty(QString, client_check_text, cxx_name = "clientCheckText")]
        // profiles
        #[qproperty(QString, profiles_json, cxx_name = "profilesJson")]
        #[qproperty(QString, selected_id, cxx_name = "selectedId")]
        #[qproperty(QString, editions_json, cxx_name = "editionsJson")]
        #[qproperty(QString, mods_json, cxx_name = "modsJson")]
        #[qproperty(bool, busy)]
        // game
        #[qproperty(QString, game_state, cxx_name = "gameState")]
        #[qproperty(i32, launch_phase, cxx_name = "launchPhase")]
        #[qproperty(QString, phase_info, cxx_name = "phaseInfo")]
        #[qproperty(f64, bundle_pct, cxx_name = "bundlePct")]
        #[qproperty(QString, launch_error, cxx_name = "launchError")]
        // spt-additions tasks
        #[qproperty(QString, task_state, cxx_name = "taskState")]
        #[qproperty(QString, task_name, cxx_name = "taskName")]
        // tray (launcher-qt.json)
        #[qproperty(bool, tray_enabled, cxx_name = "trayEnabled")]
        #[qproperty(bool, hide_on_play, cxx_name = "hideOnPlay")]
        type Launcher = super::LauncherRust;

        /// A log line: channel is "server", "game" or "task"; level "info", "ok", "warn", "error", "cmd"
        #[qsignal]
        #[cxx_name = "logLine"]
        fn log_line(self: Pin<&mut Launcher>, channel: QString, level: QString, text: QString);

        /// Short notification for the UI (kind: "ok", "warn", "error")
        #[qsignal]
        fn toast(self: Pin<&mut Launcher>, kind: QString, text: QString);
    }

    #[auto_cxx_name]
    extern "RustQt" {
        #[qinvokable]
        fn refresh(self: Pin<&mut Launcher>);
        #[qinvokable]
        fn set_connect_target(self: Pin<&mut Launcher>, target: &QString);
        #[qinvokable]
        fn set_remote(self: Pin<&mut Launcher>, host: &QString);
        #[qinvokable]
        fn connect_server(self: Pin<&mut Launcher>);

        #[qinvokable]
        fn start_server(self: Pin<&mut Launcher>);
        #[qinvokable]
        fn stop_server(self: Pin<&mut Launcher>);
        #[qinvokable]
        fn restart_server(self: Pin<&mut Launcher>);
        #[qinvokable]
        fn send_server_input(self: Pin<&mut Launcher>, line: &QString);

        #[qinvokable]
        fn select_profile(self: Pin<&mut Launcher>, id: &QString);
        #[qinvokable]
        fn create_profile(self: Pin<&mut Launcher>, username: &QString, edition: &QString);
        #[qinvokable]
        fn wipe_profile(self: Pin<&mut Launcher>, id: &QString);
        #[qinvokable]
        fn remove_profile(self: Pin<&mut Launcher>, id: &QString);

        #[qinvokable]
        fn launch(self: Pin<&mut Launcher>);
        #[qinvokable]
        fn stop_game(self: Pin<&mut Launcher>);

        #[qinvokable]
        fn select_proton(self: Pin<&mut Launcher>, path: &QString);
        #[qinvokable]
        fn save_settings(
            self: Pin<&mut Launcher>,
            spt_path: &QString,
            launch_env: &QString,
            game_mode: bool,
            clear_cache: bool,
        );
        #[qinvokable]
        fn enable_local(self: Pin<&mut Launcher>, enabled: bool);
        #[qinvokable]
        fn set_tray_options(self: Pin<&mut Launcher>, tray: bool, hide_on_play: bool);

        #[qinvokable]
        fn run_install(self: Pin<&mut Launcher>, spt_path: &QString, local: bool, steam: bool);
        /// Runs `spt-additions <args>`; args is a JSON array of strings
        #[qinvokable]
        fn run_cli(self: Pin<&mut Launcher>, name: &QString, args_json: &QString);
        #[qinvokable]
        fn cancel_task(self: Pin<&mut Launcher>);
        #[qinvokable]
        fn open_official_launcher(self: Pin<&mut Launcher>);
        #[qinvokable]
        fn open_guided_installer(self: Pin<&mut Launcher>);
        #[qinvokable]
        fn steam_running(self: &Launcher) -> bool;
        /// Environment variable (used for screenshot / start-page overrides)
        #[qinvokable]
        fn env_var(self: &Launcher, name: &QString) -> QString;
    }

    impl cxx_qt::Threading for Launcher {}
    impl cxx_qt::Initialize for Launcher {}
}

use std::pin::Pin;
use std::sync::{Arc, Mutex};

use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;

use super::work::{self, Inner};

pub struct LauncherRust {
    installed: bool,
    legacy: bool,
    spt_path: QString,
    spt_version: QString,
    prefix_path: QString,
    proton_path: QString,
    launch_env: QString,
    game_mode: bool,
    clear_cache: bool,
    local_enabled: bool,
    server_available: bool,
    fika_installed: bool,
    steam_game_dir: QString,
    protons_json: QString,
    fonts_dir: QString,
    server_state: QString,
    server_started_at: f64,
    target: QString,
    remote_host: QString,
    conn_state: QString,
    conn_host: QString,
    conn_error: QString,
    server_version: QString,
    client_check: QString,
    client_check_text: QString,
    profiles_json: QString,
    selected_id: QString,
    editions_json: QString,
    mods_json: QString,
    busy: bool,
    game_state: QString,
    launch_phase: i32,
    phase_info: QString,
    bundle_pct: f64,
    launch_error: QString,
    task_state: QString,
    task_name: QString,
    tray_enabled: bool,
    hide_on_play: bool,

    pub(crate) inner: Arc<Mutex<Inner>>,
}

impl Default for LauncherRust {
    fn default() -> Self {
        Self {
            installed: false,
            legacy: false,
            spt_path: QString::default(),
            spt_version: QString::default(),
            prefix_path: QString::default(),
            proton_path: QString::default(),
            launch_env: QString::default(),
            game_mode: false,
            clear_cache: false,
            local_enabled: false,
            server_available: false,
            fika_installed: false,
            steam_game_dir: QString::default(),
            protons_json: QString::from("[]"),
            fonts_dir: QString::from(std::env::var("SPT_LAUNCHER_FONTS").unwrap_or_default().as_str()),
            server_state: QString::from("off"),
            server_started_at: 0.0,
            target: QString::from("local"),
            remote_host: QString::default(),
            conn_state: QString::from("none"),
            conn_host: QString::default(),
            conn_error: QString::default(),
            server_version: QString::default(),
            client_check: QString::default(),
            client_check_text: QString::default(),
            profiles_json: QString::from("[]"),
            selected_id: QString::default(),
            editions_json: QString::from("[]"),
            mods_json: QString::from("[]"),
            busy: false,
            game_state: QString::from("idle"),
            launch_phase: -1,
            phase_info: QString::default(),
            bundle_pct: 0.0,
            launch_error: QString::default(),
            task_state: QString::from("idle"),
            task_name: QString::default(),
            tray_enabled: true,
            hide_on_play: true,
            inner: Arc::new(Mutex::new(Inner::default())),
        }
    }
}

impl cxx_qt::Initialize for qobject::Launcher {
    fn initialize(self: Pin<&mut Self>) {
        work::init(self);
    }
}

impl qobject::Launcher {
    pub(crate) fn inner(&self) -> Arc<Mutex<Inner>> {
        self.rust().inner.clone()
    }

    pub fn refresh(self: Pin<&mut Self>) {
        work::refresh(self);
    }
    pub fn set_connect_target(self: Pin<&mut Self>, target: &QString) {
        work::set_target(self, target.to_string());
    }
    pub fn set_remote(self: Pin<&mut Self>, host: &QString) {
        work::set_remote(self, host.to_string());
    }
    pub fn connect_server(self: Pin<&mut Self>) {
        work::connect(self);
    }
    pub fn start_server(self: Pin<&mut Self>) {
        work::start_server(self);
    }
    pub fn stop_server(self: Pin<&mut Self>) {
        work::stop_server(self, false);
    }
    pub fn restart_server(self: Pin<&mut Self>) {
        work::stop_server(self, true);
    }
    pub fn send_server_input(self: Pin<&mut Self>, line: &QString) {
        work::send_server_input(self, line.to_string());
    }
    pub fn select_profile(self: Pin<&mut Self>, id: &QString) {
        work::select_profile(self, id.to_string());
    }
    pub fn create_profile(self: Pin<&mut Self>, username: &QString, edition: &QString) {
        work::create_profile(self, username.to_string(), edition.to_string());
    }
    pub fn wipe_profile(self: Pin<&mut Self>, id: &QString) {
        work::profile_action(self, id.to_string(), work::ProfileAction::Wipe);
    }
    pub fn remove_profile(self: Pin<&mut Self>, id: &QString) {
        work::profile_action(self, id.to_string(), work::ProfileAction::Remove);
    }
    pub fn launch(self: Pin<&mut Self>) {
        work::launch(self);
    }
    pub fn stop_game(self: Pin<&mut Self>) {
        work::stop_game(self);
    }
    pub fn select_proton(self: Pin<&mut Self>, path: &QString) {
        work::select_proton(self, path.to_string());
    }
    pub fn save_settings(self: Pin<&mut Self>, spt_path: &QString, launch_env: &QString, game_mode: bool, clear_cache: bool) {
        work::save_settings(self, spt_path.to_string(), launch_env.to_string(), game_mode, clear_cache);
    }
    pub fn enable_local(self: Pin<&mut Self>, enabled: bool) {
        work::set_local_enabled(self, enabled);
    }
    pub fn set_tray_options(self: Pin<&mut Self>, tray: bool, hide_on_play: bool) {
        work::set_tray_options(self, tray, hide_on_play);
    }
    pub fn run_install(self: Pin<&mut Self>, spt_path: &QString, local: bool, steam: bool) {
        work::run_install(self, spt_path.to_string(), local, steam);
    }
    pub fn run_cli(self: Pin<&mut Self>, name: &QString, args_json: &QString) {
        let args: Vec<String> = serde_json::from_str(&args_json.to_string()).unwrap_or_default();
        work::run_tasks(self, name.to_string(), vec![args]);
    }
    pub fn cancel_task(self: Pin<&mut Self>) {
        work::cancel_task(self);
    }
    pub fn open_official_launcher(self: Pin<&mut Self>) {
        work::open_official_launcher(self);
    }
    pub fn open_guided_installer(self: Pin<&mut Self>) {
        work::open_guided_installer(self);
    }
    pub fn steam_running(&self) -> bool {
        spt_core::steam::steam_running()
    }
    pub fn env_var(&self, name: &QString) -> QString {
        QString::from(std::env::var(name.to_string()).unwrap_or_default().as_str())
    }
}
