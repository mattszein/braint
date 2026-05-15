//! Server-side subscription filter evaluation.

use braint_proto::{Entry, EntryFilter};

/// Returns `true` if `entry` matches all constraints in `filter`.
pub fn filter_matches(filter: &EntryFilter, entry: &Entry) -> bool {
    if let Some(kind) = filter.kind
        && entry.kind != kind
    {
        return false;
    }
    if let Some(ref proj) = filter.project
        && entry.project.as_ref() != Some(proj)
    {
        return false;
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
    if let Some(since_ms) = filter.since_ms
        && entry.created_at.physical_ms < since_ms
    {
        return false;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use braint_proto::{
        DeviceId, Entry, EntryId, EntryFilter, EntryKind, HybridLogicalClock,
        PrincipalTag, ProjectId, TagSet,
    };

    fn make_entry(kind: EntryKind, project: Option<&str>, free: &[&str], principal: &[PrincipalTag], physical_ms: u64) -> Entry {
        let device = DeviceId::generate();
        let hlc = HybridLogicalClock { physical_ms, logical: 0, device_id: device };
        Entry {
            id: EntryId::generate(),
            kind,
            body: "body".to_string(),
            created_at: hlc,
            created_on_device: device,
            last_modified_at: hlc,
            last_modified_on_device: device,
            project: project.map(|p| ProjectId(p.to_string())),
            tags: TagSet {
                free: free.iter().map(|s| s.to_string()).collect(),
                principal: principal.to_vec(),
            },
        }
    }

    fn empty_filter() -> EntryFilter { EntryFilter::default() }

    #[test]
    fn empty_filter_matches_everything() {
        let e = make_entry(EntryKind::Idea, None, &[], &[], 1000);
        assert!(filter_matches(&empty_filter(), &e));
    }

    #[test]
    fn kind_filter_matches() {
        let idea = make_entry(EntryKind::Idea, None, &[], &[], 1000);
        let todo = make_entry(EntryKind::Todo, None, &[], &[], 1000);
        let f = EntryFilter { kind: Some(EntryKind::Idea), ..Default::default() };
        assert!(filter_matches(&f, &idea));
        assert!(!filter_matches(&f, &todo));
    }

    #[test]
    fn project_filter_matches() {
        let with_proj = make_entry(EntryKind::Idea, Some("braint"), &[], &[], 1000);
        let other_proj = make_entry(EntryKind::Idea, Some("other"), &[], &[], 1000);
        let no_proj = make_entry(EntryKind::Idea, None, &[], &[], 1000);
        let f = EntryFilter { project: Some(ProjectId("braint".to_string())), ..Default::default() };
        assert!(filter_matches(&f, &with_proj));
        assert!(!filter_matches(&f, &other_proj));
        assert!(!filter_matches(&f, &no_proj));
    }

    #[test]
    fn free_tag_filter_all_required() {
        let e = make_entry(EntryKind::Idea, None, &["rust", "async"], &[], 1000);
        let f_one = EntryFilter { free_tags: vec!["rust".to_string()], ..Default::default() };
        let f_both = EntryFilter { free_tags: vec!["rust".to_string(), "async".to_string()], ..Default::default() };
        let f_missing = EntryFilter { free_tags: vec!["python".to_string()], ..Default::default() };
        assert!(filter_matches(&f_one, &e));
        assert!(filter_matches(&f_both, &e));
        assert!(!filter_matches(&f_missing, &e));
    }

    #[test]
    fn principal_tag_filter() {
        let e = make_entry(EntryKind::Todo, None, &[], &[PrincipalTag::Priority("high".to_string())], 1000);
        let f_match = EntryFilter {
            principal_match: vec![PrincipalTag::Priority("high".to_string())],
            ..Default::default()
        };
        let f_wrong_val = EntryFilter {
            principal_match: vec![PrincipalTag::Priority("low".to_string())],
            ..Default::default()
        };
        let f_wrong_kind = EntryFilter {
            principal_match: vec![PrincipalTag::Status("high".to_string())],
            ..Default::default()
        };
        assert!(filter_matches(&f_match, &e));
        assert!(!filter_matches(&f_wrong_val, &e));
        assert!(!filter_matches(&f_wrong_kind, &e));
    }

    #[test]
    fn since_ms_filter() {
        let old = make_entry(EntryKind::Idea, None, &[], &[], 500);
        let new = make_entry(EntryKind::Idea, None, &[], &[], 1500);
        let f = EntryFilter { since_ms: Some(1000), ..Default::default() };
        assert!(!filter_matches(&f, &old));
        assert!(filter_matches(&f, &new));
    }

    #[test]
    fn combined_filter_all_must_match() {
        let matching = make_entry(EntryKind::Todo, Some("proj"), &["tag1"], &[PrincipalTag::Status("open".to_string())], 2000);
        let wrong_kind = make_entry(EntryKind::Idea, Some("proj"), &["tag1"], &[PrincipalTag::Status("open".to_string())], 2000);
        let f = EntryFilter {
            kind: Some(EntryKind::Todo),
            project: Some(ProjectId("proj".to_string())),
            free_tags: vec!["tag1".to_string()],
            principal_match: vec![PrincipalTag::Status("open".to_string())],
            since_ms: Some(1000),
            ..Default::default()
        };
        assert!(filter_matches(&f, &matching));
        assert!(!filter_matches(&f, &wrong_kind));
    }
}
