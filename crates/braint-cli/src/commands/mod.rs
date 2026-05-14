pub mod ingest;

use crate::args::Command;

pub async fn dispatch(cmd: Command) -> crate::error::Result<()> {
    match cmd {
        Command::Ingest { text } => ingest::run(text).await,
    }
}
