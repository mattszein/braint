//! TUI entry point — terminal lifecycle, panic hook, event loop.

mod app;
mod event_loop;
pub mod panels;

use braint_client::Client;
use crossterm::{
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::io;

pub use app::App;
pub use app::ScratchPanel;

pub async fn run(client: Client) -> crate::error::Result<()> {
    // Install panic hook FIRST so terminal is always restored
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        // Restore terminal before printing the panic
        let _ = disable_raw_mode();
        let _ = execute!(io::stderr(), LeaveAlternateScreen);
        original_hook(info);
    }));

    enable_raw_mode().map_err(crate::error::CliError::Io)?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen).map_err(crate::error::CliError::Io)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal =
        Terminal::new(backend).map_err(|e| crate::error::CliError::Daemon(e.to_string()))?;

    let result = event_loop::run(&mut terminal, client).await;

    // Always restore terminal
    let _ = disable_raw_mode();
    let _ = execute!(terminal.backend_mut(), LeaveAlternateScreen);
    terminal.show_cursor().ok();

    result
}
