import QtQuick
import QtQuick.Controls.Basic

// Status pill with a dot. state: "run", "busy", "off", "na", ""
Rectangle {
    id: pill
    property string text: ""
    property string status: ""
    implicitWidth: row.implicitWidth + 20
    implicitHeight: 28
    color: "transparent"
    border.width: 1
    border.color: Theme.line2
    Row {
        id: row
        anchors.centerIn: parent
        spacing: 8
        Rectangle {
            id: dot
            width: 7
            height: 7
            radius: 4
            anchors.verticalCenter: parent.verticalCenter
            color: pill.status === "run" ? Theme.ok : (pill.status === "busy" ? Theme.accent : (pill.status === "off" ? Theme.err : Theme.dim))
            SequentialAnimation on opacity {
                running: pill.status === "busy"
                loops: Animation.Infinite
                NumberAnimation { to: 0.2; duration: 500 }
                NumberAnimation { to: 1; duration: 500 }
                onStopped: dot.opacity = 1
            }
        }
        Label {
            text: pill.text
            color: pill.status === "na" ? Theme.muted : Theme.text
            font.family: Theme.mono
            font.pixelSize: 11
            font.capitalization: Font.AllUppercase
            font.letterSpacing: 1
            anchors.verticalCenter: parent.verticalCenter
        }
    }
}
