import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import SptLauncher

// Client mods from sp-mod.com: search, install (with dependencies), update, remove
Flickable {
    id: page
    required property var app
    required property var win
    required property ListModel log

    contentHeight: col.implicitHeight + 50
    clip: true
    boundsBehavior: Flickable.StopAtBounds
    ScrollBar.vertical: ScrollBar {}

    readonly property var results: JSON.parse(app.forgeResults)
    readonly property var installed: JSON.parse(app.installedMods)
    readonly property var others: JSON.parse(app.otherMods)
    readonly property bool busy: app.modTask !== ""
    readonly property bool canChange: app.installed && !busy && app.gameState === "idle"
    property bool loaded: false
    property int pageNo: 1
    property string lastQuery: ""
    property int confirmId: 0

    function entry(id) {
        for (let i = 0; i < installed.length; i++)
            if (installed[i].id === id)
                return installed[i]
        return null
    }
    function count(n) {
        return n >= 1e6 ? (n / 1e6).toFixed(1) + "M" : (n >= 1e3 ? (n / 1e3).toFixed(1) + "K" : String(n))
    }
    function search() {
        pageNo = 1
        lastQuery = searchField.text.trim()
        app.forgeSearch(lastQuery, 1)
    }

    function load() {
        if (!visible || loaded)
            return
        loaded = true
        app.modsReload()
        if (results.length === 0)
            app.forgeSearch("", 1)
        // Developer aid: SPT_LAUNCHER_MOD_INSTALL=<mod id> installs a mod when the page opens
        const testId = parseInt(app.envVar("SPT_LAUNCHER_MOD_INSTALL"))
        if (testId > 0)
            app.modInstall(testId)
    }
    onVisibleChanged: load()
    Component.onCompleted: Qt.callLater(load)
    Connections {
        target: page.app
        function onSptPathChanged() { if (page.loaded) page.app.modsReload() }
    }

    ColumnLayout {
        id: col
        x: 24
        y: 22
        width: page.width - 48
        spacing: 18

        // ── Header ──────────────────────────────────────────────────
        RowLayout {
            Layout.fillWidth: true
            spacing: 12
            ColumnLayout {
                spacing: 2
                Lbl { text: "sp-mod.com · " + (app.sptVersion ? "SPT " + app.sptVersion : "any SPT version") + (app.localEnabled ? "" : " · client-only") }
                Label { text: "Mods"; color: Theme.text; font.family: Theme.display; font.pixelSize: 28; font.weight: Font.DemiBold; font.letterSpacing: 1 }
            }
            Item { Layout.fillWidth: true }
            Btn { text: "Check updates"; iconName: "restart"; enabled: app.installed && !page.busy && page.installed.length > 0; onClicked: app.modsReload() }
            Btn { text: "sp-mod.com"; iconName: "link"; onClicked: Qt.openUrlExternally("https://sp-mod.com") }
        }

        Rectangle {
            visible: !app.installed
            Layout.fillWidth: true
            implicitHeight: notInst.implicitHeight + 24
            color: "transparent"
            border.color: Theme.accent
            Label {
                id: notInst
                anchors.fill: parent
                anchors.margins: 12
                text: "SPT is not installed yet. Mods are copied into the SPT folder, so install SPT first."
                color: Theme.accent
                wrapMode: Text.Wrap
            }
        }

        // ── Search ─────────────────────────────────────────────────
        Rectangle {
            Layout.fillWidth: true
            implicitHeight: searchCol.implicitHeight + 24
            color: "transparent"
            border.color: Theme.line
            ColumnLayout {
                id: searchCol
                anchors.fill: parent
                anchors.margins: 12
                spacing: 8
                RowLayout {
                    Layout.fillWidth: true
                    spacing: 10
                    Field {
                        id: searchField
                        Layout.fillWidth: true
                        placeholderText: "Search sp-mod.com, or paste a mod link (sp-mod.com/mod/1775/…)"
                        Accessible.name: "Search mods"
                        onAccepted: page.search()
                    }
                    Btn { text: "Search"; kind: "pri"; onClicked: page.search() }
                }
                Label {
                    Layout.fillWidth: true
                    text: "Mods are third-party code, downloaded from wherever the author hosts them. VirusTotal links are in the log. "
                        + (app.localEnabled ? "Client and server parts are installed." : "Client-only mode: only the BepInEx (client) part is installed; the server host needs the server part.")
                    color: Theme.dim
                    font.pixelSize: 12
                    wrapMode: Text.Wrap
                }
            }
        }

        // ── Running operation ──────────────────────────────────────
        ColumnLayout {
            visible: page.busy
            Layout.fillWidth: true
            spacing: 8
            Label { text: app.modTask; color: Theme.accent; font.family: Theme.mono; font.pixelSize: 12; elide: Text.ElideMiddle; Layout.fillWidth: true }
            Rectangle {
                Layout.fillWidth: true
                height: 8
                color: Theme.panel2
                border.color: Theme.line2
                clip: true
                Rectangle {
                    visible: app.modPct >= 0
                    x: 1; y: 1
                    height: parent.height - 2
                    width: (parent.width - 2) * Math.max(0, app.modPct) / 100
                    color: Theme.accent
                }
                Rectangle {
                    visible: app.modPct < 0
                    width: parent.width * 0.25
                    height: parent.height
                    color: Theme.accent
                    NumberAnimation on x { from: -page.width * 0.25; to: page.width; duration: 1400; loops: Animation.Infinite; running: page.busy && app.modPct < 0 }
                }
            }
        }

        RowLayout {
            Layout.fillWidth: true
            spacing: 18

            // ── Results ────────────────────────────────────────────
            Panel {
                Layout.fillWidth: true
                Layout.alignment: Qt.AlignTop
                title: page.lastQuery !== "" ? "Results" : "Popular"
                subtitle: app.forgeState === "searching" ? "searching…" : page.results.length + (page.results.length === 1 ? " mod" : " mods")

                Label {
                    visible: app.forgeState === "error"
                    Layout.margins: 14
                    Layout.fillWidth: true
                    text: app.forgeError
                    color: Theme.err
                    font.family: Theme.mono
                    font.pixelSize: 12
                    wrapMode: Text.Wrap
                }
                Label {
                    visible: app.forgeState === "idle" && page.results.length === 0
                    Layout.margins: 14
                    text: "No mods found" + (app.sptVersion ? " for SPT " + app.sptVersion : "") + "."
                    color: Theme.muted
                }

                Repeater {
                    model: page.results
                    Rectangle {
                        id: row
                        required property var modelData
                        required property int index
                        readonly property var inst: page.entry(modelData.id)
                        Layout.fillWidth: true
                        implicitHeight: resRow.implicitHeight + 24
                        color: "transparent"
                        Rectangle { visible: row.index > 0; width: parent.width; height: 1; color: Theme.line }
                        RowLayout {
                            id: resRow
                            anchors.fill: parent
                            anchors.margins: 12
                            anchors.leftMargin: 14
                            spacing: 14
                            // Letter tile: files.sp-mod.com refuses hotlinked thumbnails
                            Rectangle {
                                Layout.preferredWidth: 48
                                Layout.preferredHeight: 48
                                Layout.alignment: Qt.AlignTop
                                color: Theme.panel2
                                border.color: row.inst ? Theme.ok : Theme.line2
                                Label {
                                    anchors.centerIn: parent
                                    text: row.modelData.name.replace(/[^A-Za-z0-9]/g, "").slice(0, 2).toUpperCase()
                                    color: row.inst ? Theme.ok : Theme.accent
                                    font.family: Theme.display
                                    font.pixelSize: 18
                                    font.weight: Font.DemiBold
                                    font.letterSpacing: 1
                                }
                            }
                            ColumnLayout {
                                Layout.fillWidth: true
                                Layout.minimumWidth: 0
                                Layout.preferredWidth: 100
                                spacing: 4
                                // Plain Item: the parent layout sets its width, the name elides to leave room for the tags
                                Item {
                                    id: nameRow
                                    Layout.fillWidth: true
                                    implicitHeight: nameLabel.implicitHeight
                                    Label {
                                        id: nameLabel
                                        width: Math.max(0, Math.min(implicitWidth, nameRow.width - tagRow.implicitWidth - 8))
                                        text: row.modelData.name
                                        color: Theme.text
                                        font.weight: Font.DemiBold
                                        font.pixelSize: 14
                                        elide: Text.ElideRight
                                    }
                                    Row {
                                        id: tagRow
                                        x: nameLabel.width + 8
                                        anchors.verticalCenter: nameLabel.verticalCenter
                                        spacing: 6
                                        Tag { visible: row.modelData.fika; text: "Fika" }
                                        Tag { visible: row.inst !== null; text: row.inst ? row.inst.version : ""; tone: "ok" }
                                    }
                                }
                                Label {
                                    text: (row.modelData.author ? "by " + row.modelData.author + " · " : "") + page.count(row.modelData.downloads) + " downloads"
                                    color: Theme.dim
                                    font.family: Theme.mono
                                    font.pixelSize: 11
                                }
                                Label {
                                    visible: text !== ""
                                    text: row.modelData.teaser
                                    color: Theme.muted
                                    font.pixelSize: 13
                                    wrapMode: Text.Wrap
                                    maximumLineCount: 2
                                    elide: Text.ElideRight
                                    Layout.fillWidth: true
                                }
                            }
                            ColumnLayout {
                                Layout.alignment: Qt.AlignVCenter
                                Layout.preferredWidth: 128
                                Layout.maximumWidth: 128
                                spacing: 6
                                Btn {
                                    Layout.fillWidth: true
                                    small: true
                                    text: !row.inst ? "Install" : (row.inst.update !== "" ? "Update" : "Installed")
                                    kind: row.inst && row.inst.update === "" ? "" : "pri"
                                    iconName: row.inst && row.inst.update === "" ? "check" : ""
                                    enabled: page.canChange && (!row.inst || row.inst.update !== "")
                                    onClicked: app.modInstall(row.modelData.id)
                                }
                                Btn { Layout.fillWidth: true; small: true; text: "Page"; iconName: "link"; onClicked: Qt.openUrlExternally(row.modelData.url) }
                            }
                        }
                    }
                }

                Btn {
                    visible: app.forgeHasMore && app.forgeState === "idle"
                    Layout.margins: 12
                    Layout.alignment: Qt.AlignHCenter
                    small: true
                    text: "Load more"
                    onClicked: { page.pageNo += 1; app.forgeSearch(page.lastQuery, page.pageNo) }
                }
            }

            // ── Installed / other ──────────────────────────────────
            ColumnLayout {
                Layout.preferredWidth: 360
                Layout.maximumWidth: 360
                Layout.alignment: Qt.AlignTop
                spacing: 18

                Panel {
                    Layout.fillWidth: true
                    title: "Installed"
                    subtitle: page.installed.length + " managed"
                    Label {
                        visible: page.installed.length === 0
                        Layout.margins: 14
                        Layout.fillWidth: true
                        text: "Mods you install here show up in this list, with updates and removal."
                        color: Theme.muted
                        font.pixelSize: 13
                        wrapMode: Text.Wrap
                    }
                    Repeater {
                        model: page.installed
                        Rectangle {
                            id: irow
                            required property var modelData
                            required property int index
                            readonly property bool confirming: page.confirmId === modelData.id
                            Layout.fillWidth: true
                            implicitHeight: instCol.implicitHeight + 24
                            color: confirming ? Theme.errRow : "transparent"
                            Rectangle { visible: irow.index > 0; width: parent.width; height: 1; color: Theme.line }
                            ColumnLayout {
                                id: instCol
                                anchors.fill: parent
                                anchors.margins: 12
                                anchors.leftMargin: 14
                                spacing: 6
                                RowLayout {
                                    Layout.fillWidth: true
                                    spacing: 8
                                    Label { text: irow.modelData.name; color: Theme.text; font.weight: Font.DemiBold; font.pixelSize: 13; elide: Text.ElideRight; Layout.fillWidth: true }
                                    Tag { text: irow.modelData.version }
                                }
                                Flow {
                                    Layout.fillWidth: true
                                    spacing: 6
                                    visible: irow.modelData.update !== "" || irow.modelData.serverSkipped || irow.modelData.requiredBy.length > 0
                                    Tag { visible: irow.modelData.update !== ""; text: "Update " + irow.modelData.update; tone: "acc" }
                                    Tag { visible: irow.modelData.serverSkipped; text: "Client part" }
                                    Tag { visible: irow.modelData.requiredBy.length > 0; text: "Dependency" }
                                }
                                Label {
                                    visible: irow.modelData.requiredBy.length > 0
                                    text: "Required by " + irow.modelData.requiredBy.join(", ")
                                    color: Theme.dim
                                    font.family: Theme.mono
                                    font.pixelSize: 11
                                    elide: Text.ElideRight
                                    Layout.fillWidth: true
                                }
                                RowLayout {
                                    Layout.fillWidth: true
                                    spacing: 6
                                    Btn { visible: irow.modelData.update !== "" && !irow.confirming; small: true; kind: "pri"; text: "Update"; enabled: page.canChange; onClicked: app.modInstall(irow.modelData.id) }
                                    Btn { visible: !irow.confirming; small: true; text: "Page"; iconName: "link"; onClicked: Qt.openUrlExternally(irow.modelData.url) }
                                    Item { Layout.fillWidth: true }
                                    Btn { visible: !irow.confirming; small: true; kind: "bad"; text: "Remove"; enabled: page.canChange; onClicked: page.confirmId = irow.modelData.id }
                                    Label { visible: irow.confirming; text: "Delete " + irow.modelData.files + " files?"; color: Theme.err; font.pixelSize: 12 }
                                    Btn { visible: irow.confirming; small: true; text: "Keep"; onClicked: page.confirmId = 0 }
                                    Btn { visible: irow.confirming; small: true; kind: "bad"; text: "Delete"; onClicked: { page.confirmId = 0; app.modRemove(irow.modelData.id) } }
                                }
                            }
                        }
                    }
                }

                Panel {
                    visible: page.others.length > 0
                    Layout.fillWidth: true
                    title: "Other mods"
                    subtitle: "not managed"
                    Label {
                        Layout.margins: 14
                        Layout.bottomMargin: 4
                        Layout.fillWidth: true
                        text: "Installed by hand or by another tool. The launcher doesn't touch these."
                        color: Theme.dim
                        font.pixelSize: 12
                        wrapMode: Text.Wrap
                    }
                    Repeater {
                        model: page.others
                        Label {
                            required property var modelData
                            Layout.leftMargin: 14
                            Layout.rightMargin: 14
                            Layout.bottomMargin: 4
                            Layout.fillWidth: true
                            text: modelData
                            color: Theme.muted
                            font.family: Theme.mono
                            font.pixelSize: 11
                            elide: Text.ElideMiddle
                        }
                    }
                    Item { implicitHeight: 10 }
                }
            }
        }

        // ── Log ────────────────────────────────────────────────────
        Panel {
            visible: page.log.count > 0
            Layout.fillWidth: true
            Rectangle {
                Layout.fillWidth: true
                implicitHeight: 40
                color: Theme.logBg
                RowLayout {
                    anchors.fill: parent
                    anchors.leftMargin: 12
                    anchors.rightMargin: 4
                    Lbl { text: "Mods · " + page.log.count + " lines"; Layout.fillWidth: true }
                    Btn { text: "Clear"; small: true; onClicked: page.log.clear() }
                }
            }
            LogView { Layout.fillWidth: true; Layout.preferredHeight: 200; model: page.log }
        }
    }
}
