//! Request dispatch — method name to handler module.
//!
//! Each public module exposes free async functions that accept `&DaemonState`.

pub mod confirm;
pub mod ingest;
pub mod list;
pub mod subscribe;
