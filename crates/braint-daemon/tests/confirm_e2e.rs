// TODO(phase-4+): pending/confirm/cancel flow tests.
// Voice ingest commits directly in Phase 3 — no pending state is created.
// Re-enable these tests when the pending flow is reintroduced.
mod common;

use braint_client::Client;
use braint_proto::{ConfirmRequest, ConfirmResponse, METHOD_CONFIRM};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// 1–3: voice_ingest_then_confirm, voice_ingest_then_cancel, double_cancel
// Skipped: voice no longer creates pending entries (Phase 3).
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// 4. confirm_unknown_id_returns_error
// This test does not depend on voice→pending and stays active.
// ---------------------------------------------------------------------------
#[tokio::test]
async fn confirm_unknown_id_returns_error() {
    let handle = common::spawn_test_daemon().await;
    let socket = handle.socket_path.to_str().unwrap();

    let client = Client::connect(socket).await.unwrap();

    let fake_pending_id = braint_proto::PendingId(Uuid::now_v7());
    let confirm_req = ConfirmRequest {
        pending_id: fake_pending_id,
    };

    let result: Result<ConfirmResponse, _> = client.send(METHOD_CONFIRM, &confirm_req).await;

    assert!(
        result.is_err(),
        "confirming an unknown id must return an error"
    );

    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("not found") || err_msg.contains("-32002"),
        "error message should indicate not-found: {err_msg}"
    );
}
