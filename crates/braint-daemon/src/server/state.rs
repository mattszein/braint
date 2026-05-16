//! Shared daemon state — cloned cheaply per-connection (all heavy fields are `Arc`).

use crate::{
    config::DaemonConfig, pending::PendingMap, plugin::PluginManager, storage::Storage,
    subscription::SubscriptionManager,
};
use braint_core::Clock;
use braint_proto::DeviceId;
use std::sync::Arc;
use tokio::sync::Mutex;

/// All mutable state shared between connection handler tasks.
///
/// `Clone` is O(1) — all fields are `Arc`-wrapped.
#[derive(Clone)]
pub struct DaemonState {
    /// SQLite storage, guarded for concurrent access.
    pub storage: Arc<Mutex<Storage>>,
    /// Hybrid logical clock.
    pub clock: Arc<Clock>,
    /// Stable identifier for this daemon instance.
    pub device_id: DeviceId,
    /// Pending voice confirmations awaiting user action.
    pub pending: Arc<Mutex<PendingMap>>,
    /// Active subscriptions for entry-change notifications.
    pub subs: Arc<SubscriptionManager>,
    /// Runtime configuration (paths, TTLs, limits).
    pub config: Arc<DaemonConfig>,
    /// Loaded plugin manager.
    pub plugins: Arc<PluginManager>,
}

impl DaemonState {
    /// Construct a new `DaemonState`, initialising all sub-components.
    pub fn new(
        storage: Storage,
        clock: Clock,
        device_id: DeviceId,
        config: DaemonConfig,
        plugins: PluginManager,
    ) -> Self {
        let ttl = config.pending_ttl_secs;
        Self {
            storage: Arc::new(Mutex::new(storage)),
            clock: Arc::new(clock),
            device_id,
            pending: Arc::new(Mutex::new(PendingMap::new(ttl))),
            subs: Arc::new(SubscriptionManager::new()),
            config: Arc::new(config),
            plugins: Arc::new(plugins),
        }
    }
}
