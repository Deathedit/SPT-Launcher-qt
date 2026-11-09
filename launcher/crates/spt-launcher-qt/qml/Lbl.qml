import QtQuick

// Mono, uppercase micro label
Text {
    color: Theme.muted
    font.family: Theme.mono
    font.pixelSize: 11
    font.capitalization: Font.AllUppercase
    font.letterSpacing: 1
    elide: Text.ElideRight
}
