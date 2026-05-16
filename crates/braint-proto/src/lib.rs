//! braint-proto — wire types and protocol contracts.
//!
//! This crate contains everything that crosses a crate boundary.
//! No logic, no I/O, no async.

pub mod entry;
pub mod error_codes;
pub mod jsonrpc;
pub mod method;
pub mod plugin;

pub use entry::*;
pub use error_codes::*;
pub use jsonrpc::*;
pub use method::*;
pub use plugin::{PluginManifest, PluginVerbRequest, PluginVerbResponse, VerbManifest};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
