import QtQuick
import QtQuick.Controls.Basic

TextField {
    id: f
    implicitHeight: 38
    color: Theme.text
    placeholderTextColor: Theme.dim
    selectionColor: Theme.accent
    selectedTextColor: Theme.ink
    font.family: Theme.mono
    font.pixelSize: 13
    leftPadding: 10
    rightPadding: 10
    background: Rectangle {
        radius: Theme.radius
        color: f.readOnly ? Theme.panel : Theme.bg
        border.width: 1
        border.color: f.activeFocus ? Theme.accent : Theme.line2
    }
}
