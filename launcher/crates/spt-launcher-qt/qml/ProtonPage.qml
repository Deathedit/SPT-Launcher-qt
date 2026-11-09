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

    readonly property var protons: JSON.parse(app.protonsJson)
    readonly property string latestGe: {
        for (let i = 0; i < protons.length; i++)
            if (protons[i].kind === "GE" && /^GE-Proton\d/.test(protons[i].name)) return protons[i].name
        return ""
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
                Lbl { text: "~/.steam/steam/compatibilitytools.d" }
                Label { text: "Proton"; color: Theme.text; font.family: Theme.display; font.pixelSize: 28; font.weight: Font.DemiBold; font.letterSpacing: 1 }
            }
            Item { Layout.fillWidth: true }
            Btn { text: "Rescan"; iconName: "restart"; onClicked: app.refresh() }
            Btn {
                text: app.taskState === "running" && app.taskName === "Update GE-Proton" ? "Downloading…" : "Get latest GE-Proton"
                enabled: app.taskState !== "running" && app.protonPath.indexOf("GE-Proton") >= 0
                onClicked: app.runCli("Update GE-Proton", JSON.stringify(["update", "proton"]))
            }
        }

        Rectangle {
            Layout.fillWidth: true
            implicitHeight: list.implicitHeight + 2
            color: "transparent"
            border.color: Theme.line
            ColumnLayout {
                id: list
                anchors.fill: parent
                anchors.margins: 1
                spacing: 0
                Label {
                    visible: page.protons.length === 0
                    Layout.margins: 16
                    text: "No Proton builds found. Install GE-Proton with the guided installer or ProtonUp-Qt."
                    color: Theme.muted
                }
                Repeater {
                    model: page.protons
                    Rectangle {
                        id: row
                        required property var modelData
                        required property int index
                        readonly property bool on: app.protonPath === modelData.path
                        Layout.fillWidth: true
                        implicitHeight: 62
                        color: on ? Theme.panel2 : (ma.containsMouse ? Theme.panel : "transparent")
                        Rectangle { visible: row.index > 0; width: parent.width; height: 1; color: Theme.line }
                        MouseArea { id: ma; anchors.fill: parent; hoverEnabled: true; onClicked: if (!row.on) app.selectProton(row.modelData.path) }
                        RowLayout {
                            anchors.fill: parent
                            anchors.leftMargin: 14
                            anchors.rightMargin: 14
                            spacing: 16
                            AbstractButton {
                                implicitWidth: 20
                                implicitHeight: 20
                                onClicked: if (!row.on) app.selectProton(row.modelData.path)
                                Accessible.role: Accessible.RadioButton
                                Accessible.name: "Use " + row.modelData.name
                                background: Rectangle {
                                    radius: 10
                                    color: row.on ? Theme.accent : "transparent"
                                    border.color: row.on ? Theme.accent : Theme.line2
                                    Rectangle { visible: row.on; anchors.centerIn: parent; width: 12; height: 12; radius: 6; color: "transparent"; border.color: Theme.panel2; border.width: 4 }
                                }
                            }
                            ColumnLayout {
                                Layout.fillWidth: true
                                spacing: 3
                                Label { text: row.modelData.name; color: Theme.text; font.weight: Font.DemiBold; font.pixelSize: 14 }
                                Label { text: row.modelData.path; color: Theme.dim; font.family: Theme.mono; font.pixelSize: 11; Layout.fillWidth: true; elide: Text.ElideMiddle }
                            }
                            Tag { text: row.modelData.kind }
                            Tag { visible: row.on; text: "In use"; tone: "acc" }
                            Tag { visible: row.modelData.name === page.latestGe; text: "Newest GE"; tone: "ok" }
                        }
                    }
                }
            }
        }

        Label {
            Layout.fillWidth: true
            text: "GE-Proton is recommended. The selected build is saved to LauncherSettings.json (shared with the official launcher) and app.conf, and used for every launch."
            color: Theme.muted
            wrapMode: Text.Wrap
            font.pixelSize: 13
            padding: 10
            background: Rectangle { color: "transparent"; border.color: Theme.line2 }
        }

        Panel {
            visible: app.taskName === "Update GE-Proton" && page.log.count > 0
            Layout.fillWidth: true
            LogView { Layout.fillWidth: true; Layout.preferredHeight: 180; model: page.log }
        }
    }
}
