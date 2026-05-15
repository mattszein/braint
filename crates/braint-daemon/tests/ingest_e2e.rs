mod common;

use braint_client::Client;
use braint_proto::{IngestRequest, IngestResponse, METHOD_INGEST, Source};

#[tokio::test]
async fn ingest_creates_row_in_sqlite() {
    let handle = common::spawn_test_daemon().await;

    let client = Client::connect(handle.socket_path.to_str().unwrap())
        .await
        .unwrap();

    let req = IngestRequest {
        // parse_verb requires a verb prefix: "idea" maps to EntryKind::Idea
        text: "idea explore CRDTs for sync".to_string(),
        source: Source::Cli,
    };

    let resp: IngestResponse = client.send(METHOD_INGEST, &req).await.unwrap();

    let entry_id = match resp {
        IngestResponse::Committed { entry_id } => entry_id,
        IngestResponse::Pending { .. } => panic!("expected Committed, got Pending"),
    };

    // Assert row exists in SQLite
    let conn = rusqlite::Connection::open(&handle.db_path).unwrap();
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM entries WHERE id = ?1",
            [entry_id.0.as_bytes()],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 1);
}
