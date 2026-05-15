use braint_proto::{Entry, EntryKind, PrincipalTag, TagSet};

/// Encode an [`EntryKind`] as the canonical storage string.
pub fn encode_kind(kind: EntryKind) -> &'static str {
    match kind {
        EntryKind::Idea => "idea",
        EntryKind::Todo => "todo",
        EntryKind::Note => "note",
        EntryKind::Capture => "capture",
    }
}

/// Decode a storage string back into an [`EntryKind`].
pub fn decode_kind(s: &str) -> Option<EntryKind> {
    match s {
        "idea" => Some(EntryKind::Idea),
        "todo" => Some(EntryKind::Todo),
        "note" => Some(EntryKind::Note),
        "capture" => Some(EntryKind::Capture),
        _ => None,
    }
}

/// Bind entry fields to SQLite parameters (columns 1-12).
///
/// Returns a fixed-size array to avoid temporary lifetime issues with `params!`.
pub fn bind_entry(entry: &Entry) -> [rusqlite::types::Value; 12] {
    use rusqlite::types::Value;

    let principal_json =
        serde_json::to_string(&entry.tags.principal).unwrap_or_else(|_| "[]".to_string());
    let free_json = serde_json::to_string(&entry.tags.free).unwrap_or_else(|_| "[]".to_string());
    let project_str = entry.project.as_ref().map(|p| p.0.clone());

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
        match project_str {
            Some(s) => Value::Text(s),
            None => Value::Null,
        },
        Value::Text(principal_json),
        Value::Text(free_json),
    ]
}

/// Reconstruct an [`Entry`] from a SQLite row (columns 0-11).
pub fn row_to_entry(row: &rusqlite::Row) -> rusqlite::Result<Entry> {
    use braint_proto::{DeviceId, EntryId, HybridLogicalClock, ProjectId};
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
    let project_str: Option<String> = row.get(9)?;
    let principal_json: String = row.get(10)?;
    let free_json: String = row.get(11)?;

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

    let project = project_str.map(ProjectId);

    let principal: Vec<PrincipalTag> = serde_json::from_str(&principal_json).unwrap_or_default();
    let free: Vec<String> = serde_json::from_str(&free_json).unwrap_or_default();

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
        project,
        tags: TagSet { principal, free },
    })
}
