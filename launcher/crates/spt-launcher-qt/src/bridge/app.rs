//! Application-level Qt helpers not covered by cxx-qt-lib (window icon, quit policy)

#[cxx_qt::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
        include!("cxx-qt-lib/qguiapplication.h");
        type QGuiApplication = cxx_qt_lib::QGuiApplication;

        include!("spt-launcher-qt/cpp/app.h");
        fn spt_new_application() -> UniquePtr<QGuiApplication>;
        fn spt_set_window_icon(path: &QString);
        fn spt_set_quit_on_last_window_closed(quit: bool);
    }
}

pub use ffi::{
    spt_new_application as new_application, spt_set_quit_on_last_window_closed as set_quit_on_last_window_closed,
    spt_set_window_icon as set_window_icon,
};
