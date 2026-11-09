import QtQuick
import QtQuick.Controls.Basic

// 44x26 square switch
AbstractButton {
    id: t
    checkable: true
    implicitWidth: 44
    implicitHeight: 26
    focusPolicy: Qt.StrongFocus
    opacity: enabled ? 1 : 0.35
    background: Rectangle {
        radius: Theme.radius
        color: t.checked ? "#2a2112" : Theme.panel2
        border.width: 1
        border.color: t.checked || t.visualFocus ? Theme.accent : Theme.line2
        Rectangle {
            width: 16
            height: 16
            y: 4
            x: t.checked ? 22 : 4
            color: t.checked ? Theme.accent : Theme.dim
            Behavior on x { NumberAnimation { duration: 120 } }
        }
    }
}
