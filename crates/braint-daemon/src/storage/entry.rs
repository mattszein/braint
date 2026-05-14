use braint_proto::{Entry, EntryKind};

pub fn encode_kind(kind: EntryKind) -> &'static str {
    match kind {
        EntryKind::Idea => "idea",
    }
}

pub fn decode_kind(s: &str) -> Option<EntryKind> {
    match s {
        "idea" => Some(EntryKind::Idea),
        _ => None,
    }
}

/// Bind entry fields to SQLite parameters.
/// Returns a fixed-size array to avoid temporary lifetime issues with `params!`.
pub fn bind_entry(entry: &Entry) -> [rusqlite::types::Value; 9] {
    use rusqlite::types::Value;
    [
        Value::Blob(entry.id.0.as_bytes().to_vec()),
        Value::Text(encode_kind(entry.kind).to_string()),
        Value::Text(entry.body.clone()),
        Value::Integer(entry.created_at.physical_ms as i64),
        Value::Integer(entry.created_at.logical as i64),
        Value::Blob(entry.created_on_device.0.as_bytes().to_vec()),
        Value::Integer(entry.last_modified_at.physical_ms as i64),
        Value::Integer(entry.last_modified_at.logical as i64),
        Value::Blob(entry.last_modified_on_device.0.as_bytes().to_vec()),
    ]
}

pub fn row_to_entry(row: &rusqlite::Row) -> rusqlite::Result<Entry> {
    use braint_proto::{DeviceId, EntryId, HybridLogicalClock};
    use uuid::Uuid;

    let id_bytes: Vec<u8> = row.get(0)?;
    let kind_str: String = row.get(1)?;
    let body: String = row.get(2)?;
    let created_at_physical_ms: i64 = row.get(3)?;
    let created_at_logical: i64 = row.get(4)?;
    let created_on_device_bytes: Vec<u8> = row.get(5)?;
    let last_modified_at_physical_ms: i64 = row.get(6)?;
    let last_modified_at_logical: i64 = row.get(7)?;
    let last_modified_on_device_bytes: Vec<u8> = row.get(8)?;

    let id = EntryId(Uuid::from_slice(&id_bytes).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Blob, Box::new(e))
    })?);
    let kind = decode_kind(&kind_str).ok_or_else(|| {
        rusqlite::Error::FromSqlConversionFailure(
            1,
            rusqlite::types::Type::Text,
            Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("unknown kind: {kind_str}"),
            )),
        )
    })?;
    let device_from_bytes = |bytes: Vec<u8>| -> rusqlite::Result<DeviceId> {
        Uuid::from_slice(&bytes).map(DeviceId).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Blob, Box::new(e))
        })
    };

    let created_device = device_from_bytes(created_on_device_bytes)?;

    Ok(Entry {
        id,
        kind,
        body,
        created_at: HybridLogicalClock {
            physical_ms: created_at_physical_ms as u64,
            logical: created_at_logical as u32,
            device_id: created_device,
        },
        created_on_device: created_device,
        last_modified_at: HybridLogicalClock {
            physical_ms: last_modified_at_physical_ms as u64,
            logical: last_modified_at_logical as u32,
            device_id: device_from_bytes(last_modified_on_device_bytes.clone())?,
        },
        last_modified_on_device: device_from_bytes(last_modified_on_device_bytes)?,
    })
}
