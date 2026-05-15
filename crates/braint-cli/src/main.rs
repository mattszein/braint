mod args;
mod commands;
pub mod error;
pub mod output;
pub mod tui;

#[tokio::main]
async fn main() {
    // Reset SIGPIPE to default so piping output (e.g. `braint idea x | head`) doesn't panic.
    #[cfg(unix)]
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }
    use clap::Parser;
    let cli = args::Cli::parse();

    if let Err(e) = commands::dispatch(&cli).await {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}
