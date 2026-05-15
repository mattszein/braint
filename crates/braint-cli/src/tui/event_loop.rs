//! Event loop: multiplexes crossterm keyboard events and subscription notifications.

use braint_client::Client;
use braint_proto::{
    EntryChangeNotification, EntryFilter, IngestRequest, JsonRpcNotification,
    ListRequest, ListResponse, Source, SubscribeRequest, SubscriptionTopic,
    METHOD_INGEST, METHOD_LIST,
};
use braint_core::parse_verb;
use crossterm::event::{Event, EventStream, KeyCode, KeyModifiers};
use futures::StreamExt;
use ratatui::{backend::Backend, Terminal};
use super::app::{App, Mode};

/// Milliseconds since epoch for midnight UTC today.
fn today_start_ms() -> u64 {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    // Round down to midnight UTC
    (now - now % 86400) * 1000
}

pub async fn run<B: Backend>(
    terminal: &mut Terminal<B>,
    mut client: Client,
) -> crate::error::Result<()> {
    let mut app = App::new();

    // Single subscription for all entries — one handler updates BOTH panels.
    // Avoids duplicates that occur when two overlapping subscriptions both fire
    // for the same entry and each independently push to the activity panel.
    let (_sub_id, mut events_rx) = client
        .subscribe(&SubscribeRequest {
            topic: SubscriptionTopic::Scratch,
            filter: EntryFilter::default(),
        })
        .await
        .map_err(|e| crate::error::CliError::Daemon(e.to_string()))?;

    // Initial load: all entries for scratch (newest at top).
    let all_req = ListRequest { filter: EntryFilter::default(), limit: Some(200) };
    if let Ok(resp) = client.send::<ListRequest, ListResponse>(METHOD_LIST, &all_req).await {
        for entry in resp.entries.into_iter().rev() {
            app.scratch.push(entry);
        }
    }

    // Initial load: today's entries for activity panel (oldest → newest order for push).
    let today_req = ListRequest {
        filter: EntryFilter { since_ms: Some(today_start_ms()), ..Default::default() },
        limit: Some(200),
    };
    if let Ok(resp) = client.send::<ListRequest, ListResponse>(METHOD_LIST, &today_req).await {
        // entries from storage are newest-first; reverse so activity shows newest at top
        for entry in resp.entries.into_iter().rev() {
            app.activity.push(&entry, braint_proto::EntryChange::Created);
        }
    }

    // Initial draw
    terminal.draw(|f| app.render(f)).map_err(|e| crate::error::CliError::Daemon(e.to_string()))?;

    let mut event_stream = EventStream::new();

    loop {
        tokio::select! {
            Some(event_result) = event_stream.next() => {
                match event_result {
                    Ok(Event::Key(key)) => {
                        if handle_key(&mut app, key, &mut client).await? {
                            break;
                        }
                    }
                    Err(_) => break,
                    _ => {}
                }
                terminal.draw(|f| app.render(f)).map_err(|e| crate::error::CliError::Daemon(e.to_string()))?;
            }
            Some(bytes) = events_rx.recv() => {
                if let Ok(notif) = serde_json::from_slice::<JsonRpcNotification<EntryChangeNotification>>(&bytes) {
                    // One subscription → update both panels exactly once per event.
                    app.scratch.on_change(notif.params.change, notif.params.entry.clone());
                    app.activity.push(&notif.params.entry, notif.params.change);
                }
                terminal.draw(|f| app.render(f)).map_err(|e| crate::error::CliError::Daemon(e.to_string()))?;
            }
        }
    }

    Ok(())
}

/// Returns true if the app should quit.
async fn handle_key(
    app: &mut App,
    key: crossterm::event::KeyEvent,
    client: &mut Client,
) -> crate::error::Result<bool> {
    match app.mode {
        Mode::Normal => match key.code {
            KeyCode::Char('q') => return Ok(true),
            KeyCode::Char('?') => app.mode = Mode::Help,
            KeyCode::Char(':') => {
                app.mode = Mode::Command;
                app.command.clear();
            }
            KeyCode::Char('j') | KeyCode::Down => app.scratch.next(),
            KeyCode::Char('k') | KeyCode::Up => app.scratch.prev(),
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                return Ok(true);
            }
            _ => {}
        },
        Mode::Help => match key.code {
            KeyCode::Char('q') | KeyCode::Char('?') | KeyCode::Esc => {
                app.mode = Mode::Normal;
            }
            _ => {}
        },
        Mode::Command => match key.code {
            KeyCode::Esc => {
                app.mode = Mode::Normal;
                app.command.clear();
            }
            KeyCode::Enter => {
                let cmd = app.command.trim().to_string();
                app.command.clear();
                app.mode = Mode::Normal;
                if !cmd.is_empty() {
                    run_command(app, client, &cmd).await;
                }
            }
            KeyCode::Backspace => { app.command.pop(); }
            KeyCode::Char(c) => { app.command.push(c); }
            _ => {}
        },
    }
    Ok(false)
}

async fn run_command(app: &mut App, client: &mut Client, cmd: &str) {
    // Parse the command via parse_verb (client-side syntax check)
    match parse_verb(cmd) {
        Err(e) => {
            app.status = format!("error: {e}");
        }
        Ok(_invocation) => {
            let req = IngestRequest { text: cmd.to_string(), source: Source::Cli };
            match client.send::<IngestRequest, braint_proto::IngestResponse>(METHOD_INGEST, &req).await {
                Ok(braint_proto::IngestResponse::Committed { entry_id }) => {
                    app.status = format!("captured: {entry_id}");
                }
                Ok(braint_proto::IngestResponse::Pending { pending_id, .. }) => {
                    app.status = format!("pending: {pending_id}");
                }
                Err(e) => {
                    app.status = format!("error: {e}");
                }
            }
        }
    }
}
