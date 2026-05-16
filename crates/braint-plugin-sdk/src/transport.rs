//! Synchronous stdin/stdout transport using 4-byte big-endian length-prefix framing.
//!
//! This module uses only `std::io` — no tokio — because plugins are simple
//! single-threaded binaries.

use crate::error::Result;
use std::io::{Read, Write};

/// Read one length-prefixed JSON frame from a reader.
///
/// The wire format is: `[u32 big-endian length][payload bytes]`.
pub fn read_frame<R: Read>(reader: &mut R) -> Result<Vec<u8>> {
    let mut len_buf = [0u8; 4];
    reader.read_exact(&mut len_buf)?;
    let len = u32::from_be_bytes(len_buf) as usize;
    let mut buf = vec![0u8; len];
    reader.read_exact(&mut buf)?;
    Ok(buf)
}

/// Write one length-prefixed JSON frame to a writer.
///
/// The wire format is: `[u32 big-endian length][payload bytes]`.
pub fn write_frame<W: Write>(writer: &mut W, data: &[u8]) -> Result<()> {
    let len = (data.len() as u32).to_be_bytes();
    writer.write_all(&len)?;
    writer.write_all(data)?;
    writer.flush()?;
    Ok(())
}
