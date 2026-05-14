use clap::Parser;

mod args;
mod commands;
mod error;
mod output;

#[tokio::main]
async fn main() {
    let cli = args::Cli::parse();

    if let Err(e) = commands::dispatch(cli.cmd).await {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}
