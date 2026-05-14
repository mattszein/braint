//! braint-daemon — the background process.
//!
//! Owns the async runtime, socket, SQLite, and wiring between them.
//! Business logic delegates to `braint_core`; persistence delegates to `storage`.

pub mod error;
pub mod handler;
pub mod server;
pub mod storage;

pub use error::{DaemonError, Result};
