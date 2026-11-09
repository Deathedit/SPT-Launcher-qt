//! Core of the SPT launcher: everything that is not UI.
//!
//! - [`api`]: the SPT server's launcher protocol (zlib JSON over HTTPS)
//! - [`game`]: client checks, file cleanup, patching, umu-run launch
//! - [`bundles`]: mod bundle manifest check and parallel download
//! - [`config`]: spt-additions' `app.conf`, the shared `LauncherSettings.json`, our UI state
//! - [`process`]: child processes with line-based output (server, installer, game)
//! - [`proton`], [`steam`]: Proton build discovery, Steam library detection
//!
//! The protocol and file layout follow the official SPT launcher (NCSA licensed).

pub mod api;
pub mod bundles;
pub mod config;
pub mod game;
pub mod models;
pub mod process;
pub mod proton;
pub mod steam;
pub mod text;
