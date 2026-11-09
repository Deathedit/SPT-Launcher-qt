import QtQuick
import QtQuick.Controls.Basic

// Square-cornered mono button. kind: "" (outline), "pri" (accent), "bad" (danger)
Button {
    id: b
    property string kind: ""
    property bool small: false
    property bool big: false
    property string iconName: ""

    readonly property bool hot: hovered && enabled
    readonly property color fg: kind === "pri" ? Theme.ink : (kind === "bad" ? Theme.err : (hot ? Theme.accent : Theme.text))

    implicitHeight: big ? 56 : (small ? 32 : 36)
    leftPadding: big ? 26 : (small ? 10 : 14)
    rightPadding: leftPadding
    opacity: enabled ? 1 : 0.35
    focusPolicy: Qt.StrongFocus
    hoverEnabled: true

    background: Rectangle {
        radius: Theme.radius
        color: b.kind === "pri" ? (b.hot ? Theme.accentHover : Theme.accent)
             : (b.kind === "bad" && b.hot ? Theme.errRow : "transparent")
        border.width: 1
        border.color: b.kind === "pri" ? Theme.accent : (b.kind === "bad" ? Theme.err : (b.hot ? Theme.accent : Theme.line2))
        Rectangle {
            visible: b.visualFocus
            anchors.fill: parent
            anchors.margins: -3
            color: "transparent"
            border.color: Theme.accent
            border.width: 1
            radius: Theme.radius
        }
    }

    contentItem: Row {
        spacing: 8
        Icon {
            visible: b.iconName !== ""
            name: b.iconName
            color: b.fg
            width: b.big ? 20 : 16
            height: width
            anchors.verticalCenter: parent.verticalCenter
        }
        Label {
            text: b.text
            color: b.fg
            font.family: Theme.mono
            font.pixelSize: b.big ? 14 : (b.small ? 11 : 12)
            font.weight: Font.Medium
            font.capitalization: Font.AllUppercase
            font.letterSpacing: b.big ? 2 : 1
            anchors.verticalCenter: parent.verticalCenter
        }
    }
}
