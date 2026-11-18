//! Vesper Chess — peer-to-peer terminal chess.
//!
//! Multiplayer has one authority: the process that hosts the game. The
//! joining process dials it directly. There is no central server in this
//! crate.
//!
//! `ui` → `app` → `domain`. `net` → `session` → `domain`. The UI reaches
//! the network only through `GameProvider`.

pub mod app;
pub mod domain;
pub mod identity;
pub mod net;
pub mod protocol;
pub mod session;
pub mod ui;
