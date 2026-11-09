import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import SptLauncher

Item {
    id: page
    required property var app
    required property ListModel log

    property string filter: "all"
    property double now: Date.now()
    readonly property bool up: app.serverState === "running" || app.serverState === "external"

    Timer { interval: 1000; repeat: true; running: page.up && page.visible; onTriggered: page.now = Date.now() }

    function uptime() {
        if (!up || app.serverStartedAt <= 0) return "--:--:--"
        const t = Math.max(0, Math.floor((now - app.serverStartedAt) / 1000))
        const p = n => (n < 10 ? "0" : "") + n
        return p(Math.floor(t / 3600)) + ":" + p(Math.floor(t % 3600 / 60)) + ":" + p(t % 60)
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 24
        anchors.topMargin: 22
        spacing: 18

        RowLayout {
            Layout.fillWidth: true
            spacing: 16
            ColumnLayout {
                spacing: 2
                Lbl { text: "SPT_Runtime / SPT.Server.Linux · Nix FHS env" }
                Label { text: "Server"; color: Theme.text; font.family: Theme.display; font.pixelSize: 28; font.weight: Font.DemiBold; font.letterSpacing: 1 }
            }
            Item { Layout.fillWidth: true }
            Row {
                visible: app.localEnabled
                spacing: 8
                Btn { text: "Start"; kind: "pri"; iconName: "play"; enabled: app.serverState === "off" && app.installed; onClicked: app.startServer() }
                Btn { text: "Restart"; iconName: "restart"; enabled: app.serverState === "running" || app.serverState === "starting"; onClicked: app.restartServer() }
                Btn { text: "Stop"; iconName: "stop"; enabled: app.serverState === "running" || app.serverState === "starting"; onClicked: app.stopServer() }
            }
        }

        // Client-only
        Rectangle {
            visible: !app.localEnabled
            Layout.fillWidth: true
            implicitHeight: emptyCol.implicitHeight + 72
            color: "transparent"
            border.color: Theme.line2
            ColumnLayout {
                id: emptyCol
                anchors.centerIn: parent
                width: Math.min(parent.width - 40, 460)
                spacing: 12
                Icon { name: "server"; color: Theme.dim; Layout.preferredWidth: 32; Layout.preferredHeight: 32; Layout.alignment: Qt.AlignHCenter; stroke: 1.4 }
                Label { text: "Client-only install"; color: Theme.text; font.weight: Font.DemiBold; font.pixelSize: 15; Layout.alignment: Qt.AlignHCenter }
                Label {
                    text: app.serverAvailable
                        ? "The local server is not set up. Play connects to a remote or Fika host instead. Enable it to host your own game."
                        : "This is the client-only package (no spt-server). Install spt-launcher-qt instead of spt-launcher-qt-client to host a local server."
                    color: Theme.muted
                    wrapMode: Text.Wrap
                    horizontalAlignment: Text.AlignHCenter
                    Layout.fillWidth: true
                }
                Btn { visible: app.serverAvailable; text: "Enable local server"; kind: "pri"; Layout.alignment: Qt.AlignHCenter; onClicked: app.enableLocal(true) }
            }
        }

        Item { visible: !app.localEnabled; Layout.fillHeight: true }

        // Stats
        GridLayout {
            visible: app.localEnabled
            Layout.fillWidth: true
            columns: 4
            columnSpacing: 1
            rowSpacing: 1
            component Stat: Rectangle {
                property string label
                property string value
                Layout.fillWidth: true
                implicitHeight: 66
                color: Theme.bg
                border.color: Theme.line
                ColumnLayout {
                    anchors.fill: parent
                    anchors.margins: 12
                    anchors.leftMargin: 14
                    spacing: 6
                    Lbl { text: parent.parent.label }
                    Label { text: parent.parent.value; color: Theme.text; font.family: Theme.mono; font.pixelSize: 18; font.weight: Font.Medium; elide: Text.ElideRight; Layout.fillWidth: true }
                }
            }
            Stat { label: "Status"; value: ({ off: "Offline", starting: "Starting", stopping: "Stopping", running: "Online", external: "Online (external)" })[app.serverState] }
            Stat { label: "Uptime"; value: page.uptime() }
            Stat { label: "Endpoint"; value: "127.0.0.1:6969" }
            Stat { label: "Profiles"; value: app.target === "local" && app.connState === "ok" ? JSON.parse(app.profilesJson).length : "–" }
        }

        // Console
        Rectangle {
            visible: app.localEnabled
            Layout.fillWidth: true
            Layout.fillHeight: true
            color: "transparent"
            border.color: Theme.line
            ColumnLayout {
                anchors.fill: parent
                anchors.margins: 1
                spacing: 0
                Rectangle {
                    Layout.fillWidth: true
                    implicitHeight: 44
                    color: Theme.logBg
                    RowLayout {
                        anchors.fill: parent
                        anchors.leftMargin: 12
                        anchors.rightMargin: 8
                        spacing: 6
                        Lbl { text: "spt-server · " + page.log.count + " lines"; Layout.fillWidth: true }
                        component Chip: AbstractButton {
                            id: chip
                            property bool on: false
                            implicitHeight: 30
                            leftPadding: 10
                            rightPadding: 10
                            hoverEnabled: true
                            background: Rectangle { radius: Theme.radius; color: chip.on ? Theme.accent : "transparent"; border.color: chip.on ? Theme.accent : Theme.line2 }
                            contentItem: Label {
                                text: chip.text
                                color: chip.on ? Theme.ink : (chip.hovered ? Theme.text : Theme.muted)
                                font.family: Theme.mono
                                font.pixelSize: 11
                                font.capitalization: Font.AllUppercase
                                font.letterSpacing: 1
                                verticalAlignment: Text.AlignVCenter
                            }
                        }
                        Repeater {
                            model: [["all", "All"], ["info", "Info"], ["warn", "Warn"], ["error", "Error"]]
                            Chip { required property var modelData; text: modelData[1]; on: page.filter === modelData[0]; onClicked: page.filter = modelData[0] }
                        }
                        Chip { text: "Follow"; on: consoleView.follow; onClicked: consoleView.follow = !consoleView.follow }
                        Chip { text: "Clear"; onClicked: page.log.clear() }
                    }
                }
                LogView {
                    id: consoleView
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    model: page.log
                    filter: ({ all: null, info: ["info", "ok", "cmd"], warn: ["warn"], error: ["error"] })[page.filter]
                }
                Rectangle { Layout.fillWidth: true; height: 1; color: Theme.line }
                RowLayout {
                    Layout.fillWidth: true
                    Layout.margins: 6
                    Layout.leftMargin: 14
                    spacing: 10
                    Label { text: "›"; color: Theme.accent; font.family: Theme.mono; font.pixelSize: 16 }
                    TextField {
                        id: stdin
                        Layout.fillWidth: true
                        placeholderText: app.serverState === "external" ? "Started outside the launcher: no console input" : "Send input to the server (stdin)"
                        placeholderTextColor: Theme.dim
                        color: Theme.text
                        font.family: Theme.mono
                        font.pixelSize: 13
                        background: Item { implicitWidth: 200 }
                        enabled: app.serverState === "running"
                        onAccepted: if (text !== "") { app.sendServerInput(text); text = "" }
                    }
                    Btn { text: "Send"; enabled: app.serverState === "running" && stdin.text !== ""; onClicked: { app.sendServerInput(stdin.text); stdin.text = "" } }
                }
            }
        }
    }
}
