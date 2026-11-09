//! spt-launcher-qt: SPT launcher & installer (Rust + QML)

mod bridge;

use cxx_qt_lib::{QGuiApplication, QQmlApplicationEngine, QString, QUrl};

fn main() {
    // The UI draws its own controls; the Basic style keeps them unthemed by the desktop
    if std::env::var_os("QT_QUICK_CONTROLS_STYLE").is_none() {
        std::env::set_var("QT_QUICK_CONTROLS_STYLE", "Basic");
    }

    let mut app = bridge::app::new_application();
    if let Some(mut app) = app.as_mut() {
        app.as_mut().set_application_name(&QString::from("spt-launcher-qt"));
        app.as_mut().set_organization_name(&QString::from("spt-additions"));
    }
    // Wayland app id / X11 WM_CLASS must match the .desktop file (the Nix wrapper renames the binary)
    QGuiApplication::set_desktop_file_name(&QString::from("spt-launcher-qt"));
    bridge::app::set_window_icon(&QString::from(":/qt/qml/SptLauncher/icons/spt-launcher-qt.svg"));
    // Closing the window may only hide it to the tray; Main.qml quits explicitly
    bridge::app::set_quit_on_last_window_closed(false);

    let mut engine = QQmlApplicationEngine::new();
    if let Some(engine) = engine.as_mut() {
        engine.load(&QUrl::from("qrc:/qt/qml/SptLauncher/qml/Main.qml"));
    }

    if let Some(app) = app.as_mut() {
        app.exec();
    }
}
