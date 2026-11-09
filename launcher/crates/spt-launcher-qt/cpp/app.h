#pragma once
// Small QGuiApplication statics that cxx-qt-lib does not wrap.

#include <memory>

#include <QtCore/QString>
#include <QtGui/QGuiApplication>
#include <QtGui/QIcon>
#include <QtWidgets/QApplication>

// A QApplication (not just QGuiApplication): the tray icon falls back to the Qt Widgets
// implementation (XEmbed) when no StatusNotifierItem host is available.
inline std::unique_ptr<QGuiApplication> spt_new_application()
{
    static int argc = 1;
    static char arg0[] = "spt-launcher-qt";
    static char *argv[] = {arg0, nullptr};
    return std::make_unique<QApplication>(argc, argv);
}

inline void spt_set_window_icon(const QString &path)
{
    QGuiApplication::setWindowIcon(QIcon(path));
}

inline void spt_set_quit_on_last_window_closed(bool quit)
{
    QGuiApplication::setQuitOnLastWindowClosed(quit);
}
