import QtQuick
import QtQuick.Shapes

// Line icons on a 20x20 grid (same paths as the mockup)
Item {
    id: root
    property string name: "play"
    property color color: Theme.text
    property real stroke: 1.6
    implicitWidth: 16
    implicitHeight: 16

    readonly property var paths: ({
        "play": "M6 4 L16 10 L6 16 Z",
        "stop": "M5 5 H15 V15 H5 Z",
        "restart": "M16 10 A6 6 0 1 1 14.2 5.7 M16 3 V7 H12",
        "plus": "M10 4 V16 M4 10 H16",
        "check": "M4 10.5 L8 14.5 L16 5.5",
        "dot": "M9 10 A1 1 0 1 0 11 10 A1 1 0 1 0 9 10",
        "spin": "M10 4 A6 6 0 0 1 16 10",
        "server": "M3 4 H17 V9 H3 Z M3 11 H17 V16 H3 Z M6 6.5 H6.1 M6 13.5 H6.1",
        "brand": "M5.5 4 H14.5 A1.5 1.5 0 0 1 16 5.5 V14.5 A1.5 1.5 0 0 1 14.5 16 H5.5 A1.5 1.5 0 0 1 4 14.5 V5.5 A1.5 1.5 0 0 1 5.5 4 Z M4 7.5 H16 M4 12.5 H16",
        "folder": "M3 6 V15 H17 V7 H9 L7.5 5 H3 Z",
        "link": "M8 12 L12 8 M9 5 H15 V11 M15 5 L9 11 M13 13 V15 H5 V7 H7"
    })

    Shape {
        anchors.fill: parent
        preferredRendererType: Shape.CurveRenderer
        ShapePath {
            strokeColor: root.color
            strokeWidth: root.stroke
            fillColor: root.name === "play" ? root.color : "transparent"
            capStyle: ShapePath.RoundCap
            joinStyle: ShapePath.RoundJoin
            scale: Qt.size(root.width / 20, root.height / 20)
            PathSvg { path: root.paths[root.name] || "" }
        }
    }
}
