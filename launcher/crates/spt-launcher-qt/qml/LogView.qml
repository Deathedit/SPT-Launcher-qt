import QtQuick
import QtQuick.Controls.Basic

// Terminal-style log bound to a ListModel {time, level, text}
Rectangle {
    id: v
    property alias model: list.model
    property bool follow: true
    property var filter: null   // array of levels to show, or null for all
    color: Theme.logBg
    clip: true

    function scrollToEnd() { if (v.follow) list.positionViewAtEnd() }

    ListView {
        id: list
        anchors.fill: parent
        anchors.topMargin: 8
        anchors.bottomMargin: 8
        boundsBehavior: Flickable.StopAtBounds
        ScrollBar.vertical: ScrollBar { policy: ScrollBar.AsNeeded }
        onCountChanged: Qt.callLater(v.scrollToEnd)
        delegate: Item {
            required property string time
            required property string level
            required property string text
            readonly property bool shown: v.filter === null || v.filter.indexOf(level) >= 0
            width: ListView.view.width
            height: shown ? Math.max(row.implicitHeight, 21) : 0
            visible: shown
            Row {
                id: row
                x: 14
                width: parent.width - 28
                spacing: 16
                Label { text: parent.parent.time; color: "#857f74"; font.family: Theme.mono; font.pixelSize: 12 }
                Label {
                    width: parent.width - 80
                    text: parent.parent.text
                    color: Theme.levelColor(parent.parent.level)
                    font.family: Theme.mono
                    font.pixelSize: 12
                    wrapMode: Text.WrapAnywhere
                    textFormat: Text.PlainText
                }
            }
        }
    }
}
