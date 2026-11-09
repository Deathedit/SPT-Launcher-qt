pragma Singleton
import QtQuick

// "Field-kit dark" tokens (same as the design canvas)
QtObject {
    readonly property color bg: "#121110"
    readonly property color panel: "#181715"
    readonly property color panel2: "#201e1b"
    readonly property color line: "#2c2a26"
    readonly property color line2: "#3d3a34"
    readonly property color text: "#ece7dd"
    readonly property color muted: "#a7a195"
    readonly property color dim: "#8a8479"
    readonly property color accent: "#e3a43c"
    readonly property color accentHover: "#f0b552"
    readonly property color ink: "#1b1407"
    readonly property color ok: "#8fc06f"
    readonly property color err: "#f07c69"
    readonly property color logBg: "#0c0b0a"
    readonly property color warnRow: "#221a10"
    readonly property color errRow: "#2a1714"

    readonly property string display: "Chakra Petch"
    readonly property string sans: "IBM Plex Sans"
    readonly property string mono: "JetBrains Mono"

    readonly property int radius: 2
    readonly property int gutter: 22

    function levelColor(lv) {
        switch (lv) {
        case "ok": return ok
        case "warn": return accent
        case "error": return err
        case "cmd": return muted
        default: return text
        }
    }
}
