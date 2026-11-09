import QtQuick
import QtQuick.Controls.Basic

// Small outlined tag. tone: "", "acc", "ok", "err"
Rectangle {
    id: tag
    property string text: ""
    property string tone: ""
    readonly property color c: tone === "acc" ? Theme.accent : (tone === "ok" ? Theme.ok : (tone === "err" ? Theme.err : Theme.muted))
    implicitWidth: label.implicitWidth + 12
    implicitHeight: 20
    color: "transparent"
    border.width: 1
    border.color: tone === "" ? Theme.line2 : c
    Label {
        id: label
        anchors.centerIn: parent
        text: tag.text
        color: tag.c
        font.family: Theme.mono
        font.pixelSize: 10
        font.capitalization: Font.AllUppercase
        font.letterSpacing: 0.8
    }
}
