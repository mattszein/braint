//! Server-side subscription filter evaluation.

use braint_proto::{Entry, EntryFilter};

/// Returns `true` if `entry` matches all constraints in `filter`.
pub fn filter_matches(filter: &EntryFilter, entry: &Entry) -> bool {
    if let Some(kind) = filter.kind {
        if entry.kind != kind {
            return false;
        }
    }
    if let Some(ref proj) = filter.project {
        if entry.project.as_ref() != Some(proj) {
            return false;
        }
    }
    for tag in &filter.free_tags {
        if !entry.tags.free.iter().any(|t| t == tag) {
            return false;
        }
    }
    for principal in &filter.principal_match {
        if !entry.tags.principal.iter().any(|p| {
            p.prefix() == principal.prefix() && p.value() == principal.value()
        }) {
            return false;
        }
    }
    if let Some(since_ms) = filter.since_ms {
        if entry.created_at.physical_ms < since_ms {
            return false;
        }
    }
    true
}
