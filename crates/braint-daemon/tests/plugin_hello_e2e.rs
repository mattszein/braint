//! End-to-end test for the `hello` plugin.

mod common;

use braint_proto::{EntryKind, IngestRequest, IngestResponse, METHOD_INGEST, Source};
use std::path::PathBuf;

/// Create a temporary directory containing only the hello plugin binary (symlinked).
/// This avoids scanning the full `target/debug/` directory which contains many non-plugin binaries.
fn make_plugin_dir() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let src = common::hello_plugin_path();
    let dst = dir.path().join("braint-plugin-hello");
    std::fs::copy(&src, &dst).expect("failed to copy hello plugin binary");
    // Ensure the copy is executable.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&dst).unwrap().permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&dst, perms).unwrap();
    }
    let plugin_dir = dir.path().to_path_buf();
    (dir, plugin_dir)
}

/// The hello plugin should handle the `hello` verb and commit a Capture entry.
#[tokio::test]
async fn hello_plugin_routes_and_responds() {
    let (_dir, plugin_dir) = make_plugin_dir();
    let handle = common::spawn_test_daemon_with_plugins(vec![plugin_dir]).await;

    let req = IngestRequest {
        text: "hello world".to_string(),
        source: Source::Cli,
    };

    let resp: IngestResponse = handle.client.send(METHOD_INGEST, &req).await.unwrap();

    match resp {
        IngestResponse::Committed { kind, body, .. } => {
            assert_eq!(kind, EntryKind::Capture);
            assert_eq!(body, "hello world");
        }
        IngestResponse::Pending { .. } => panic!("expected Committed, got Pending"),
    }
}

/// A bare `hello` (no argument) should produce `body = "hello"`.
#[tokio::test]
async fn hello_plugin_empty_body() {
    let (_dir, plugin_dir) = make_plugin_dir();
    let handle = common::spawn_test_daemon_with_plugins(vec![plugin_dir]).await;

    let req = IngestRequest {
        text: "hello".to_string(),
        source: Source::Cli,
    };

    let resp: IngestResponse = handle.client.send(METHOD_INGEST, &req).await.unwrap();

    match resp {
        IngestResponse::Committed { body, .. } => {
            assert_eq!(body, "hello");
        }
        IngestResponse::Pending { .. } => panic!("expected Committed, got Pending"),
    }
}
