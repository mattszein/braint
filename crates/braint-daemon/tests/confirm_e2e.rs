mod common;

use braint_client::Client;
use braint_proto::{
    CancelRequest, CancelResponse, ConfirmRequest, ConfirmResponse, IngestRequest, IngestResponse,
    METHOD_CANCEL, METHOD_CONFIRM, METHOD_INGEST, Source,
};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Helper: ingest a Voice entry and return the PendingId.
// ---------------------------------------------------------------------------
async fn ingest_voice(client: &Client, text: &str) -> braint_proto::PendingId {
    let req = IngestRequest {
        text: text.to_string(),
        source: Source::Voice,
    };
    let resp: IngestResponse = client.send(METHOD_INGEST, &req).await.unwrap();
    match resp {
        IngestResponse::Pending { pending_id, .. } => pending_id,
        IngestResponse::Committed { .. } => panic!("expected Pending, got Committed"),
    }
}

// ---------------------------------------------------------------------------
// 1. voice_ingest_then_confirm_creates_row
// ---------------------------------------------------------------------------
#[tokio::test]
async fn voice_ingest_then_confirm_creates_row() {
    let handle = common::spawn_test_daemon().await;
    let socket = handle.socket_path.to_str().unwrap();

    let client = Client::connect(socket).await.unwrap();
    let pending_id = ingest_voice(&client, "idea test confirmation flow").await;

    let confirm_req = ConfirmRequest { pending_id };
    let confirm_resp: ConfirmResponse =
        client.send(METHOD_CONFIRM, &confirm_req).await.unwrap();

    // The entry must now be present in SQLite.
    let count = common::query_count(&handle.db_path, confirm_resp.entry_id);
    assert_eq!(count, 1, "confirmed entry must be persisted in SQLite");
}

// ---------------------------------------------------------------------------
// 2. voice_ingest_then_cancel_no_row
// ---------------------------------------------------------------------------
#[tokio::test]
async fn voice_ingest_then_cancel_no_row() {
    let handle = common::spawn_test_daemon().await;
    let socket = handle.socket_path.to_str().unwrap();

    let client = Client::connect(socket).await.unwrap();

    // Capture the preview entry id so we can look it up later.
    let req = IngestRequest {
        text: "idea test cancel flow".to_string(),
        source: Source::Voice,
    };
    let resp: IngestResponse = client.send(METHOD_INGEST, &req).await.unwrap();
    let (pending_id, preview_entry_id) = match resp {
        IngestResponse::Pending { pending_id, preview } => (pending_id, preview.id),
        IngestResponse::Committed { .. } => panic!("expected Pending"),
    };

    let cancel_req = CancelRequest { pending_id };
    let _: CancelResponse = client.send(METHOD_CANCEL, &cancel_req).await.unwrap();

    // The entry must NOT be in SQLite.
    let count = common::query_count(&handle.db_path, preview_entry_id);
    assert_eq!(count, 0, "cancelled entry must not be persisted in SQLite");
}

// ---------------------------------------------------------------------------
// 3. double_cancel_is_ok (idempotent)
// ---------------------------------------------------------------------------
#[tokio::test]
async fn double_cancel_is_ok() {
    let handle = common::spawn_test_daemon().await;
    let socket = handle.socket_path.to_str().unwrap();

    let client = Client::connect(socket).await.unwrap();
    let pending_id = ingest_voice(&client, "idea test double cancel").await;

    let cancel_req = CancelRequest { pending_id };

    // First cancel — should succeed.
    let _: CancelResponse = client
        .send(METHOD_CANCEL, &cancel_req)
        .await
        .unwrap();

    // Second cancel — must also succeed (idempotent).
    let _: CancelResponse = client
        .send(METHOD_CANCEL, &cancel_req)
        .await
        .unwrap();
}

// ---------------------------------------------------------------------------
// 4. confirm_unknown_id_returns_error
// ---------------------------------------------------------------------------
#[tokio::test]
async fn confirm_unknown_id_returns_error() {
    let handle = common::spawn_test_daemon().await;
    let socket = handle.socket_path.to_str().unwrap();

    let client = Client::connect(socket).await.unwrap();

    // Use a random PendingId that was never registered.
    let fake_pending_id = braint_proto::PendingId(Uuid::now_v7());
    let confirm_req = ConfirmRequest { pending_id: fake_pending_id };

    let result: Result<ConfirmResponse, _> = client.send(METHOD_CONFIRM, &confirm_req).await;

    assert!(result.is_err(), "confirming an unknown id must return an error");

    let err_msg = result.unwrap_err().to_string();
    // The server returns ERR_NOT_FOUND (-32002) with "pending entry not found".
    assert!(
        err_msg.contains("not found") || err_msg.contains("-32002"),
        "error message should indicate not-found: {err_msg}"
    );
}
