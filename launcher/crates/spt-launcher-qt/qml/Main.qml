import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import QtQuick.Window
import Qt.labs.platform as Platform
import SptLauncher

ApplicationWindow {
    id: win
    width: 1280
    height: 820
    minimumWidth: 900
    minimumHeight: 620
    visible: true
    title: "SPT Launcher"
    color: Theme.bg
    font.family: Theme.sans
    font.pixelSize: 14
    palette.windowText: Theme.text
    palette.text: Theme.text
    palette.base: Theme.bg
    palette.window: Theme.bg
    background: Rectangle { color: Theme.bg }

    property string view: "play"
    // Developer aids (see README): echo logs to stderr, launch right after connecting
    property bool echoLog: false
    property bool autoLaunch: false

    Launcher { id: app }

    // Fonts shipped with the package (SPT_LAUNCHER_FONTS)
    Repeater {
        model: ["ChakraPetch-Medium.ttf", "ChakraPetch-SemiBold.ttf", "IBMPlexSans-Regular.otf", "IBMPlexSans-Medium.otf",
                "IBMPlexSans-SemiBold.otf", "JetBrainsMono-Regular.ttf", "JetBrainsMono-Medium.ttf"]
        Item {
            id: fontItem
            required property string modelData
            FontLoader { source: app.fontsDir !== "" ? "file://" + app.fontsDir + "/" + fontItem.modelData : "" }
        }
    }

    // Logs (filled from Rust via logLine)
    ListModel { id: serverLog }
    ListModel { id: gameLog }
    ListModel { id: taskLog }
    ListModel { id: modLog }

    function stamp() {
        return Qt.formatTime(new Date(), "hh:mm:ss")
    }
    function appendLog(model, level, text) {
        model.append({ time: stamp(), level: level, text: text })
        if (model.count > 2000)
            model.remove(0, model.count - 2000)
    }

    Connections {
        target: app
        function onLogLine(channel, level, text) {
            win.appendLog(channel === "server" ? serverLog : (channel === "task" ? taskLog : (channel === "mods" ? modLog : gameLog)), level, text)
            if (win.echoLog)
                console.log("[" + channel + "/" + level + "] " + text)
        }
        function onConnStateChanged() {
            if (win.autoLaunch && app.connState === "ok") {
                win.autoLaunch = false
                Qt.callLater(app.launch)
            }
        }
        function onToast(kind, text) {
            toast.show(kind, text)
        }
        function onGameStateChanged() {
            if (app.gameState === "running" && app.hideOnPlay && win.trayActive && win.visible) {
                win.hiddenForGame = true
                win.hide()
            } else if (app.gameState === "idle" && win.hiddenForGame) {
                win.hiddenForGame = false
                win.showWindow()
            }
        }
    }

    // ── System tray ───────────────────────────────────────────────
    // StatusNotifierItem over D-Bus (KDE, GNOME + AppIndicator, waybar, noctalia, ...)
    readonly property bool trayAvailable: tray.available
    readonly property bool trayActive: tray.available && app.trayEnabled
    property bool hiddenForGame: false
    property bool trayHintShown: false

    function showWindow() {
        win.show()
        win.raise()
        win.requestActivate()
    }
    function toggleWindow() {
        if (win.visible) {
            win.hiddenForGame = false
            win.hide()
        } else {
            win.showWindow()
        }
    }

    onClosing: close => {
        if (win.trayActive) {
            close.accepted = false
            win.hide()
            if (!win.trayHintShown && tray.supportsMessages) {
                win.trayHintShown = true
                tray.showMessage("SPT Launcher", "Still running in the tray. Quit from the tray menu.", Platform.SystemTrayIcon.Information, 4000)
            }
        } else {
            Qt.quit()
        }
    }

    Platform.SystemTrayIcon {
        id: tray
        visible: app.trayEnabled
        // A fixed-size PNG: Qt sends SVG tray icons over D-Bus as 16/22 px bitmaps only (blurry)
        icon.source: "qrc:/qt/qml/SptLauncher/icons/spt-launcher-qt-tray.png"
        tooltip: "SPT Launcher · " + ({ idle: "Game idle", launching: "Launching", running: "In game" })[app.gameState]
        onActivated: reason => {
            if (reason === Platform.SystemTrayIcon.Trigger)
                win.toggleWindow()
        }
        menu: Platform.Menu {
            Platform.MenuItem {
                text: win.visible ? "Hide launcher" : "Show launcher"
                onTriggered: win.toggleWindow()
            }
            Platform.MenuSeparator {}
            Platform.MenuItem {
                text: win.selectedProfile ? "Play as " + win.selectedProfile.nick : "Play"
                enabled: win.canLaunch
                onTriggered: {
                    win.view = "play"
                    app.launch()
                }
            }
            Platform.MenuItem {
                text: "Stop game"
                enabled: app.gameState !== "idle"
                onTriggered: app.stopGame()
            }
            Platform.MenuSeparator {}
            Platform.MenuItem {
                text: "Quit"
                onTriggered: Qt.quit()
            }
        }
    }

    Component.onCompleted: {
        appendLog(gameLog, "cmd", "Ready. Connect to a server, pick a profile, launch.")
        if (!app.installed)
            view = "install"
        const start = app.envVar("SPT_LAUNCHER_VIEW")
        if (start !== "")
            view = start
        echoLog = app.envVar("SPT_LAUNCHER_ECHO_LOG") === "1"
        autoLaunch = app.envVar("SPT_LAUNCHER_AUTOLAUNCH") === "1"
        const delay = parseInt(app.envVar("SPT_LAUNCHER_SCREENSHOT_DELAY"))
        if (delay > 0)
            shotTimer.interval = delay
        const shot = app.envVar("SPT_LAUNCHER_SCREENSHOT")
        if (shot !== "")
            shotTimer.path = shot
    }

    // Developer aid: SPT_LAUNCHER_SCREENSHOT=/tmp/x.png saves a screenshot and quits
    Timer {
        id: shotTimer
        property string path: ""
        interval: 2500
        running: path !== ""
        onTriggered: rootLayout.grabToImage(function (r) {
            r.saveToFile(shotTimer.path)
            Qt.quit()
        })
    }

    readonly property var selectedProfile: {
        const list = JSON.parse(app.profilesJson)
        for (let i = 0; i < list.length; i++)
            if (list[i].id === app.selectedId)
                return list[i]
        return null
    }
    readonly property bool canLaunch: app.installed && app.connState === "ok" && app.gameState === "idle" && selectedProfile !== null && !selectedProfile.invalid

    Rectangle {
        id: rootLayout
        anchors.fill: parent
        color: Theme.bg

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        // ── Top bar ────────────────────────────────────────────────
        Rectangle {
            Layout.fillWidth: true
            implicitHeight: 54
            color: Theme.bg
            Rectangle { anchors.bottom: parent.bottom; width: parent.width; height: 1; color: Theme.line }
            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: 20
                anchors.rightMargin: 20
                spacing: 18
                Row {
                    spacing: 10
                    Icon { name: "brand"; color: Theme.accent; width: 22; height: 22; anchors.verticalCenter: parent.verticalCenter }
                    Label {
                        text: "SPT"
                        color: Theme.text
                        font.family: Theme.display
                        font.pixelSize: 17
                        font.weight: Font.DemiBold
                        font.letterSpacing: 3
                        anchors.verticalCenter: parent.verticalCenter
                    }
                    Label { text: "/ launcher"; color: Theme.dim; font.family: Theme.mono; font.pixelSize: 11; anchors.verticalCenter: parent.verticalCenter }
                }
                Pill {
                    text: !app.localEnabled ? "Client-only" : ({ off: "Server offline", starting: "Starting", stopping: "Stopping", running: "Server online", external: "Server online" })[app.serverState]
                    status: !app.localEnabled ? "na" : ({ off: "off", starting: "busy", stopping: "busy", running: "run", external: "run" })[app.serverState]
                }
                Pill {
                    text: ({ idle: "Game idle", launching: "Launching", running: "In game" })[app.gameState]
                    status: ({ idle: "", launching: "busy", running: "run" })[app.gameState]
                }
                Item { Layout.fillWidth: true }
                Lbl {
                    text: "Profile  <font color='" + Theme.text + "'>" + (win.selectedProfile ? win.selectedProfile.nick : "none") + "</font>"
                    textFormat: Text.StyledText
                }
                Btn {
                    text: "Play"
                    kind: "pri"
                    iconName: "play"
                    enabled: win.canLaunch
                    onClicked: { win.view = "play"; app.launch() }
                }
            }
        }

        RowLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            spacing: 0

            // ── Sidebar ────────────────────────────────────────────
            Rectangle {
                Layout.preferredWidth: 236
                Layout.fillHeight: true
                color: Theme.bg
                Rectangle { anchors.right: parent.right; width: 1; height: parent.height; color: Theme.line }

                ColumnLayout {
                    anchors.fill: parent
                    anchors.margins: 12
                    anchors.topMargin: 18
                    spacing: 22

                    component NavItem: AbstractButton {
                        id: nav
                        property string idx
                        property string page
                        property string count: ""
                        readonly property bool on: win.view === page
                        Layout.fillWidth: true
                        implicitHeight: 44
                        hoverEnabled: true
                        onClicked: win.view = page
                        Accessible.name: text
                        background: Rectangle {
                            radius: Theme.radius
                            color: nav.on ? Theme.panel2 : (nav.hovered ? Theme.panel : "transparent")
                            border.width: 1
                            border.color: nav.on ? Theme.line2 : (nav.visualFocus ? Theme.accent : "transparent")
                        }
                        contentItem: RowLayout {
                            spacing: 12
                            Label { text: nav.idx; color: nav.on ? Theme.accent : Theme.dim; font.family: Theme.mono; font.pixelSize: 11; Layout.preferredWidth: 18; Layout.leftMargin: 12 }
                            Label { text: nav.text; color: nav.on || nav.hovered ? Theme.text : Theme.muted; font.family: Theme.sans; font.pixelSize: 14; font.weight: Font.Medium; Layout.fillWidth: true }
                            Label { text: nav.count; color: Theme.dim; font.family: Theme.mono; font.pixelSize: 11; Layout.rightMargin: 12 }
                        }
                    }

                    ColumnLayout {
                        spacing: 2
                        Lbl { text: "Game"; Layout.leftMargin: 12; Layout.bottomMargin: 8 }
                        NavItem { idx: "01"; text: "Play"; page: "play"; count: app.connState === "ok" ? JSON.parse(app.profilesJson).length : "" }
                        NavItem { idx: "02"; text: "Server"; page: "server"; count: !app.localEnabled ? "–" : (app.serverState === "running" || app.serverState === "external" ? "on" : "off") }
                        NavItem { idx: "03"; text: "Mods"; page: "mods"; count: app.modTask !== "" ? "…" : (JSON.parse(app.installedMods).length || "") }
                    }
                    ColumnLayout {
                        spacing: 2
                        Lbl { text: "Setup"; Layout.leftMargin: 12; Layout.bottomMargin: 8 }
                        NavItem { idx: "04"; text: "Install"; page: "install"; count: app.taskState === "running" ? "…" : "" }
                        NavItem { idx: "05"; text: "Proton"; page: "proton"; count: JSON.parse(app.protonsJson).length }
                        NavItem { idx: "06"; text: "Settings"; page: "settings" }
                    }
                    Item { Layout.fillHeight: true }

                    // Install summary
                    Rectangle { Layout.fillWidth: true; height: 1; color: Theme.line }
                    GridLayout {
                        Layout.fillWidth: true
                        Layout.leftMargin: 12
                        Layout.rightMargin: 12
                        Layout.bottomMargin: 6
                        columns: 2
                        rowSpacing: 8
                        columnSpacing: 12
                        component Kv: Label {
                            font.family: Theme.mono
                            font.pixelSize: 12
                            elide: Text.ElideLeft
                            Layout.fillWidth: true
                            horizontalAlignment: Text.AlignRight
                        }
                        component Kk: Label { color: Theme.dim; font.family: Theme.mono; font.pixelSize: 12 }
                        Kk { text: "SPT" }
                        Kv { text: app.installed ? (app.sptVersion || "installed") : "not installed"; color: app.installed ? Theme.text : Theme.accent }
                        Kk { text: "Proton" }
                        Kv { text: app.protonPath ? app.protonPath.split("/").pop() : "–"; color: Theme.text }
                        Kk { text: "Prefix" }
                        Kv { text: app.prefixPath.indexOf("compatdata/3932890") >= 0 ? "Steam · 3932890" : (app.prefixPath || "–"); color: Theme.text }
                        Kk { text: "Mode" }
                        Kv { text: app.localEnabled ? "Local server" : "Client-only"; color: app.localEnabled ? Theme.ok : Theme.accent }
                        Kk { text: "Runtime" }
                        Kv { text: "Nix · umu-run"; color: Theme.text }
                    }
                }
            }

            // ── Pages ──────────────────────────────────────────────
            StackLayout {
                Layout.fillWidth: true
                Layout.fillHeight: true
                currentIndex: ["play", "server", "mods", "install", "proton", "settings"].indexOf(win.view)
                PlayPage { app: app; win: win; log: gameLog }
                ServerPage { app: app; log: serverLog }
                ModsPage { app: app; win: win; log: modLog }
                InstallPage { app: app; win: win; log: taskLog }
                ProtonPage { app: app; win: win; log: taskLog }
                SettingsPage { app: app; win: win; log: taskLog }
            }
        }
    }

    }

    // ── Toast ──────────────────────────────────────────────────────
    Rectangle {
        id: toast
        property string kind: "ok"
        property string text: ""
        function show(k, t) { kind = k; text = t; opacity = 1; hideTimer.restart() }
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        anchors.margins: 20
        width: Math.min(460, toastText.implicitWidth + 32)
        height: toastText.implicitHeight + 22
        opacity: 0
        visible: opacity > 0
        color: Theme.panel2
        border.width: 1
        border.color: kind === "error" ? Theme.err : (kind === "warn" ? Theme.accent : Theme.ok)
        Behavior on opacity { NumberAnimation { duration: 180 } }
        Label {
            id: toastText
            anchors.centerIn: parent
            width: Math.min(implicitWidth, 428)
            text: toast.text
            wrapMode: Text.Wrap
            color: Theme.text
            font.family: Theme.sans
            font.pixelSize: 13
        }
        Timer { id: hideTimer; interval: 4500; onTriggered: toast.opacity = 0 }
        MouseArea { anchors.fill: parent; onClicked: toast.opacity = 0 }
    }
}
