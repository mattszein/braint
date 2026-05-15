//! Integration tests for the Phase 2 verb grammar parser.

use braint_core::{parse_verb, VerbInvocation};
use braint_proto::{EntryKind, PrincipalTag, ProjectId, TagSet};

struct Case {
    input: &'static str,
    kind: EntryKind,
    body: &'static str,
    project: Option<&'static str>,
    principal: Vec<PrincipalTag>,
    free: Vec<&'static str>,
}

fn check(case: Case) {
    let result = parse_verb(case.input).unwrap_or_else(|e| {
        panic!("parse_verb({:?}) failed: {}", case.input, e);
    });

    assert_eq!(
        result.kind, case.kind,
        "kind mismatch for {:?}",
        case.input
    );
    assert_eq!(
        result.body, case.body,
        "body mismatch for {:?}",
        case.input
    );
    assert_eq!(
        result.project,
        case.project.map(|p| ProjectId(p.to_string())),
        "project mismatch for {:?}",
        case.input
    );
    assert_eq!(
        result.tags.principal, case.principal,
        "principal tags mismatch for {:?}",
        case.input
    );
    let expected_free: Vec<String> = case.free.iter().map(|s| s.to_string()).collect();
    assert_eq!(
        result.tags.free, expected_free,
        "free tags mismatch for {:?}",
        case.input
    );
}

// Case 1
#[test]
fn idea_no_separator_body_from_tokens() {
    check(Case {
        input: "idea explore CRDTs",
        kind: EntryKind::Idea,
        body: "explore CRDTs",
        project: None,
        principal: vec![],
        free: vec![],
    });
}

// Case 2
#[test]
fn idea_for_project_with_separator() {
    check(Case {
        input: "idea for pro-rails \u{2014} explore CRDTs",
        kind: EntryKind::Idea,
        body: "explore CRDTs",
        project: Some("pro-rails"),
        principal: vec![],
        free: vec![],
    });
}

// Case 3
#[test]
fn todo_priority_when_no_separator() {
    check(Case {
        input: "todo finish parser priority:high when:today",
        kind: EntryKind::Todo,
        body: "finish parser",
        project: None,
        principal: vec![
            PrincipalTag::Priority("high".to_string()),
            PrincipalTag::When("today".to_string()),
        ],
        free: vec![],
    });
}

// Case 4
#[test]
fn todo_for_project_with_separator_and_priority() {
    check(Case {
        input: "todo for my-project \u{2014} finish auth refactor priority:high",
        kind: EntryKind::Todo,
        body: "finish auth refactor priority:high",
        project: Some("my-project"),
        principal: vec![],
        free: vec![],
    });
}

// Case 5
#[test]
fn note_for_project_with_separator() {
    check(Case {
        input: "note for pro-rails \u{2014} thoughts on architecture",
        kind: EntryKind::Note,
        body: "thoughts on architecture",
        project: Some("pro-rails"),
        principal: vec![],
        free: vec![],
    });
}

// Case 6
#[test]
fn capture_no_separator() {
    check(Case {
        input: "capture try cr-sqlite",
        kind: EntryKind::Capture,
        body: "try cr-sqlite",
        project: None,
        principal: vec![],
        free: vec![],
    });
}

// Case 7
#[test]
fn idea_case_insensitive() {
    check(Case {
        input: "IDEA explore CRDTs",
        kind: EntryKind::Idea,
        body: "explore CRDTs",
        project: None,
        principal: vec![],
        free: vec![],
    });
}

// Case 8: separator before `for`, so `for fun` is body
#[test]
fn idea_separator_before_for() {
    check(Case {
        input: "idea \u{2014} for fun",
        kind: EntryKind::Idea,
        body: "for fun",
        project: None,
        principal: vec![],
        free: vec![],
    });
}

// Case 9
#[test]
fn idea_hash_tags_no_separator() {
    check(Case {
        input: "idea #rust #async explore",
        kind: EntryKind::Idea,
        body: "explore",
        project: None,
        principal: vec![],
        free: vec!["rust", "async"],
    });
}

// Case 10
#[test]
fn idea_tags_prefix_no_separator() {
    check(Case {
        input: "idea tags:rust,async explore",
        kind: EntryKind::Idea,
        body: "explore",
        project: None,
        principal: vec![],
        free: vec!["rust", "async"],
    });
}

// Case 11
#[test]
fn todo_status_when_scope_with_separator() {
    check(Case {
        input: "todo status:open when:today scope:always \u{2014} buy milk",
        kind: EntryKind::Todo,
        body: "buy milk",
        project: None,
        principal: vec![
            PrincipalTag::Status("open".to_string()),
            PrincipalTag::When("today".to_string()),
            PrincipalTag::Scope("always".to_string()),
        ],
        free: vec![],
    });
}

// Case 12
#[test]
fn todo_repeat_with_separator() {
    check(Case {
        input: "todo repeat:daily \u{2014} take meds",
        kind: EntryKind::Todo,
        body: "take meds",
        project: None,
        principal: vec![PrincipalTag::Repeat("daily".to_string())],
        free: vec![],
    });
}

// Case 13
#[test]
fn todo_due_with_separator() {
    check(Case {
        input: "todo due:2025-01-01 \u{2014} file taxes",
        kind: EntryKind::Todo,
        body: "file taxes",
        project: None,
        principal: vec![PrincipalTag::Due("2025-01-01".to_string())],
        free: vec![],
    });
}

// Case 14
#[test]
fn idea_only_verb() {
    check(Case {
        input: "idea",
        kind: EntryKind::Idea,
        body: "",
        project: None,
        principal: vec![],
        free: vec![],
    });
}

// Case 15
#[test]
fn idea_verb_trailing_spaces() {
    check(Case {
        input: "idea   ",
        kind: EntryKind::Idea,
        body: "",
        project: None,
        principal: vec![],
        free: vec![],
    });
}

// Case 16
#[test]
fn idea_project_prefix_with_separator() {
    check(Case {
        input: "idea project:my-project \u{2014} do thing",
        kind: EntryKind::Idea,
        body: "do thing",
        project: Some("my-project"),
        principal: vec![],
        free: vec![],
    });
}

// Case 17
#[test]
fn idea_for_project_priority_with_separator() {
    check(Case {
        input: "idea for my-project priority:high \u{2014} do thing",
        kind: EntryKind::Idea,
        body: "do thing",
        project: Some("my-project"),
        principal: vec![PrincipalTag::Priority("high".to_string())],
        free: vec![],
    });
}

// Case 18
#[test]
fn note_for_project_with_separator_2() {
    check(Case {
        input: "note for pro-rails \u{2014} draft API spec",
        kind: EntryKind::Note,
        body: "draft API spec",
        project: Some("pro-rails"),
        principal: vec![],
        free: vec![],
    });
}

// Case 19
#[test]
fn todo_separator_only() {
    check(Case {
        input: "todo \u{2014} buy milk",
        kind: EntryKind::Todo,
        body: "buy milk",
        project: None,
        principal: vec![],
        free: vec![],
    });
}

// Case 20
#[test]
fn todo_no_separator_body_from_tokens() {
    check(Case {
        input: "todo buy milk",
        kind: EntryKind::Todo,
        body: "buy milk",
        project: None,
        principal: vec![],
        free: vec![],
    });
}

// Case 21
#[test]
fn idea_two_word_body_no_separator() {
    check(Case {
        input: "idea hello world",
        kind: EntryKind::Idea,
        body: "hello world",
        project: None,
        principal: vec![],
        free: vec![],
    });
}

// Case 22: `for priority:high` — next word has known prefix, so `for` is NOT consumed as project
#[test]
fn idea_for_known_prefix_not_project() {
    check(Case {
        input: "idea for priority:high \u{2014} do thing",
        kind: EntryKind::Idea,
        body: "do thing",
        project: None,
        principal: vec![PrincipalTag::Priority("high".to_string())],
        free: vec![],
    });
}

// Case 23
#[test]
fn idea_for_world_with_separator() {
    check(Case {
        input: "idea for world \u{2014} body text",
        kind: EntryKind::Idea,
        body: "body text",
        project: Some("world"),
        principal: vec![],
        free: vec![],
    });
}

// Case 24: `hello` is unrecognized header token, silently dropped (separator present)
#[test]
fn idea_for_world_unrecognized_header_dropped() {
    check(Case {
        input: "idea for world hello \u{2014} body",
        kind: EntryKind::Idea,
        body: "body",
        project: Some("world"),
        principal: vec![],
        free: vec![],
    });
}

// Case 25
#[test]
fn idea_for_world_empty_body() {
    check(Case {
        input: "idea for world \u{2014} ",
        kind: EntryKind::Idea,
        body: "",
        project: Some("world"),
        principal: vec![],
        free: vec![],
    });
}

// Case 26
#[test]
fn idea_hash_tag_with_separator() {
    check(Case {
        input: "idea #rust \u{2014} explore concurrency",
        kind: EntryKind::Idea,
        body: "explore concurrency",
        project: None,
        principal: vec![],
        free: vec!["rust"],
    });
}

// Case 27
#[test]
fn todo_priority_empty_body() {
    check(Case {
        input: "todo priority:high \u{2014} ",
        kind: EntryKind::Todo,
        body: "",
        project: None,
        principal: vec![PrincipalTag::Priority("high".to_string())],
        free: vec![],
    });
}

// Case 28: priority value preserved as-is (no lowercasing)
#[test]
fn todo_priority_uppercase_value() {
    check(Case {
        input: "todo priority:HIGH \u{2014} do thing",
        kind: EntryKind::Todo,
        body: "do thing",
        project: None,
        principal: vec![PrincipalTag::Priority("HIGH".to_string())],
        free: vec![],
    });
}

// Case 29
#[test]
fn idea_separator_empty_body() {
    check(Case {
        input: "idea \u{2014} ",
        kind: EntryKind::Idea,
        body: "",
        project: None,
        principal: vec![],
        free: vec![],
    });
}

// Case 30
#[test]
fn note_for_project_body_with_spaces() {
    check(Case {
        input: "note for the-project \u{2014} remember to check logs",
        kind: EntryKind::Note,
        body: "remember to check logs",
        project: Some("the-project"),
        principal: vec![],
        free: vec![],
    });
}

// Case 31
#[test]
fn todo_for_project_with_separator() {
    check(Case {
        input: "todo for pro-rails \u{2014} finish auth",
        kind: EntryKind::Todo,
        body: "finish auth",
        project: Some("pro-rails"),
        principal: vec![],
        free: vec![],
    });
}

// Case 32
#[test]
fn capture_for_project_hash_tag_with_separator() {
    check(Case {
        input: "capture for scratch-pad \u{2014} quick note #important",
        kind: EntryKind::Capture,
        body: "quick note #important",
        project: Some("scratch-pad"),
        principal: vec![],
        free: vec![],
    });
}

// Extra: unknown verb returns error
#[test]
fn unknown_verb_returns_error() {
    let err = parse_verb("foo bar").unwrap_err();
    assert!(err.to_string().contains("unknown verb"));
}

// Extra: double-dash separator works like em-dash
#[test]
fn double_dash_separator() {
    check(Case {
        input: "idea for my-project -- do thing",
        kind: EntryKind::Idea,
        body: "do thing",
        project: Some("my-project"),
        principal: vec![],
        free: vec![],
    });
}
