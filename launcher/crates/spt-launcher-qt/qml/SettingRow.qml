import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts

// "Label / code hint ........ control" row with a dashed divider
Item {
    id: r
    property string label: ""
    property string hint: ""
    property bool last: false
    default property alias control: slot.data
    Layout.fillWidth: true
    implicitHeight: Math.max(texts.implicitHeight, slot.implicitHeight) + 20

    ColumnLayout {
        id: texts
        anchors.left: parent.left
        anchors.right: slot.left
        anchors.rightMargin: 16
        anchors.verticalCenter: parent.verticalCenter
        spacing: 2
        Label { text: r.label; color: Theme.text; font.family: Theme.sans; font.pixelSize: 14; Layout.fillWidth: true; elide: Text.ElideRight }
        Label { text: r.hint; visible: r.hint !== ""; color: Theme.dim; font.family: Theme.mono; font.pixelSize: 11; Layout.fillWidth: true; elide: Text.ElideMiddle }
    }
    Row {
        id: slot
        anchors.right: parent.right
        anchors.verticalCenter: parent.verticalCenter
        spacing: 8
    }
    Row {
        visible: !r.last
        anchors.bottom: parent.bottom
        width: parent.width
        clip: true
        spacing: 3
        Repeater {
            model: Math.ceil(r.width / 6)
            Rectangle { width: 3; height: 1; color: Theme.line }
        }
    }
}
