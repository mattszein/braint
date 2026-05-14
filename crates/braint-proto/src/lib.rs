//! braint-proto — wire types and protocol contracts.
//!
//! This crate contains everything that crosses a crate boundary.
//! No logic, no I/O, no async.

pub mod entry;
pub mod jsonrpc;
pub mod method;

pub use entry::*;
pub use jsonrpc::*;
pub use method::*;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
