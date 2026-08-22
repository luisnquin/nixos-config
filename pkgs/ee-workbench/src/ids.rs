use std::io::Read;

use anyhow::{Result, bail};
use chrono::{DateTime, Utc};

pub const MAX_SLUG_LEN: usize = 64;

/// Slugs are file names and Git paths, so they stay lowercase ASCII with
/// single inner dashes: no case-folding surprises, no shell quoting.
pub fn valid_slug(slug: &str) -> bool {
    if slug.is_empty() || slug.len() > MAX_SLUG_LEN {
        return false;
    }

    if slug.starts_with('-') || slug.ends_with('-') || slug.contains("--") {
        return false;
    }

    slug.bytes()
        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

pub fn check_slug(kind: &str, slug: &str) -> Result<()> {
    if !valid_slug(slug) {
        bail!(
            "invalid {kind} slug {slug:?}: use lowercase letters, digits and single dashes (max {MAX_SLUG_LEN})"
        );
    }

    Ok(())
}

/// `<project>/<slug>`, the only cross-domain reference form the CLI accepts.
pub fn split_ref(reference: &str) -> Result<(String, String)> {
    let Some((project, slug)) = reference.split_once('/') else {
        bail!("expected a <project>/<slug> reference, got {reference:?}");
    };

    check_slug("project", project)?;
    check_slug("experiment", slug)?;

    Ok((project.to_string(), slug.to_string()))
}

/// Event names are the primary key: sortable by time, unique per event, and
/// never rewritten. The random tail is what keeps two events inside the same
/// second from colliding.
pub fn event_id(at: DateTime<Utc>) -> Result<String> {
    Ok(format!(
        "{}-{}",
        at.format("%Y%m%dT%H%M%SZ"),
        random_hex(3)?
    ))
}

fn random_hex(bytes: usize) -> Result<String> {
    let mut buf = vec![0u8; bytes];
    std::fs::File::open("/dev/urandom")?.read_exact(&mut buf)?;

    Ok(buf.iter().map(|b| format!("{b:02x}")).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugs_reject_shapes_that_would_break_paths() {
        assert!(valid_slug("bench-psu"));
        assert!(valid_slug("rp2040"));

        assert!(!valid_slug(""));
        assert!(!valid_slug("Bench"));
        assert!(!valid_slug("-bench"));
        assert!(!valid_slug("bench-"));
        assert!(!valid_slug("bench--psu"));
        assert!(!valid_slug("bench/psu"));
        assert!(!valid_slug("bench psu"));
        assert!(!valid_slug(&"a".repeat(MAX_SLUG_LEN + 1)));
    }

    #[test]
    fn refs_split_into_project_and_slug() {
        let (project, slug) = split_ref("bench-psu/ripple").unwrap();
        assert_eq!((project.as_str(), slug.as_str()), ("bench-psu", "ripple"));

        assert!(split_ref("ripple").is_err());
        assert!(split_ref("Bench/ripple").is_err());
    }

    #[test]
    fn event_ids_sort_by_time_and_do_not_repeat() {
        let at = DateTime::parse_from_rfc3339("2026-08-21T14:30:12Z")
            .unwrap()
            .to_utc();

        let id = event_id(at).unwrap();
        assert!(id.starts_with("20260821T143012Z-"), "{id}");
        assert_eq!(id.len(), "20260821T143012Z-".len() + 6);

        let other = event_id(at).unwrap();
        assert_ne!(id, other);
    }
}
