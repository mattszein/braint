//! Verb grammar parser — first-token dispatch, pure string operations.
//!
//! No external parser crates. No regex. No I/O.

use braint_proto::{EntryKind, PrincipalTag, ProjectId, TagSet};

use crate::error::{CoreError, Result};

/// A fully-parsed verb invocation: kind, body, optional project, and tags.
#[derive(Debug, Clone, PartialEq)]
pub struct VerbInvocation {
    /// The type of entry this invocation creates.
    pub kind: EntryKind,
    /// The free-form body text.
    pub body: String,
    /// Optional project assignment.
    pub project: Option<ProjectId>,
    /// Structured and free tags parsed from header tokens.
    pub tags: TagSet,
}

/// Known prefix strings that prevent `for <next>` from consuming a project id.
const KNOWN_PREFIXES: &[&str] = &[
    "project:", "status:", "priority:", "when:", "due:", "scope:", "repeat:", "type:", "tags:", "#",
];

fn has_known_prefix(token: &str) -> bool {
    KNOWN_PREFIXES
        .iter()
        .any(|p| token.starts_with(p))
}

/// Parse a verb invocation from free-form text.
///
/// Grammar:
/// ```text
/// <verb> [header-tokens]* [— | --] [body text]
/// ```
///
/// The first token (case-insensitive) selects the `EntryKind`.
/// Everything between the verb and the separator (if any) is the header.
/// Everything after the separator is the body.
/// If there is no separator, unrecognized header tokens form the body.
pub fn parse_verb(text: &str) -> Result<VerbInvocation> {
    let tokens: Vec<&str> = text.split_whitespace().collect();

    if tokens.is_empty() {
        return Err(CoreError::Verb("empty input".to_string()));
    }

    let first = tokens[0];
    let kind = match first.to_lowercase().as_str() {
        "idea" => EntryKind::Idea,
        "todo" => EntryKind::Todo,
        "note" => EntryKind::Note,
        "capture" => EntryKind::Capture,
        other => return Err(CoreError::Verb(format!("unknown verb: {other}"))),
    };

    // Find separator index (the token that is exactly "—" or "--")
    let sep_pos = tokens[1..]
        .iter()
        .position(|t| *t == "\u{2014}" || *t == "--")
        .map(|i| i + 1); // adjust for the slice offset

    // Split into header tokens and body tokens
    let (header_tokens, body_tokens): (&[&str], &[&str]) = match sep_pos {
        Some(sep) => (&tokens[1..sep], &tokens[sep + 1..]),
        None => (&tokens[1..], &[]),
    };

    let has_separator = sep_pos.is_some();

    let mut project: Option<ProjectId> = None;
    let mut principal: Vec<PrincipalTag> = Vec::new();
    let mut free: Vec<String> = Vec::new();
    let mut body_accum: Vec<&str> = Vec::new();

    let mut i = 0;
    while i < header_tokens.len() {
        let tok = header_tokens[i];

        if tok == "for" {
            // `for <next_word>` — consume next word as project if it doesn't start with a known prefix
            if let Some(next) = header_tokens.get(i + 1).filter(|n| !has_known_prefix(n)) {
                project = Some(ProjectId(next.to_string()));
                i += 2;
                continue;
            }
            // `for` with no valid next — fall through to unrecognized
            if has_separator {
                // silently ignore
            } else {
                body_accum.push(tok);
            }
            i += 1;
            continue;
        }

        if let Some(val) = tok.strip_prefix("project:") {
            project = Some(ProjectId(val.to_string()));
        } else if let Some(val) = tok.strip_prefix("status:") {
            principal.push(PrincipalTag::Status(val.to_string()));
        } else if let Some(val) = tok.strip_prefix("priority:") {
            principal.push(PrincipalTag::Priority(val.to_string()));
        } else if let Some(val) = tok.strip_prefix("when:") {
            principal.push(PrincipalTag::When(val.to_string()));
        } else if let Some(val) = tok.strip_prefix("due:") {
            principal.push(PrincipalTag::Due(val.to_string()));
        } else if let Some(val) = tok.strip_prefix("scope:") {
            principal.push(PrincipalTag::Scope(val.to_string()));
        } else if let Some(val) = tok.strip_prefix("repeat:") {
            principal.push(PrincipalTag::Repeat(val.to_string()));
        } else if let Some(val) = tok.strip_prefix("type:") {
            principal.push(PrincipalTag::Type(val.to_string()));
        } else if let Some(val) = tok.strip_prefix("tags:") {
            for tag in val.split(',') {
                let t = tag.trim();
                if !t.is_empty() {
                    free.push(t.to_string());
                }
            }
        } else if let Some(val) = tok.strip_prefix('#') {
            if !val.is_empty() {
                free.push(val.to_string());
            }
        } else {
            // unrecognized token
            if has_separator {
                // silently drop
            } else {
                body_accum.push(tok);
            }
        }

        i += 1;
    }

    // Build body
    let body = if has_separator {
        body_tokens.join(" ").trim().to_string()
    } else {
        body_accum.join(" ").trim().to_string()
    };

    Ok(VerbInvocation {
        kind,
        body,
        project,
        tags: TagSet { principal, free },
    })
}
