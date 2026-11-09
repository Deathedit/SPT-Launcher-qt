import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import SptLauncher

Flickable {
    id: page
    required property var app
    required property var win
    required property ListModel log

    contentHeight: col.implicitHeight + 50
    clip: true
    boundsBehavior: Flickable.StopAtBounds
    ScrollBar.vertical: ScrollBar {}

    property bool confirmUninstall: false
    readonly property bool dirty: pathField.text !== app.sptPath || envField.text !== app.launchEnv
        || gmToggle.checked !== app.gameMode || cacheToggle.checked !== app.clearCache
    readonly property bool taskRunning: app.taskState === "running"

    function reset() {
        pathField.text = app.sptPath
        envField.text = app.launchEnv
        gmToggle.checked = app.gameMode
        cacheToggle.checked = app.clearCache
    }
    Connections {
        target: page.app
        function onSptPathChanged() { page.reset() }
        function onLaunchEnvChanged() { envField.text = page.app.launchEnv }
        function onGameModeChanged() { gmToggle.checked = page.app.gameMode }
        function onClearCacheChanged() { cacheToggle.checked = page.app.clearCache }
    }
    Component.onCompleted: reset()

    component Group: ColumnLayout {
        property string title
        property bool danger: false
        Layout.fillWidth: true
        Layout.leftMargin: 16
        Layout.rightMargin: 16
        Layout.topMargin: 14
        Layout.bottomMargin: 4
        spacing: 0
        Label { text: parent.title; color: parent.danger ? Theme.err : Theme.accent; font.family: Theme.mono; font.pixelSize: 12; Layout.bottomMargin: 4 }
    }
    component Divider: Rectangle { Layout.fillWidth: true; height: 1; color: Theme.line }

    ColumnLayout {
        id: col
        x: 24
        y: 22
        width: page.width - 48
        spacing: 18

        RowLayout {
            Layout.fillWidth: true
            spacing: 12
            ColumnLayout {
                spacing: 2
                Lbl { text: "app.conf · LauncherSettings.json" }
                Label { text: "Settings"; color: Theme.text; font.family: Theme.display; font.pixelSize: 28; font.weight: Font.DemiBold; font.letterSpacing: 1 }
            }
            Item { Layout.fillWidth: true }
            Btn { text: "Revert"; visible: page.dirty; onClicked: page.reset() }
            Btn { text: "Save"; kind: "pri"; enabled: page.dirty; onClicked: app.saveSettings(pathField.text, envField.text, gmToggle.checked, cacheToggle.checked) }
        }

        Panel {
            Layout.fillWidth: true

            Group {
                title: "Paths"
                SettingRow {
                    label: "SPT directory"
                    hint: "spt-path in ~/.config/spt-additions/app.conf"
                    Field { id: pathField; width: 340; Accessible.name: "SPT directory" }
                    Btn { text: "Open"; small: true; iconName: "folder"; onClicked: Qt.openUrlExternally("file://" + app.sptPath) }
                }
                SettingRow {
                    label: "Wine prefix"
                    hint: app.prefixPath.indexOf("compatdata") >= 0 ? "Steam manages this prefix" : "WINEPREFIX"
                    last: true
                    Field { width: 340; text: app.prefixPath; readOnly: true; Accessible.name: "Wine prefix" }
                }
            }
            Divider {}
            Group {
                title: "Launch"
                SettingRow {
                    label: "Launch options"
                    hint: "Steam style: NAME=VALUE sets env, -arg is passed to the game"
                    Field { id: envField; width: 340; placeholderText: "PROTON_USE_XALIA=0 MANGOHUD=1"; Accessible.name: "Launch options" }
                }
                SettingRow {
                    label: "Run with gamemoderun"
                    hint: "programs.gamemode.enable = true on NixOS"
                    Toggle { id: gmToggle; Accessible.name: "Run with gamemoderun" }
                }
                SettingRow {
                    label: "Clear game cache on launch"
                    hint: "Deletes SPT_Runtime/user/sptappdata before each start"
                    last: true
                    Toggle { id: cacheToggle; Accessible.name: "Clear game cache on launch" }
                }
            }
            Divider {}
            Group {
                title: "Integration"
                SettingRow {
                    label: "Local server"
                    hint: app.serverAvailable ? "Off = client-only (server=\"0\" in app.conf)" : "Not available in the client-only package"
                    Toggle { checked: app.localEnabled; enabled: app.serverAvailable; onToggled: app.enableLocal(checked); Accessible.name: "Local server" }
                }
                SettingRow {
                    label: "System tray icon"
                    hint: page.win.trayAvailable ? "Closing the window keeps the launcher running in the tray" : "No tray found (needs a StatusNotifierItem host)"
                    Toggle { checked: app.trayEnabled; onToggled: app.setTrayOptions(checked, app.hideOnPlay); Accessible.name: "System tray icon" }
                }
                SettingRow {
                    label: "Hide to tray while playing"
                    hint: "Hides when the game starts, comes back when it exits"
                    Toggle { checked: app.hideOnPlay; enabled: app.trayEnabled; onToggled: app.setTrayOptions(app.trayEnabled, checked); Accessible.name: "Hide to tray while playing" }
                }
                SettingRow {
                    label: "Steam non-Steam game entry"
                    hint: "SPT - Launcher" + (app.localEnabled ? " + Server" : "") + " · Steam must be closed"
                    Btn { text: "Add"; small: true; enabled: !page.taskRunning && app.installed; onClicked: app.steamRunning() ? app.toast("warn", "Close Steam first (Steam → Exit)") : app.runCli("Add to Steam", JSON.stringify(["steam"])) }
                    Btn { text: "Remove"; small: true; enabled: !page.taskRunning && app.installed; onClicked: app.steamRunning() ? app.toast("warn", "Close Steam first (Steam → Exit)") : app.runCli("Remove from Steam", JSON.stringify(["steam", "remove"])) }
                }
                SettingRow {
                    label: "Desktop shortcuts"
                    hint: "Application menu entries"
                    last: true
                    Btn { text: "Create"; small: true; enabled: !page.taskRunning && app.installed; onClicked: app.runCli("Shortcuts", JSON.stringify(["shortcut", "spt"])) }
                }
            }
            Divider {}
            Group {
                title: "Mods"
                SettingRow {
                    label: "Fika"
                    hint: app.fikaInstalled ? "Installed" : (app.localEnabled ? "Client plugin + server mod" : "Client plugin only (client-only mode)")
                    last: true
                    Btn {
                        text: app.taskState === "running" && app.taskName === "Install Fika" ? "Installing…" : (app.fikaInstalled ? "Update Fika" : "Install Fika")
                        small: true
                        enabled: !page.taskRunning && app.installed
                        onClicked: app.runCli("Install Fika", JSON.stringify(app.fikaInstalled ? ["update", "fika"] : ["install", "fika"]))
                    }
                }
            }
            Divider {}
            Group {
                title: "Fallback"
                SettingRow {
                    label: "Official SPT launcher"
                    hint: "SPT.Launcher.Linux · only if Play breaks after an SPT update"
                    last: true
                    Btn { text: "Open"; small: true; enabled: app.installed; onClicked: app.openOfficialLauncher() }
                }
            }
            Divider {}
            Group {
                title: "Danger zone"
                danger: true
                SettingRow {
                    label: "Uninstall SPT"
                    hint: "Deletes the SPT directory, shortcuts & Steam entries"
                    last: true
                    Btn { visible: !page.confirmUninstall; text: "Uninstall"; small: true; kind: "bad"; enabled: app.installed && !page.taskRunning; onClicked: page.confirmUninstall = true }
                    Btn { visible: page.confirmUninstall; text: "Keep"; small: true; onClicked: page.confirmUninstall = false }
                    Btn {
                        visible: page.confirmUninstall
                        text: "Delete SPT"
                        small: true
                        kind: "bad"
                        onClicked: {
                            page.confirmUninstall = false
                            app.stopServer()
                            app.runCli("Uninstall SPT", JSON.stringify(["uninstall", "spt"]))
                            page.win.view = "install"
                        }
                    }
                }
            }
        }

        Panel {
            visible: page.log.count > 0 && app.taskName !== "Install SPT"
            Layout.fillWidth: true
            Rectangle {
                Layout.fillWidth: true
                implicitHeight: 40
                color: Theme.logBg
                Lbl { anchors.verticalCenter: parent.verticalCenter; x: 12; text: (app.taskName || "spt-additions") + " · " + app.taskState }
            }
            LogView { Layout.fillWidth: true; Layout.preferredHeight: 200; model: page.log }
        }
    }
}
