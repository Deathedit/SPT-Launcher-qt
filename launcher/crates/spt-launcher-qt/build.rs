use cxx_qt_build::{CxxQtBuilder, QmlFile, QmlModule};

fn main() {
    let qml = [
        QmlFile::from("qml/Theme.qml").singleton(true),
        "qml/Main.qml".into(),
        "qml/Btn.qml".into(),
        "qml/Field.qml".into(),
        "qml/Toggle.qml".into(),
        "qml/Tag.qml".into(),
        "qml/Pill.qml".into(),
        "qml/Lbl.qml".into(),
        "qml/Panel.qml".into(),
        "qml/LogView.qml".into(),
        "qml/SettingRow.qml".into(),
        "qml/Icon.qml".into(),
        "qml/PlayPage.qml".into(),
        "qml/ServerPage.qml".into(),
        "qml/InstallPage.qml".into(),
        "qml/ProtonPage.qml".into(),
        "qml/SettingsPage.qml".into(),
    ];
    for f in &qml {
        println!("cargo:rerun-if-changed={}", f.get_path().display());
    }
    CxxQtBuilder::new_qml_module(QmlModule::new("SptLauncher").qml_files(qml))
        .qt_module("Network")
        .qt_module("Widgets")
        .qrc_resources(["icons/spt-launcher-qt.svg", "icons/spt-launcher-qt-tray.png"])
        .files(["src/bridge/launcher.rs", "src/bridge/app.rs"])
        .build();
}
