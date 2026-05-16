//! braint-plugin-hello — a minimal example plugin that contributes the `hello` verb.

use braint_plugin_sdk::Plugin;
use braint_proto::{
    DeviceId, Entry, EntryId, EntryKind, HybridLogicalClock, TagSet,
    plugin::PluginVerbResponse,
};

fn main() {
    Plugin::new("hello", "0.1.0")
        .verb(
            "hello",
            "Creates a capture entry greeting the argument",
            false,
            |req| {
                let body = if req.body.is_empty() {
                    "hello".to_string()
                } else {
                    format!("hello {}", req.body)
                };

                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as u64;
                let device_id = DeviceId::generate();
                let hlc = HybridLogicalClock {
                    physical_ms: now,
                    logical: 0,
                    device_id,
                };
                let entry = Entry {
                    id: EntryId::generate(),
                    kind: EntryKind::Capture,
                    body,
                    project: None,
                    tags: TagSet::default(),
                    created_at: hlc,
                    created_on_device: device_id,
                    last_modified_at: hlc,
                    last_modified_on_device: device_id,
                };
                Ok(PluginVerbResponse::Create { entry })
            },
        )
        .run();
}
