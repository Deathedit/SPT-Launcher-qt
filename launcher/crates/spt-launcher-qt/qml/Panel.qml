import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts

// Bordered section with an optional header row
Rectangle {
    id: p
    property string title: ""
    property string subtitle: ""
    default property alias content: body.data
    property alias header: headerExtra.data
    color: "transparent"
    border.width: 1
    border.color: Theme.line
    implicitHeight: col.implicitHeight + 2
    implicitWidth: col.implicitWidth + 2

    ColumnLayout {
        id: col
        anchors.fill: parent
        anchors.margins: 1
        spacing: 0
        RowLayout {
            visible: p.title !== ""
            Layout.fillWidth: true
            Layout.margins: 14
            Layout.leftMargin: 16
            spacing: 12
            Label {
                text: p.title
                color: Theme.text
                font.family: Theme.display
                font.pixelSize: 19
                font.weight: Font.DemiBold
                font.letterSpacing: 0.5
            }
            Lbl { text: p.subtitle; visible: p.subtitle !== ""; Layout.fillWidth: true }
            Item { Layout.fillWidth: p.subtitle === "" }
            Row { id: headerExtra; spacing: 8 }
        }
        Rectangle { visible: p.title !== ""; Layout.fillWidth: true; height: 1; color: Theme.line }
        ColumnLayout {
            id: body
            Layout.fillWidth: true
            Layout.fillHeight: true
            spacing: 0
        }
    }
}
