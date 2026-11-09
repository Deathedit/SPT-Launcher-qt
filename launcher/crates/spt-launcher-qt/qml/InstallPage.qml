import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import SptLauncher

// Drives the spt-additions CLI and shows its output
Flickable {
    id: page
    required property var app
    required property var win
    required property ListModel log

    contentHeight: col.implicitHeight + 50
    clip: true
    boundsBehavior: Flickable.StopAtBounds
    ScrollBar.vertical: ScrollBar {}

    readonly property bool running: app.taskState === "running"
    readonly property bool steamFound: app.steamGameDir !== ""
    property bool optLocal: app.localEnabled || (!app.installed && app.serverAvailable)
    property bool optSteam: true

    // Step detection from the installer's output (best effort)
    readonly property var steps: [
        { t: "Wine prefix", d: "Steam compatdata · workarounds", k: ["prefix path", "workaround"] },
        { t: "GE-Proton", d: "~/.steam/steam/compatibilitytools.d", k: ["proton"] },
        { t: "Dependencies", d: "winetricks · .NET desktop runtime", k: ["installing:", "winetricks", "dll override", "dotnet"] },
        { t: "Copy & downpatch", d: "Live files → SPT directory · hpatchz", k: ["copying", "patching", "patcher"] },
        { t: "SPT", d: "Download & extract the SPT release", k: ["spt.7z", "extracting", "downloading"] },
        { t: "Finalize", d: "Launch scripts · shortcuts · Steam", k: ["windows version", "launcher.sh", "shortcut", "steam user", "has been installed"] }
    ]
    property int stepIdx: -1
    Connections {
        target: page.log
        function onCountChanged() {
            if (!page.running || page.log.count === 0 || page.app.taskName !== "Install SPT") return
            const line = page.log.get(page.log.count - 1).text.toLowerCase()
            for (let i = page.steps.length - 1; i > page.stepIdx; i--) {
                if (page.steps[i].k.some(k => line.indexOf(k) >= 0)) { page.stepIdx = i; break }
            }
        }
    }

    ColumnLayout {
        id: col
        x: 24
        y: 22
        width: page.width - 48
        spacing: 18

        RowLayout {
            Layout.fillWidth: true
            ColumnLayout {
                spacing: 2
                Lbl { text: "spt-additions · installer" }
                Label { text: "Install SPT"; color: Theme.text; font.family: Theme.display; font.pixelSize: 28; font.weight: Font.DemiBold; font.letterSpacing: 1 }
            }
            Item { Layout.fillWidth: true }
            Tag {
                text: page.running ? "Running · " + app.taskName : (app.installed ? "Installed" : "Not installed")
                tone: page.running ? "acc" : (app.installed ? "ok" : "")
            }
        }

        RowLayout {
            Layout.fillWidth: true
            spacing: 18

            // Steps
            Rectangle {
                Layout.preferredWidth: 300
                Layout.alignment: Qt.AlignTop
                implicitHeight: stepCol.implicitHeight + 2
                color: "transparent"
                border.color: Theme.line
                ColumnLayout {
                    id: stepCol
                    anchors.fill: parent
                    anchors.margins: 1
                    spacing: 0
                    Repeater {
                        model: page.steps
                        Rectangle {
                            id: st
                            required property var modelData
                            required property int index
                            readonly property bool done: app.installed && !page.running || (page.running && page.stepIdx > index)
                            readonly property bool active: page.running && page.stepIdx === index
                            Layout.fillWidth: true
                            implicitHeight: 58
                            color: active ? Theme.panel2 : "transparent"
                            Rectangle { visible: st.index > 0; width: parent.width; height: 1; color: Theme.line }
                            RowLayout {
                                anchors.fill: parent
                                anchors.leftMargin: 14
                                anchors.rightMargin: 14
                                spacing: 12
                                Label { text: "0" + (st.index + 1); color: st.done ? Theme.ok : (st.active ? Theme.accent : Theme.dim); font.family: Theme.mono; font.pixelSize: 11; Layout.preferredWidth: 22 }
                                ColumnLayout {
                                    Layout.fillWidth: true
                                    spacing: 2
                                    Label { text: st.modelData.t; color: st.active ? Theme.accent : Theme.text; font.weight: Font.DemiBold; font.pixelSize: 14 }
                                    Label { text: st.modelData.d; color: Theme.dim; font.family: Theme.mono; font.pixelSize: 11; Layout.fillWidth: true; elide: Text.ElideRight }
                                }
                                Icon { visible: st.done; name: "check"; color: Theme.ok; stroke: 1.8; width: 18; height: 18 }
                            }
                        }
                    }
                }
            }

            // Options / progress
            Panel {
                Layout.fillWidth: true
                Layout.alignment: Qt.AlignTop
                title: app.installed && !page.running ? "SPT " + app.sptVersion + " is installed" : (page.running ? app.taskName : "Before you start")
                header: [
                    Tag { visible: !page.running && !app.installed; text: page.steamFound ? "Steam copy found" : "No Steam copy"; tone: page.steamFound ? "ok" : "acc" },
                    Tag { visible: !page.running && app.installed; text: "Ready"; tone: "ok" }
                ]

                ColumnLayout {
                    visible: !page.running && !app.installed
                    Layout.fillWidth: true
                    Layout.margins: 16
                    Layout.topMargin: 6
                    spacing: 0
                    SettingRow {
                        label: "SPT directory"
                        hint: "Live files are copied & downpatched here"
                        Field { id: pathField; width: 320; text: app.sptPath; Accessible.name: "SPT directory" }
                    }
                    SettingRow {
                        label: "Game (Steam)"
                        hint: page.steamFound ? app.steamGameDir : "Install Escape from Tarkov in Steam and run it once"
                        Tag { text: page.steamFound ? "Found" : "Missing"; tone: page.steamFound ? "ok" : "err" }
                    }
                    SettingRow {
                        label: "Set up the local server"
                        hint: app.serverAvailable ? "Off = client-only, join a remote / Fika host" : "Client-only package: no spt-server"
                        Toggle { checked: page.optLocal; enabled: app.serverAvailable; onToggled: page.optLocal = checked; Accessible.name: "Set up the local server" }
                    }
                    SettingRow {
                        label: "Add to Steam"
                        hint: "Non-Steam game entry · Steam must be closed"
                        last: true
                        Toggle { checked: page.optSteam; onToggled: page.optSteam = checked; Accessible.name: "Add to Steam" }
                    }
                    Label {
                        Layout.fillWidth: true
                        Layout.topMargin: 10
                        text: page.steamFound
                            ? "Needs the size of your game files plus 10 GB free. Close the game first. The installer resumes where it stopped if interrupted."
                            : "Only the Steam version can be installed from here. For the BSG launcher version, use the guided installer in a terminal."
                        color: page.steamFound ? Theme.muted : Theme.accent
                        wrapMode: Text.Wrap
                        font.pixelSize: 13
                        padding: 10
                        background: Rectangle { color: "transparent"; border.color: page.steamFound ? Theme.line2 : Theme.accent }
                    }
                }

                ColumnLayout {
                    visible: !page.running && app.installed
                    Layout.fillWidth: true
                    Layout.margins: 16
                    spacing: 9
                    component Dkv: RowLayout {
                        property string k
                        property string v
                        Layout.fillWidth: true
                        Lbl { text: parent.k; Layout.preferredWidth: 150 }
                        Label { text: parent.v; color: Theme.text; font.family: Theme.mono; font.pixelSize: 12; Layout.fillWidth: true; horizontalAlignment: Text.AlignRight; elide: Text.ElideLeft }
                    }
                    Dkv { k: "Directory"; v: app.sptPath }
                    Dkv { k: "Mode"; v: app.localEnabled ? "Local server" : "Client-only" }
                    Dkv { k: "Proton"; v: app.protonPath ? app.protonPath.split("/").pop() : "–" }
                    Dkv { k: "Prefix"; v: app.prefixPath || "–" }
                    Dkv { k: "Fika"; v: app.fikaInstalled ? "installed" : "not installed" }
                }

                ColumnLayout {
                    visible: page.running
                    Layout.fillWidth: true
                    Layout.margins: 16
                    spacing: 10
                    Lbl { text: page.stepIdx >= 0 ? "Step " + (page.stepIdx + 1) + " / " + page.steps.length + " · " + page.steps[page.stepIdx].t : "Starting…"; color: Theme.accent }
                    ProgressBar {
                        Layout.fillWidth: true
                        indeterminate: true
                        background: Rectangle { implicitHeight: 8; color: Theme.panel2; border.color: Theme.line2 }
                        contentItem: Item {
                            implicitHeight: 8
                            clip: true
                            Rectangle {
                                width: parent.width * 0.25
                                height: parent.height
                                color: Theme.accent
                                NumberAnimation on x { from: -parent.width * 0.25; to: parent.width; duration: 1400; loops: Animation.Infinite; running: page.running }
                            }
                        }
                    }
                    Label {
                        text: page.log.count ? page.log.get(page.log.count - 1).text : ""
                        color: Theme.dim
                        font.family: Theme.mono
                        font.pixelSize: 11
                        elide: Text.ElideMiddle
                        Layout.fillWidth: true
                    }
                }

                Rectangle { Layout.fillWidth: true; height: 1; color: Theme.line }
                RowLayout {
                    Layout.fillWidth: true
                    Layout.margins: 12
                    Layout.leftMargin: 16
                    spacing: 10
                    Lbl { text: page.running ? "~/.cache/spt-additions/installer.log" : "spt-additions CLI"; Layout.fillWidth: true }
                    Btn { visible: !page.running; text: "Guided installer (terminal)"; small: true; onClicked: app.openGuidedInstaller() }
                    Btn { visible: !page.running && app.installed; text: "Refresh"; small: true; onClicked: app.refresh() }
                    Btn { visible: !page.running && app.installed; text: "Go to Play"; kind: "pri"; onClicked: page.win.view = "play" }
                    Btn {
                        visible: !page.running && !app.installed
                        text: "Start install"
                        kind: "pri"
                        enabled: page.steamFound
                        onClicked: { page.stepIdx = -1; page.log.clear(); app.runInstall(pathField.text, page.optLocal, page.optSteam) }
                    }
                    Btn { visible: page.running; text: "Cancel"; kind: "bad"; onClicked: app.cancelTask() }
                }
            }
        }

        // Maintenance
        Panel {
            visible: app.installed
            Layout.fillWidth: true
            title: "Maintenance"
            subtitle: "spt-additions"
            ColumnLayout {
                Layout.fillWidth: true
                Layout.margins: 16
                Layout.topMargin: 4
                spacing: 0
                SettingRow {
                    label: "Update SPT"
                    hint: "spt-additions update spt · patch releases only"
                    Btn { text: "Check & update"; small: true; enabled: !page.running; onClicked: app.runCli("Update SPT", JSON.stringify(["update", "spt"])) }
                }
                SettingRow {
                    label: "Re-apply downpatch"
                    hint: "spt-additions patch · after a live game update"
                    Btn { text: "Patch"; small: true; enabled: !page.running; onClicked: app.runCli("Patch", JSON.stringify(["patch"])) }
                }
                SettingRow {
                    label: "Desktop shortcuts"
                    hint: "~/.local/share/applications"
                    last: true
                    Btn { text: "Create"; small: true; enabled: !page.running; onClicked: app.runCli("Shortcuts", JSON.stringify(["shortcut", "spt"])) }
                }
            }
        }

        // Output
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
                    Lbl { text: "spt-additions output · " + page.log.count + " lines"; Layout.fillWidth: true }
                    Btn { text: "Clear"; small: true; enabled: !page.running; onClicked: page.log.clear() }
                }
                Rectangle { anchors.bottom: parent.bottom; width: parent.width; height: 1; color: Theme.line }
            }
            LogView { Layout.fillWidth: true; Layout.preferredHeight: 260; model: page.log }
        }
    }
}
