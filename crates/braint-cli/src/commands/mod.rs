pub mod cancel;
pub mod confirm;
pub mod ingest;

use crate::args::{Cli, Command};
use crate::output::OutputMode;
use braint_proto::Source;

pub async fn dispatch(cli: &Cli) -> crate::error::Result<()> {
    let mode = OutputMode::from_flag(cli.json);
    let source: Source = cli.source.map(Into::into).unwrap_or(Source::Cli);
    let socket = socket_path(cli);

    match &cli.cmd {
        Some(Command::Ingest { text }) => ingest::run("ingest", text, source, &socket, &mode).await,
        Some(Command::Idea { text }) => ingest::run("idea", text, source, &socket, &mode).await,
        Some(Command::Todo { text }) => ingest::run("todo", text, source, &socket, &mode).await,
        Some(Command::Note { text }) => ingest::run("note", text, source, &socket, &mode).await,
        Some(Command::Capture { text }) => ingest::run("capture", text, source, &socket, &mode).await,
        Some(Command::Confirm { pending_id }) => confirm::run(pending_id, &socket, &mode).await,
        Some(Command::Cancel { pending_id }) => cancel::run(pending_id, &socket, &mode).await,
        None => {
            // No subcommand: check if stdin is a TTY
            if is_terminal::IsTerminal::is_terminal(&std::io::stdin()) {
                // TTY → launch TUI
                let client = braint_client::Client::connect(&socket)
                    .await
                    .map_err(|e| crate::error::CliError::Daemon(e.to_string()))?;
                crate::tui::run(client).await
            } else {
                // piped stdin: read and ingest as idea
                use std::io::Read;
                let mut text = String::new();
                std::io::stdin()
                    .read_to_string(&mut text)
                    .map_err(crate::error::CliError::Io)?;
                ingest::run("idea", text.trim(), source, &socket, &mode).await
            }
        }
    }
}

fn socket_path(cli: &Cli) -> String {
    cli.socket.clone().unwrap_or_else(|| {
        let runtime_dir = std::env::var_os("XDG_RUNTIME_DIR")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(std::env::temp_dir);
        runtime_dir
            .join("braint.sock")
            .to_string_lossy()
            .to_string()
    })
}
