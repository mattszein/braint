use braint_proto::{DeviceId, HybridLogicalClock};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

/// Clock state for this daemon instance.
pub struct Clock {
    device_id: DeviceId,
    logical: AtomicU32,
}

impl Clock {
    pub fn new(device_id: DeviceId) -> Self {
        Self {
            device_id,
            logical: AtomicU32::new(0),
        }
    }

    pub fn now(&self) -> HybridLogicalClock {
        let physical_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock before 1970")
            .as_millis() as u64;
        let logical = self.logical.fetch_add(1, Ordering::SeqCst);
        HybridLogicalClock {
            physical_ms,
            logical,
            device_id: self.device_id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clock_increments_logical() {
        let device = DeviceId::generate();
        let clock = Clock::new(device);
        let a = clock.now();
        let b = clock.now();
        assert_eq!(b.logical, a.logical + 1);
        assert!(b.physical_ms >= a.physical_ms);
    }
}
