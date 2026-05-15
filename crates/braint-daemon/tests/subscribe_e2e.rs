mod common;

use braint_client::Client;
use braint_proto::{
    EntryChange, EntryChangeNotification, EntryFilter, EntryKind, IngestRequest, IngestResponse,
    JsonRpcNotification, METHOD_INGEST, METHOD_UNSUBSCRIBE, Source, SubscribeRequest,
    SubscriptionId, SubscriptionTopic, UnsubscribeRequest, UnsubscribeResponse,
};
use tokio::sync::mpsc;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Call `client.subscribe` and return the notification receiver.
async fn subscribe(
    client: &Client,
    topic: SubscriptionTopic,
    filter: EntryFilter,
) -> (SubscriptionId, mpsc::Receiver<Vec<u8>>) {
    let req = SubscribeRequest { topic, filter };
    client.subscribe(&req).await.unwrap()
}

/// Wait up to `timeout_ms` milliseconds for a notification on `rx`.
/// Returns `Some(bytes)` if one arrived, `None` if the deadline elapsed.
async fn recv_timeout(rx: &mut mpsc::Receiver<Vec<u8>>, timeout_ms: u64) -> Option<Vec<u8>> {
    tokio::time::timeout(std::time::Duration::from_millis(timeout_ms), rx.recv())
        .await
        .ok() // Err(_) means timed-out → None
        .flatten()
}

/// Ingest a Cli entry and return the committed EntryId.
async fn ingest_cli(client: &Client, text: &str) -> braint_proto::EntryId {
    let req = IngestRequest {
        text: text.to_string(),
        source: Source::Cli,
    };
    let resp: IngestResponse = client.send(METHOD_INGEST, &req).await.unwrap();
    match resp {
        IngestResponse::Committed { entry_id, .. } => entry_id,
        IngestResponse::Pending { .. } => panic!("expected Committed, got Pending"),
    }
}

// ---------------------------------------------------------------------------
// 1. subscribe_receives_notification_on_ingest
// ---------------------------------------------------------------------------
#[tokio::test]
async fn subscribe_receives_notification_on_ingest() {
    let handle = common::spawn_test_daemon().await;
    let socket = handle.socket_path.to_str().unwrap();

    // Client A: subscribe with no filter.
    let client_a = Client::connect(socket).await.unwrap();
    let (_sub_id, mut notif_rx) = subscribe(
        &client_a,
        SubscriptionTopic::Scratch,
        EntryFilter::default(),
    )
    .await;

    // Brief pause to ensure the subscription is fully registered before ingesting.
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    // Client B: ingest an idea.
    let client_b = Client::connect(socket).await.unwrap();
    let entry_id = ingest_cli(&client_b, "idea test notification").await;

    // Client A: must receive exactly one notification within 500 ms.
    let notif_bytes = recv_timeout(&mut notif_rx, 500)
        .await
        .expect("timed out waiting for notification");

    let notif: JsonRpcNotification<EntryChangeNotification> =
        serde_json::from_slice(&notif_bytes).unwrap();

    assert_eq!(notif.params.entry.id, entry_id);
    assert_eq!(notif.params.change, EntryChange::Created);
}

// ---------------------------------------------------------------------------
// 2. subscribe_filter_excludes_non_matching
// ---------------------------------------------------------------------------
#[tokio::test]
async fn subscribe_filter_excludes_non_matching() {
    let handle = common::spawn_test_daemon().await;
    let socket = handle.socket_path.to_str().unwrap();

    // Client A subscribes only for Todo entries.
    let client_a = Client::connect(socket).await.unwrap();
    let filter = EntryFilter {
        kind: Some(EntryKind::Todo),
        ..EntryFilter::default()
    };
    let (_sub_id, mut notif_rx) = subscribe(&client_a, SubscriptionTopic::Scratch, filter).await;

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    // Client B ingests an Idea (not a Todo) — should NOT match the filter.
    let client_b = Client::connect(socket).await.unwrap();
    ingest_cli(&client_b, "idea explore subscription filtering").await;

    // Wait 100 ms; client A must NOT receive anything.
    let result = recv_timeout(&mut notif_rx, 100).await;
    assert!(
        result.is_none(),
        "subscriber with Todo filter must not receive an Idea notification"
    );
}

// ---------------------------------------------------------------------------
// 3. multi_client_concurrent_ingest
// ---------------------------------------------------------------------------
#[tokio::test]
async fn multi_client_concurrent_ingest() {
    let handle = common::spawn_test_daemon().await;
    let socket = handle.socket_path.to_str().unwrap();

    let client_1 = Client::connect(socket).await.unwrap();
    let client_2 = Client::connect(socket).await.unwrap();

    let (id1, id2) = tokio::join!(
        ingest_cli(&client_1, "idea concurrent ingest client one"),
        ingest_cli(&client_2, "idea concurrent ingest client two"),
    );

    assert_eq!(
        common::query_count(&handle.db_path, id1),
        1,
        "entry from client 1 must be persisted"
    );
    assert_eq!(
        common::query_count(&handle.db_path, id2),
        1,
        "entry from client 2 must be persisted"
    );
}

// ---------------------------------------------------------------------------
// 4. unsubscribe_stops_notifications
// ---------------------------------------------------------------------------
#[tokio::test]
async fn unsubscribe_stops_notifications() {
    let handle = common::spawn_test_daemon().await;
    let socket = handle.socket_path.to_str().unwrap();

    // Client A: subscribe.
    let client_a = Client::connect(socket).await.unwrap();
    let (sub_id, mut notif_rx) = subscribe(
        &client_a,
        SubscriptionTopic::Scratch,
        EntryFilter::default(),
    )
    .await;

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    // Client B: first ingest — client A should receive this.
    let client_b = Client::connect(socket).await.unwrap();
    ingest_cli(&client_b, "idea first ingest before unsubscribe").await;

    let first = recv_timeout(&mut notif_rx, 500).await;
    assert!(
        first.is_some(),
        "client A must receive the first notification"
    );

    // Client A: unsubscribe.
    let unsub_req = UnsubscribeRequest {
        subscription_id: sub_id,
    };
    let _: UnsubscribeResponse = client_a.send(METHOD_UNSUBSCRIBE, &unsub_req).await.unwrap();

    // Give the unsubscribe a moment to propagate on the server.
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    // Client B: second ingest — client A must NOT receive this.
    ingest_cli(&client_b, "idea second ingest after unsubscribe").await;

    let second = recv_timeout(&mut notif_rx, 100).await;
    assert!(
        second.is_none(),
        "client A must not receive notifications after unsubscribing"
    );
}
