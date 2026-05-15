//! braint-core — pure domain logic.
//!
//! No SQLite, no sockets, no tokio. Parse, validate, filter, clock.

pub mod clock;
pub mod error;
pub mod parse;

pub use clock::Clock;
pub use error::{CoreError, Result};
pub use parse::{VerbInvocation, parse_ingest, parse_verb};
