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

    readonly property var profiles: JSON.parse(app.profilesJson)
    readonly property var editions: JSON.parse(app.editionsJson)
    readonly property var mods: JSON.parse(app.modsJson)
    readonly property bool isLocal: app.target === "local"
    readonly property bool connected: app.connState === "ok"
    property string confirmId: ""
    property string confirmKind: ""
    property bool creating: false
    property string newEdition: "Standard"

    function money(n) { return String(n).replace(/\B(?=(\d{3})+(?!\d))/g, " ") }

    readonly property string title: !app.installed ? "Not installed"
        : app.gameState === "running" ? "In game"
        : app.gameState === "launching" ? "Deploying…"
        : connected ? "Ready to deploy" : "Connect to a server"

    ColumnLayout {
        id: col
        x: 24
        y: 22
        width: page.width - 48
        spacing: 18

        // Head
        RowLayout {
            Layout.fillWidth: true
            spacing: 16
            ColumnLayout {
                spacing: 2
                Lbl { text: "Single Player Tarkov" + (app.sptVersion ? " · " + app.sptVersion : "") }
                Label { text: page.title; color: Theme.text; font.family: Theme.display; font.pixelSize: 28; font.weight: Font.DemiBold; font.letterSpacing: 1 }
            }
            Item { Layout.fillWidth: true }
            Tag {
                visible: page.connected && app.serverVersion !== ""
                text: "Server " + app.serverVersion + (app.clientCheck === "match" ? " · compatible" : app.clientCheck === "mismatch" ? " · client mismatch" : "")
                tone: app.clientCheck === "mismatch" || app.clientCheck === "error" ? "err" : "ok"
            }
        }

        // Not installed
        Rectangle {
            visible: !app.installed
            Layout.fillWidth: true
            implicitHeight: 48
            color: "transparent"
            border.color: Theme.accent
            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: 14
                anchors.rightMargin: 8
                Lbl { text: app.legacy ? "SPT 4.0 (legacy) is installed · use the official launcher" : "SPT is not installed yet"; color: Theme.accent; Layout.fillWidth: true }
                Btn { text: "Open installer"; kind: "pri"; small: true; onClicked: page.win.view = "install" }
            }
        }

        // Connection bar
        Rectangle {
            Layout.fillWidth: true
            implicitHeight: connRow.implicitHeight + 24
            color: Theme.panel
            border.color: Theme.line
            RowLayout {
                id: connRow
                anchors.fill: parent
                anchors.margins: 12
                anchors.leftMargin: 14
                spacing: 14
                Row {
                    component Tab: AbstractButton {
                        id: tb
                        property bool on: false
                        implicitHeight: 34
                        leftPadding: 12
                        rightPadding: 12
                        hoverEnabled: true
                        opacity: enabled ? 1 : 0.35
                        background: Rectangle { color: tb.on ? Theme.text : "transparent" }
                        contentItem: Label {
                            text: tb.text
                            color: tb.on ? Theme.bg : (tb.hovered ? Theme.text : Theme.muted)
                            font.family: Theme.mono
                            font.pixelSize: 11
                            font.capitalization: Font.AllUppercase
                            font.letterSpacing: 1
                            verticalAlignment: Text.AlignVCenter
                        }
                    }
                    Rectangle {
                        width: tabs.implicitWidth + 2
                        height: 36
                        color: "transparent"
                        border.color: Theme.line2
                        Row {
                            id: tabs
                            x: 1
                            y: 1
                            Tab { text: "Local server"; on: page.isLocal; enabled: app.localEnabled && app.gameState === "idle"; onClicked: app.setConnectTarget("local") }
                            Tab { text: "Remote / Fika host"; on: !page.isLocal; enabled: app.gameState === "idle"; onClicked: app.setConnectTarget("remote") }
                        }
                    }
                }
                Rectangle {
                    visible: page.isLocal
                    implicitWidth: addrText.implicitWidth + 24
                    implicitHeight: 38
                    color: "transparent"
                    border.color: Theme.line2
                    Label { id: addrText; anchors.centerIn: parent; text: "127.0.0.1:6969"; color: Theme.text; font.family: Theme.mono; font.pixelSize: 13 }
                }
                Field {
                    id: hostField
                    visible: !page.isLocal
                    Layout.preferredWidth: 240
                    text: app.remoteHost
                    placeholderText: "host:6969"
                    enabled: app.gameState === "idle"
                    onEditingFinished: app.setRemote(text)
                    onAccepted: { app.setRemote(text); app.connectServer() }
                    Accessible.name: "Host address"
                }
                Pill {
                    text: ({ none: "Not connected", connecting: "Connecting", ok: "Connected", fail: "Unreachable" })[app.connState]
                    status: ({ none: "off", connecting: "busy", ok: "run", fail: "off" })[app.connState]
                }
                Item { Layout.fillWidth: true }
                Btn {
                    visible: app.installed && page.isLocal && app.serverState !== "running" && app.serverState !== "external"
                    text: app.serverState === "starting" ? "Starting…" : "Start server"
                    kind: "pri"
                    enabled: app.serverState === "off"
                    onClicked: app.startServer()
                }
                Btn {
                    visible: app.installed && (!page.isLocal || app.serverState === "running" || app.serverState === "external")
                    text: app.connState === "connecting" ? "Connecting…" : (page.connected ? "Reconnect" : "Connect")
                    enabled: app.connState !== "connecting" && app.gameState === "idle"
                    onClicked: { if (!page.isLocal) app.setRemote(hostField.text); app.connectServer() }
                }
            }
        }

        Rectangle {
            visible: app.connState === "fail"
            Layout.fillWidth: true
            implicitHeight: 48
            color: "transparent"
            border.color: Theme.err
            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: 14
                anchors.rightMargin: 8
                Lbl { text: "No answer from " + app.connHost + " · " + app.connError; color: Theme.err; Layout.fillWidth: true; font.capitalization: Font.MixedCase; font.letterSpacing: 0 }
                Btn { text: "Retry"; small: true; onClicked: app.connectServer() }
            }
        }

        // Profiles + launch
        RowLayout {
            visible: app.installed && page.connected
            Layout.fillWidth: true
            spacing: 18

            Panel {
                Layout.fillWidth: true
                Layout.alignment: Qt.AlignTop
                title: "Profiles"
                subtitle: page.profiles.length + (page.profiles.length === 1 ? " profile" : " profiles") + " on " + app.connHost
                header: [
                    Btn {
                        text: "New profile"
                        small: true
                        iconName: "plus"
                        enabled: !page.creating && !app.busy && app.gameState === "idle"
                        onClicked: { page.creating = true; newUser.text = ""; page.newEdition = page.editions.length ? (page.editions.some(e => e.name === "Standard") ? "Standard" : page.editions[0].name) : "Standard"; newUser.forceActiveFocus() }
                    }
                ]

                // New profile form
                ColumnLayout {
                    visible: page.creating
                    Layout.fillWidth: true
                    Layout.margins: 16
                    spacing: 12
                    Lbl { text: "Username" }
                    Field {
                        id: newUser
                        Layout.fillWidth: true
                        placeholderText: "Login name for this server"
                        onAccepted: createBtn.clicked()
                    }
                    readonly property bool taken: page.profiles.some(p => p.username.toLowerCase() === newUser.text.trim().toLowerCase())
                    Label { visible: parent.taken && newUser.text.trim() !== ""; text: "A profile with this username already exists"; color: Theme.err; font.family: Theme.mono; font.pixelSize: 12 }
                    Lbl { text: "Edition" }
                    GridLayout {
                        Layout.fillWidth: true
                        columns: Math.max(1, Math.floor(width / 240))
                        columnSpacing: 8
                        rowSpacing: 8
                        Repeater {
                            model: page.editions
                            AbstractButton {
                                id: ed
                                required property var modelData
                                readonly property bool on: page.newEdition === modelData.name
                                Layout.fillWidth: true
                                implicitHeight: edCol.implicitHeight + 20
                                hoverEnabled: true
                                onClicked: page.newEdition = modelData.name
                                Accessible.role: Accessible.RadioButton
                                Accessible.name: modelData.name
                                background: Rectangle {
                                    color: ed.on ? Theme.warnRow : "transparent"
                                    border.color: ed.on || ed.hovered ? Theme.accent : Theme.line2
                                    radius: Theme.radius
                                }
                                contentItem: RowLayout {
                                    spacing: 12
                                    Rectangle {
                                        Layout.alignment: Qt.AlignTop
                                        Layout.topMargin: 2
                                        Layout.leftMargin: 12
                                        width: 20; height: 20; radius: 10
                                        color: ed.on ? Theme.accent : "transparent"
                                        border.color: ed.on ? Theme.accent : Theme.line2
                                        Rectangle { visible: ed.on; anchors.centerIn: parent; width: 10; height: 10; radius: 5; color: Theme.accent; border.color: Theme.panel2; border.width: 0 }
                                    }
                                    ColumnLayout {
                                        id: edCol
                                        Layout.fillWidth: true
                                        Layout.rightMargin: 12
                                        spacing: 2
                                        Label { text: ed.modelData.name; color: Theme.text; font.family: Theme.sans; font.weight: Font.DemiBold; font.pixelSize: 14; Layout.fillWidth: true; elide: Text.ElideRight }
                                        Label { text: ed.modelData.desc; color: Theme.muted; font.family: Theme.sans; font.pixelSize: 12; Layout.fillWidth: true; wrapMode: Text.Wrap; maximumLineCount: 3; elide: Text.ElideRight }
                                    }
                                }
                            }
                        }
                    }
                    RowLayout {
                        Layout.fillWidth: true
                        Item { Layout.fillWidth: true }
                        Btn { text: "Cancel"; onClicked: page.creating = false }
                        Btn {
                            id: createBtn
                            text: app.busy ? "Creating…" : "Create profile"
                            kind: "pri"
                            enabled: newUser.text.trim() !== "" && !parent.parent.taken && !app.busy
                            onClicked: { app.createProfile(newUser.text.trim(), page.newEdition); page.creating = false }
                        }
                    }
                }
                Rectangle { visible: page.creating; Layout.fillWidth: true; height: 1; color: Theme.line }

                Label {
                    visible: page.profiles.length === 0 && !page.creating
                    Layout.margins: 16
                    Layout.fillWidth: true
                    text: "No profiles on this server yet. Create one to start playing."
                    color: Theme.muted
                    font.pixelSize: 13
                    wrapMode: Text.Wrap
                }

                Repeater {
                    model: page.profiles
                    Rectangle {
                        id: prow
                        required property var modelData
                        required property int index
                        readonly property bool on: app.selectedId === modelData.id
                        readonly property bool confirming: page.confirmId === modelData.id
                        Layout.fillWidth: true
                        implicitHeight: prowLayout.implicitHeight + 24
                        color: confirming ? Theme.warnRow : (on ? Theme.panel2 : "transparent")
                        Rectangle { visible: prow.index > 0; width: parent.width; height: 1; color: Theme.line }
                        MouseArea { anchors.fill: parent; enabled: !prow.modelData.invalid; onClicked: app.selectProfile(prow.modelData.id) }
                        RowLayout {
                            id: prowLayout
                            anchors.fill: parent
                            anchors.margins: 12
                            anchors.leftMargin: 14
                            anchors.rightMargin: 14
                            spacing: 16
                            AbstractButton {
                                implicitWidth: 20
                                implicitHeight: 20
                                enabled: !prow.modelData.invalid && app.gameState === "idle"
                                onClicked: app.selectProfile(prow.modelData.id)
                                Accessible.role: Accessible.RadioButton
                                Accessible.name: "Select " + prow.modelData.nick
                                background: Rectangle {
                                    radius: 10
                                    color: prow.on ? Theme.accent : "transparent"
                                    border.color: prow.on ? Theme.accent : Theme.line2
                                    Rectangle { visible: prow.on; anchors.centerIn: parent; width: 12; height: 12; radius: 6; color: "transparent"; border.color: Theme.panel2; border.width: 4 }
                                }
                            }
                            ColumnLayout {
                                Layout.fillWidth: true
                                spacing: 3
                                Flow {
                                    Layout.fillWidth: true
                                    spacing: 8
                                    Label { text: prow.modelData.nick; color: Theme.text; font.family: Theme.sans; font.weight: Font.DemiBold; font.pixelSize: 14 }
                                    Tag { text: prow.modelData.side; visible: prow.modelData.side !== "–" }
                                    Tag { text: prow.modelData.edition }
                                    Tag { visible: prow.modelData.fresh; text: "Fresh · create character in game"; tone: "acc" }
                                    Tag { visible: prow.modelData.invalid; text: "Can't be loaded"; tone: "err" }
                                }
                                Label {
                                    text: "user: " + prow.modelData.username + " · " + page.money(prow.modelData.roubles) + " ₽"
                                    color: Theme.dim
                                    font.family: Theme.mono
                                    font.pixelSize: 11
                                    Layout.fillWidth: true
                                    elide: Text.ElideRight
                                }
                            }
                            ColumnLayout {
                                Layout.preferredWidth: 120
                                spacing: 4
                                Label { text: prow.modelData.fresh ? "Level –" : "Level " + prow.modelData.level; color: Theme.muted; font.family: Theme.mono; font.pixelSize: 12 }
                                Rectangle {
                                    Layout.fillWidth: true
                                    height: 4
                                    color: Theme.panel2
                                    Rectangle { height: parent.height; width: parent.width * (prow.modelData.fresh ? 0 : prow.modelData.xpPct) / 100; color: Theme.accent }
                                }
                            }
                            Row {
                                visible: !prow.confirming
                                spacing: 6
                                Btn { text: "Wipe"; small: true; enabled: app.gameState === "idle" && !app.busy; onClicked: { page.confirmId = prow.modelData.id; page.confirmKind = "wipe" } }
                                Btn { text: "Remove"; small: true; kind: "bad"; enabled: app.gameState === "idle" && !app.busy; onClicked: { page.confirmId = prow.modelData.id; page.confirmKind = "remove" } }
                            }
                            Row {
                                visible: prow.confirming
                                spacing: 6
                                Lbl { text: page.confirmKind === "wipe" ? "Reset all progress?" : "Delete this profile?"; color: Theme.accent; anchors.verticalCenter: parent.verticalCenter }
                                Btn { text: "Keep"; small: true; onClicked: page.confirmId = "" }
                                Btn {
                                    text: page.confirmKind === "wipe" ? "Wipe" : "Remove"
                                    small: true
                                    kind: "bad"
                                    onClicked: {
                                        const id = prow.modelData.id
                                        const kind = page.confirmKind
                                        page.confirmId = ""
                                        if (kind === "wipe") app.wipeProfile(id); else app.removeProfile(id)
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Launch panel
            Panel {
                Layout.preferredWidth: 320
                Layout.alignment: Qt.AlignTop
                ColumnLayout {
                    Layout.fillWidth: true
                    Layout.margins: 16
                    spacing: 14
                    ColumnLayout {
                        spacing: 6
                        Lbl { text: "Deploy as" }
                        Label { text: page.win.selectedProfile ? page.win.selectedProfile.nick : "none"; color: Theme.text; font.family: Theme.display; font.pixelSize: 24; font.weight: Font.DemiBold; font.letterSpacing: 1; Layout.fillWidth: true; elide: Text.ElideRight }
                        Label {
                            readonly property var p: page.win.selectedProfile
                            text: p ? (p.side !== "–" ? p.side + " · " : "") + (p.fresh ? "fresh profile" : "level " + p.level) + " · " + p.edition : "Pick or create a profile"
                            color: Theme.dim
                            font.family: Theme.mono
                            font.pixelSize: 11
                            Layout.fillWidth: true
                            elide: Text.ElideRight
                        }
                    }
                    Btn {
                        Layout.fillWidth: true
                        big: true
                        kind: "pri"
                        iconName: "play"
                        text: app.gameState === "idle" ? "Launch" : (app.gameState === "launching" ? "Launching…" : "In game")
                        enabled: page.win.canLaunch
                        onClicked: app.launch()
                    }
                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 2
                        Repeater {
                            model: [
                                { t: "Check client", i: "spt-core.dll vs server" },
                                { t: "Prepare game files", i: "cleanup · patches" },
                                { t: "Mod bundles", i: "/singleplayer/bundles" },
                                { t: "Start game", i: "umu-run" }
                            ]
                            RowLayout {
                                id: ph
                                required property var modelData
                                required property int index
                                readonly property bool done: app.gameState === "running" || (app.gameState === "launching" && app.launchPhase > index)
                                readonly property bool active: app.gameState === "launching" && app.launchPhase === index
                                Layout.fillWidth: true
                                implicitHeight: 30
                                spacing: 12
                                Item {
                                    width: 16; height: 16
                                    Icon { anchors.fill: parent; visible: ph.done; name: "check"; color: Theme.ok; stroke: 1.8 }
                                    Icon {
                                        anchors.fill: parent
                                        visible: ph.active
                                        name: "spin"
                                        color: Theme.accent
                                        stroke: 1.8
                                        RotationAnimation on rotation { running: ph.active; from: 0; to: 360; duration: 900; loops: Animation.Infinite }
                                    }
                                    Icon { anchors.fill: parent; visible: !ph.done && !ph.active; name: "dot"; color: Theme.dim }
                                }
                                Label { text: ph.modelData.t; color: ph.active ? Theme.accent : (ph.done ? Theme.text : Theme.muted); font.pixelSize: 13; Layout.fillWidth: true }
                                Label {
                                    text: ph.active && app.phaseInfo !== "" ? app.phaseInfo : ph.modelData.i
                                    color: Theme.dim
                                    font.family: Theme.mono
                                    font.pixelSize: 11
                                    elide: Text.ElideLeft
                                    Layout.maximumWidth: 170
                                }
                            }
                        }
                    }
                    Rectangle {
                        visible: app.gameState === "launching" && app.launchPhase === 2
                        Layout.fillWidth: true
                        height: 8
                        color: Theme.panel2
                        border.color: Theme.line2
                        Rectangle { x: 1; y: 1; height: parent.height - 2; width: (parent.width - 2) * app.bundlePct / 100; color: Theme.accent; Behavior on width { NumberAnimation { duration: 200 } } }
                    }
                    Label {
                        visible: app.launchError !== "" && app.gameState === "idle"
                        text: app.launchError
                        color: Theme.err
                        font.family: Theme.mono
                        font.pixelSize: 12
                        wrapMode: Text.Wrap
                        Layout.fillWidth: true
                    }
                    Btn {
                        visible: app.gameState !== "idle"
                        Layout.fillWidth: true
                        small: true
                        kind: "bad"
                        text: app.gameState === "launching" ? "Cancel" : "Force stop"
                        onClicked: app.stopGame()
                    }
                }
                Rectangle { Layout.fillWidth: true; height: 1; color: Theme.line }
                Lbl {
                    Layout.margins: 12
                    Layout.leftMargin: 16
                    Layout.fillWidth: true
                    text: "umu-run · " + (app.protonPath ? app.protonPath.split("/").pop() : "no proton") + (app.gameMode ? " · gamemode" : "")
                }
            }
        }

        // Server mods
        Panel {
            visible: app.installed && page.connected
            Layout.fillWidth: true
            title: ""
            Rectangle {
                Layout.fillWidth: true
                implicitHeight: 44
                color: Theme.panel
                RowLayout {
                    anchors.fill: parent
                    anchors.leftMargin: 14
                    anchors.rightMargin: 14
                    Lbl { text: "Server mods · /launcher/v2/mod-pages"; Layout.fillWidth: true }
                    Lbl { text: page.mods.length + (page.mods.length === 1 ? " mod" : " mods") }
                }
            }
            Label {
                visible: page.mods.length === 0
                Layout.margins: 14
                text: "The server reports no mods. Nothing to download before launch."
                color: Theme.muted
                font.pixelSize: 13
            }
            Repeater {
                model: page.mods
                Rectangle {
                    required property var modelData
                    Layout.fillWidth: true
                    implicitHeight: modRow.implicitHeight + 24
                    color: "transparent"
                    Rectangle { width: parent.width; height: 1; color: Theme.line }
                    RowLayout {
                        id: modRow
                        anchors.fill: parent
                        anchors.margins: 12
                        anchors.leftMargin: 14
                        ColumnLayout {
                            Layout.fillWidth: true
                            spacing: 2
                            Label { text: modelData.name; color: Theme.text; font.weight: Font.DemiBold; font.pixelSize: 14 }
                            Label { text: modelData.desc; visible: text !== ""; color: Theme.dim; font.family: Theme.mono; font.pixelSize: 11; Layout.fillWidth: true; elide: Text.ElideRight }
                        }
                        Btn { visible: modelData.url !== ""; text: "Homepage"; small: true; iconName: "link"; onClicked: Qt.openUrlExternally(modelData.url) }
                    }
                }
            }
        }

        // Launcher log
        Panel {
            Layout.fillWidth: true
            Rectangle {
                Layout.fillWidth: true
                implicitHeight: 40
                color: Theme.logBg
                RowLayout {
                    anchors.fill: parent
                    anchors.leftMargin: 12
                    anchors.rightMargin: 8
                    Lbl { text: "launcher · " + page.log.count + " lines"; Layout.fillWidth: true }
                    Btn { text: "Clear"; small: true; onClicked: page.log.clear() }
                }
                Rectangle { anchors.bottom: parent.bottom; width: parent.width; height: 1; color: Theme.line }
            }
            LogView {
                Layout.fillWidth: true
                Layout.preferredHeight: 220
                model: page.log
            }
        }
    }
}
