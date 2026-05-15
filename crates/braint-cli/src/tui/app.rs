//! App state and rendering.

use braint_proto::{Entry, EntryChange};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Normal,
    Command,
    Help,
}

pub struct ScratchPanel {
    pub entries: Vec<Entry>,
    pub list_state: ListState,
}

impl Default for ScratchPanel {
    fn default() -> Self { Self::new() }
}

impl ScratchPanel {
    pub fn new() -> Self {
        Self { entries: Vec::new(), list_state: ListState::default() }
    }

    pub fn push(&mut self, entry: Entry) {
        self.entries.insert(0, entry); // newest first
    }

    pub fn on_change(&mut self, change: EntryChange, entry: Entry) {
        match change {
            EntryChange::Created => self.push(entry),
            EntryChange::Updated => {
                if let Some(pos) = self.entries.iter().position(|e| e.id == entry.id) {
                    self.entries[pos] = entry;
                }
            }
            EntryChange::Deleted => {
                self.entries.retain(|e| e.id != entry.id);
            }
        }
    }

    pub fn next(&mut self) {
        let len = self.entries.len();
        if len == 0 { return; }
        let i = self.list_state.selected().map(|i| (i + 1) % len).unwrap_or(0);
        self.list_state.select(Some(i));
    }

    pub fn prev(&mut self) {
        let len = self.entries.len();
        if len == 0 { return; }
        let i = self.list_state.selected()
            .map(|i| if i == 0 { len - 1 } else { i - 1 })
            .unwrap_or(0);
        self.list_state.select(Some(i));
    }
}

pub struct ActivityPanel {
    pub lines: Vec<String>, // newest first, max 200
}

impl ActivityPanel {
    pub fn new() -> Self { Self { lines: Vec::new() } }

    pub fn push(&mut self, entry: &Entry, change: EntryChange) {
        let verb = match entry.kind {
            braint_proto::EntryKind::Idea | braint_proto::EntryKind::Capture => "idea",
            braint_proto::EntryKind::Todo => "todo",
            braint_proto::EntryKind::Note => "note",
        };
        let action = match change {
            EntryChange::Created => "+",
            EntryChange::Updated => "~",
            EntryChange::Deleted => "-",
        };
        let preview = if entry.body.len() > 50 {
            format!("{}…", &entry.body[..50])
        } else {
            entry.body.clone()
        };
        self.lines.insert(0, format!("{action} [{verb}] {preview}"));
        self.lines.truncate(200);
    }
}

pub struct App {
    pub mode: Mode,
    pub scratch: ScratchPanel,
    pub activity: ActivityPanel,
    pub command: String,
    pub status: String,
}

impl Default for App {
    fn default() -> Self { Self::new() }
}

impl App {
    pub fn new() -> Self {
        Self {
            mode: Mode::Normal,
            scratch: ScratchPanel::new(),
            activity: ActivityPanel::new(),
            command: String::new(),
            status: String::new(),
        }
    }

    pub fn render(&mut self, f: &mut Frame) {
        let size = f.area();

        // Layout: header (1), main body (rest), command line (1), status (1)
        let outer = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // header
                Constraint::Min(0),    // body
                Constraint::Length(1), // command
                Constraint::Length(1), // status bar
            ])
            .split(size);

        // Header
        let header_text = " braint  [?] help  [q] quit  [j/k] nav  [:] command";
        let header = Paragraph::new(header_text)
            .style(Style::default().bg(Color::DarkGray).fg(Color::White));
        f.render_widget(header, outer[0]);

        // Body: two columns
        let body = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(outer[1]);

        self.render_scratch(f, body[0]);
        self.render_activity(f, body[1]);

        // Command line
        let cmd_text = match self.mode {
            Mode::Command => format!(":{}", self.command),
            Mode::Help => " [q/?/Esc] close help  [j/k] nav  [:] command mode".to_string(),
            Mode::Normal => String::new(),
        };
        let cmd_widget = Paragraph::new(cmd_text)
            .style(Style::default().fg(Color::Yellow));
        f.render_widget(cmd_widget, outer[2]);

        // Status bar
        let status = Paragraph::new(self.status.clone())
            .style(Style::default().fg(Color::Gray));
        f.render_widget(status, outer[3]);

        // Help overlay
        if self.mode == Mode::Help {
            self.render_help(f, size);
        }
    }

    fn render_scratch(&mut self, f: &mut Frame, area: Rect) {
        let title = format!(" Scratch ({}) ", self.scratch.entries.len());
        let block = Block::default().borders(Borders::ALL).title(title);

        let items: Vec<ListItem> = self.scratch.entries.iter().map(|e| {
            let preview = if e.body.len() > 60 { format!("{}…", &e.body[..60]) } else { e.body.clone() };
            let proj = e.project.as_ref().map(|p| format!("[{}] ", p)).unwrap_or_default();
            ListItem::new(Line::from(format!("• {proj}{preview}")))
        }).collect();

        let list = List::new(items)
            .block(block)
            .highlight_style(Style::default().bg(Color::Blue).add_modifier(Modifier::BOLD))
            .highlight_symbol("> ");

        f.render_stateful_widget(list, area, &mut self.scratch.list_state);
    }

    fn render_activity(&mut self, f: &mut Frame, area: Rect) {
        let block = Block::default().borders(Borders::ALL).title(" Recent Activity ");
        let items: Vec<ListItem> = self.activity.lines.iter()
            .map(|l| ListItem::new(Line::from(l.as_str())))
            .collect();
        let list = List::new(items).block(block);
        f.render_widget(list, area);
    }

    fn render_help(&self, f: &mut Frame, area: Rect) {
        use ratatui::widgets::Clear;
        let popup = centered_rect(60, 14, area);
        f.render_widget(Clear, popup);
        let help_text = vec![
            Line::from(vec![Span::styled(" Keybindings", Style::default().add_modifier(Modifier::BOLD))]),
            Line::from(""),
            Line::from("  j / ↓    navigate down"),
            Line::from("  k / ↑    navigate up"),
            Line::from("  :        enter command mode"),
            Line::from("  Esc      exit command mode / close help"),
            Line::from("  ?        toggle this help"),
            Line::from("  q        quit"),
            Line::from(""),
            Line::from("  Command mode (:)"),
            Line::from("  :idea <text>    capture an idea"),
            Line::from("  :todo <text>    create a task"),
            Line::from("  :note <text>    create a note"),
            Line::from("  Enter           send command"),
        ];
        let help = Paragraph::new(help_text)
            .block(Block::default().borders(Borders::ALL).title(" Help "))
            .style(Style::default().bg(Color::DarkGray));
        f.render_widget(help, popup);
    }
}

fn centered_rect(percent_x: u16, height: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(r.height.saturating_sub(height) / 2),
            Constraint::Length(height),
            Constraint::Min(0),
        ])
        .split(r);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
