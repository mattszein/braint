use clap::{Parser, Subcommand, ValueEnum};

/// Source of the ingest request.
#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum SourceArg {
    Cli,
    Voice,
}

impl From<SourceArg> for braint_proto::Source {
    fn from(s: SourceArg) -> Self {
        match s {
            SourceArg::Cli => Self::Cli,
            SourceArg::Voice => Self::Voice,
        }
    }
}

#[derive(Parser)]
#[command(name = "braint", about = "Personal daemon CLI")]
pub struct Cli {
    #[command(subcommand)]
    pub cmd: Option<Command>,

    /// Output as NDJSON
    #[arg(long, global = true)]
    pub json: bool,

    /// Override the source tag (default: cli)
    #[arg(long, global = true)]
    pub source: Option<SourceArg>,

    /// Override the Unix socket path
    #[arg(long, global = true)]
    pub socket: Option<String>,
}

#[derive(Subcommand)]
pub enum Command {
    /// Ingest raw text as an idea (default verb)
    Ingest {
        /// The text to ingest
        text: String,
    },
    /// Capture an idea
    Idea {
        /// The text to ingest
        text: String,
    },
    /// Create a task
    Todo {
        /// The text to ingest
        text: String,
    },
    /// Create a note
    Note {
        /// The text to ingest
        text: String,
    },
    /// Capture text (alias for idea)
    Capture {
        /// The text to ingest
        text: String,
    },
    /// Confirm a pending voice entry
    Confirm {
        /// The pending entry UUID
        pending_id: String,
    },
    /// Cancel a pending voice entry
    Cancel {
        /// The pending entry UUID
        pending_id: String,
    },
}
