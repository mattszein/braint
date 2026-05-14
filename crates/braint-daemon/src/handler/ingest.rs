use crate::storage::Storage;
use braint_core::{Clock, parse_ingest};
use braint_proto::{DeviceId, IngestRequest, IngestResponse, JsonRpcError};

pub struct IngestHandler {
    storage: Storage,
    clock: Clock,
    device_id: DeviceId,
}

impl IngestHandler {
    pub fn new(storage: Storage, clock: Clock, device_id: DeviceId) -> Self {
        Self {
            storage,
            clock,
            device_id,
        }
    }

    pub fn handle(&mut self, req: IngestRequest) -> Result<IngestResponse, JsonRpcError> {
        let hlc = self.clock.now();
        let entry = parse_ingest(&req.text, self.device_id, hlc)
            .map_err(|e| JsonRpcError::new(-32000, format!("parse error: {e}")))?;
        let id = entry.id;
        self.storage
            .save(&entry)
            .map_err(|e| JsonRpcError::new(-32001, format!("storage error: {e}")))?;
        Ok(IngestResponse { entry_id: id })
    }
}
