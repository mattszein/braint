//! Plugin lifecycle — spawn, read/write tasks, and manifest fetching.

use braint_proto::{JsonRpcResponse, plugin::PluginManifest};
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::process::{Child, Command};
use tokio::sync::{Mutex, mpsc, oneshot};

/// Read a 4-byte big-endian length prefix, then that many bytes.
async fn read_async_frame<R: AsyncReadExt + Unpin>(reader: &mut R) -> std::io::Result<Vec<u8>> {
    let mut len_buf = [0u8; 4];
    reader.read_exact(&mut len_buf).await?;
    let len = u32::from_be_bytes(len_buf) as usize;
    let mut buf = vec![0u8; len];
    reader.read_exact(&mut buf).await?;
    Ok(buf)
}

/// Write a 4-byte big-endian length prefix, then the data.
async fn write_async_frame<W: AsyncWriteExt + Unpin>(
    writer: &mut W,
    data: &[u8],
) -> std::io::Result<()> {
    let len = (data.len() as u32).to_be_bytes();
    writer.write_all(&len).await?;
    writer.write_all(data).await?;
    writer.flush().await?;
    Ok(())
}

/// Fetch and validate the plugin manifest by running `binary --manifest`.
///
/// Returns an error if the process exits non-zero or if the output is not valid JSON.
pub async fn fetch_manifest(
    binary: &PathBuf,
) -> Result<PluginManifest, crate::error::DaemonError> {
    let output = Command::new(binary).arg("--manifest").output().await?;
    if !output.status.success() {
        return Err(crate::error::DaemonError::PluginManifestError(format!(
            "{}: --manifest exited with {}",
            binary.display(),
            output.status
        )));
    }
    let manifest: PluginManifest = serde_json::from_slice(&output.stdout).map_err(|e| {
        crate::error::DaemonError::PluginManifestError(format!("{}: {e}", binary.display()))
    })?;
    Ok(manifest)
}

/// Spawn a plugin binary, wire up async reader/writer tasks, and return a [`PluginHandle`].
pub async fn spawn_plugin(
    binary: &PathBuf,
    manifest: PluginManifest,
) -> Result<super::handle::PluginHandle, crate::error::DaemonError> {
    let mut child: Child = Command::new(binary)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()?;

    let stdin = child.stdin.take().expect("stdin should be piped");
    let stdout = child.stdout.take().expect("stdout should be piped");
    let stderr = child.stderr.take().expect("stderr should be piped");

    let child = Arc::new(Mutex::new(child));
    let pending: Arc<Mutex<HashMap<i64, oneshot::Sender<JsonRpcResponse<Value>>>>> =
        Arc::new(Mutex::new(HashMap::new()));
    let (frame_tx, mut frame_rx) = mpsc::channel::<Vec<u8>>(256);

    // Writer task: take unframed payloads from channel, frame and write to plugin stdin.
    let mut writer = BufWriter::new(stdin);
    tokio::spawn(async move {
        while let Some(payload) = frame_rx.recv().await {
            if write_async_frame(&mut writer, &payload).await.is_err() {
                break;
            }
        }
    });

    // Reader task: read framed responses from plugin stdout, dispatch to pending channels.
    let pending_clone = pending.clone();
    let plugin_name = manifest.name.clone();
    tokio::spawn(async move {
        let mut reader = BufReader::new(stdout);
        loop {
            match read_async_frame(&mut reader).await {
                Ok(frame) => {
                    if let Ok(resp) =
                        serde_json::from_slice::<JsonRpcResponse<Value>>(&frame)
                    {
                        let id = resp.id;
                        if let Some(tx) = pending_clone.lock().await.remove(&id) {
                            let _ = tx.send(resp);
                        }
                    }
                }
                Err(_) => {
                    tracing::error!(plugin = %plugin_name, "plugin stdout closed");
                    break;
                }
            }
        }
    });

    // Stderr logger task: forward plugin stderr lines to tracing.
    let plugin_name2 = manifest.name.clone();
    tokio::spawn(async move {
        use tokio::io::AsyncBufReadExt;
        let mut lines = tokio::io::BufReader::new(stderr).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            tracing::info!(plugin = %plugin_name2, "{}", line);
        }
    });

    Ok(super::handle::PluginHandle {
        manifest,
        frame_tx,
        pending,
        child,
    })
}
