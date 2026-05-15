//! TUI snapshot tests using ratatui's TestBackend.

use braint_cli::tui::App;
use ratatui::{Terminal, backend::TestBackend};

#[test]
fn initial_render_is_stable() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = App::new();
    terminal.draw(|f| app.render(f)).unwrap();
    let buffer = terminal.backend().buffer().clone();
    // Check header line is present
    let first_line: String = (0..80)
        .map(|x| buffer[(x, 0)].symbol().to_string())
        .collect();
    assert!(
        first_line.contains("braint"),
        "header should contain 'braint'"
    );
    // Check panel titles
    let rendered: String = buffer
        .content()
        .iter()
        .map(|c| c.symbol().to_string())
        .collect();
    assert!(rendered.contains("Scratch"), "should have Scratch panel");
    assert!(
        rendered.contains("Recent Activity"),
        "should have Recent Activity panel"
    );
}

#[test]
fn after_entries_shows_in_scratch() {
    use braint_proto::{DeviceId, Entry, EntryId, EntryKind, HybridLogicalClock, TagSet};
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = App::new();

    let device = DeviceId::generate();
    let hlc = HybridLogicalClock {
        physical_ms: 1000,
        logical: 0,
        device_id: device,
    };
    for i in 0..3 {
        let entry = Entry {
            id: EntryId::generate(),
            kind: EntryKind::Idea,
            body: format!("test entry {i}"),
            project: None,
            tags: TagSet::default(),
            created_at: hlc,
            created_on_device: device,
            last_modified_at: hlc,
            last_modified_on_device: device,
        };
        app.scratch.push(entry);
    }

    terminal.draw(|f| app.render(f)).unwrap();
    let buffer = terminal.backend().buffer().clone();
    let rendered: String = buffer
        .content()
        .iter()
        .map(|c| c.symbol().to_string())
        .collect();
    assert!(rendered.contains("test entry"), "should show entries");
    let title: String = (0..80)
        .map(|x| buffer[(x, 1)].symbol().to_string())
        .collect();
    assert!(title.contains("Scratch (3)"), "count should be 3");
}
