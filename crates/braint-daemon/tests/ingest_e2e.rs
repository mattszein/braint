mod common;

use braint_client::Client;
use braint_proto::{IngestRequest, JsonRpcRequest, METHOD_INGEST, Source};

#[tokio::test]
async fn ingest_creates_row_in_sqlite() {
    let handle = common::spawn_test_daemon().await;

    let mut client = Client::connect(handle.socket_path.to_str().unwrap())
        .await
        .unwrap();

    let req = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: 1,
        method: METHOD_INGEST.to_string(),
        params: IngestRequest {
            text: "explore CRDTs for sync".to_string(),
            source: Source::Cli,
        },
    };

    let resp: braint_proto::JsonRpcResponse<braint_proto::IngestResponse> =
        client.send(&req).await.unwrap();

    let entry_id = resp.result.unwrap().entry_id;

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
