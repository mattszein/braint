//! braint-daemon — the background process.
//!
//! Owns the async runtime, socket, SQLite, and wiring between them.
//! Business logic delegates to `braint_core`; persistence delegates to `storage`.

pub mod config;
pub mod error;
pub mod handler;
pub mod pending;
pub mod server;
pub mod storage;
pub mod subscription;

pub use config::{DaemonConfig, load_or_create_device_id};
pub use error::{DaemonError, Result};
pub use subscription::SubscriptionManager;
