use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "braint", about = "Personal daemon CLI")]
pub struct Cli {
    #[command(subcommand)]
    pub cmd: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Ingest a raw text string as an idea
    Ingest {
        /// The text to ingest
        text: String,
    },
}
