//! braint-client — IPC client for talking to the daemon.
//!
//! Handles connection, length-prefixed JSON-RPC framing, and request/response.

pub mod client;
pub mod error;
pub mod framing;

pub use client::Client;
pub use error::{ClientError, Result};
