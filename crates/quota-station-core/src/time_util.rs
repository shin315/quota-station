//! RFC 3339 timestamps as quota-axi writes them.

use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

pub fn parse_ts(text: &str) -> Option<OffsetDateTime> {
    OffsetDateTime::parse(text, &Rfc3339).ok()
}

/// The latest of `stamps`, as written; unparseable stamps are skipped.
pub fn latest<'a>(stamps: impl IntoIterator<Item = &'a str>) -> Option<String> {
    stamps
        .into_iter()
        .filter_map(|s| parse_ts(s).map(|t| (t, s)))
        .max_by_key(|(t, _)| *t)
        .map(|(_, s)| s.to_string())
}

/// `at` is still ahead of `now`. An unknown clock counts as ahead, so an end
/// time is never reported as passed without evidence.
pub fn is_after(at: &str, now: Option<OffsetDateTime>) -> bool {
    match (parse_ts(at), now) {
        (Some(at), Some(now)) => at > now,
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn latest_compares_instants_not_text() {
        let got = latest(["2026-01-15T07:00:00.000Z", "2026-01-15T08:30:00+02:00", "nope"]);
        assert_eq!(got.as_deref(), Some("2026-01-15T07:00:00.000Z"));
    }

    #[test]
    fn unknown_clock_keeps_an_end_ahead() {
        assert!(is_after("2026-01-15T07:00:00Z", None));
        assert!(!is_after("2026-01-15T07:00:00Z", parse_ts("2026-01-15T07:00:01Z")));
    }
}
