//! Request dispatch — maps JSON-RPC method names to handlers.

pub mod ingest;

pub use ingest::IngestHandler;
