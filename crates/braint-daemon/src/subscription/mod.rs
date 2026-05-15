//! Subscription manager — server-side pub/sub for entry change notifications.
//!
//! Clients subscribe with an [`EntryFilter`] and receive [`JsonRpcNotification`]s
//! whenever a matching entry is created, updated, or deleted.

pub mod filter;

use braint_proto::{
    EntryChange, EntryChangeNotification, EntryFilter, JsonRpcNotification,
    METHOD_NOTIFY_ENTRY_CHANGED, SubscriptionId,
};
use std::collections::HashMap;
use tokio::sync::mpsc;
use uuid::Uuid;

/// Sender type for outbound entry-change notifications.
pub type NotifySender = mpsc::Sender<JsonRpcNotification<EntryChangeNotification>>;
/// Receiver type for outbound entry-change notifications.
pub type NotifyReceiver = mpsc::Receiver<JsonRpcNotification<EntryChangeNotification>>;

/// Opaque identifier for a single socket connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ConnectionId(pub Uuid);

impl ConnectionId {
    /// Generate a new unique connection id.
    pub fn generate() -> Self {
        Self(Uuid::now_v7())
    }
}

/// Internal record for one subscription.
struct SubscriptionEntry {
    filter: EntryFilter,
    sender: NotifySender,
    connection_id: ConnectionId,
}

/// Manages all active subscriptions across all connections.
///
/// Cheaply cloneable — the inner state is wrapped in an `Arc`.
#[derive(Default)]
pub struct SubscriptionManager {
    subs: tokio::sync::RwLock<HashMap<SubscriptionId, SubscriptionEntry>>,
}

impl SubscriptionManager {
    /// Create a new, empty `SubscriptionManager`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a new subscription. Returns `(subscription_id, receiver)`.
    pub async fn subscribe(
        &self,
        filter: EntryFilter,
        connection_id: ConnectionId,
    ) -> (SubscriptionId, NotifyReceiver) {
        let id = SubscriptionId::generate();
        let (tx, rx) = mpsc::channel(1024);
        let mut subs = self.subs.write().await;
        subs.insert(
            id,
            SubscriptionEntry {
                filter,
                sender: tx,
                connection_id,
            },
        );
        (id, rx)
    }

    /// Remove a single subscription by id.
    pub async fn unsubscribe(&self, id: SubscriptionId) {
        self.subs.write().await.remove(&id);
    }

    /// Remove all subscriptions belonging to the given connection.
    ///
    /// Called when a connection closes to prevent memory leaks.
    pub async fn unregister_connection(&self, connection_id: ConnectionId) {
        self.subs
            .write()
            .await
            .retain(|_, e| e.connection_id != connection_id);
    }

    /// Push a notification to all subscribers whose filter matches `entry`.
    ///
    /// Slow or dead subscriber channels are silently skipped — they will be cleaned up
    /// when the connection closes and calls [`unregister_connection`](Self::unregister_connection).
    pub async fn publish(&self, change: EntryChange, entry: &braint_proto::Entry) {
        let subs = self.subs.read().await;
        for (sub_id, sub) in subs.iter() {
            if filter::filter_matches(&sub.filter, entry) {
                let notif = JsonRpcNotification::new(
                    METHOD_NOTIFY_ENTRY_CHANGED,
                    EntryChangeNotification {
                        subscription_id: *sub_id,
                        change,
                        entry: entry.clone(),
                    },
                );
                // Ignore send errors — slow/dead clients are cleaned up on connection close.
                let _ = sub.sender.try_send(notif);
            }
        }
    }
}
