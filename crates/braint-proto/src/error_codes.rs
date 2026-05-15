//! Canonical JSON-RPC error code ranges for braint.
//!
//! Application error codes live in the range -32000 to -32099.
//! Plugin error codes are reserved at -32010 and below.

pub const ERR_PARSE: i32 = -32000;
pub const ERR_STORAGE: i32 = -32001;
pub const ERR_NOT_FOUND: i32 = -32002;
pub const ERR_TTL_EXPIRED: i32 = -32003;
pub const ERR_VALIDATION: i32 = -32004;
/// Plugin error code range start (Phase 4a).
pub const ERR_PLUGIN_BASE: i32 = -32010;
