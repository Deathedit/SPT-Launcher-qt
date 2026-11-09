//! The Qt side. One QObject (`Launcher`) exposes state as properties and
//! actions as invokables. All slow work runs on worker threads and posts
//! results back to the Qt thread with `CxxQtThread::queue`.

pub mod app;
pub mod launcher;
mod work;
