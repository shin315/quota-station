// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! This Mac's wall clock, written the way the switcher's pause flag and log
//! write it: `2026-10-16T18:30`, no offset.
//!
//! The `time` crate will not read the local offset in a multi-threaded
//! process, so the offset comes from `localtime_r`, which is thread-safe.

use time::{Date, Month, OffsetDateTime, PrimitiveDateTime, Time, UtcOffset};

/// The offset of this Mac's time zone at `at`.
pub fn offset_at(at: OffsetDateTime) -> UtcOffset {
    // `time_t` is 64-bit on macOS.
    let t = at.unix_timestamp() as libc::time_t;
    // SAFETY: `localtime_r` writes only into the `tm` it is given, and a
    // zeroed `tm` is a valid value of that plain C struct.
    let mut tm: libc::tm = unsafe { std::mem::zeroed() };
    if unsafe { libc::localtime_r(&t, &mut tm) }.is_null() {
        return UtcOffset::UTC;
    }
    i32::try_from(tm.tm_gmtoff)
        .ok()
        .and_then(|s| UtcOffset::from_whole_seconds(s).ok())
        .unwrap_or(UtcOffset::UTC)
}

/// `at` on this Mac's clock, to the second: `2026-10-16T18:30:05`.
pub fn stamp(at: OffsetDateTime) -> String {
    let l = at.to_offset(offset_at(at));
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}",
        l.year(),
        u8::from(l.month()),
        l.day(),
        l.hour(),
        l.minute(),
        l.second()
    )
}

/// `at` on this Mac's clock, to the minute, as the pause flag stores it.
pub fn minute(at: OffsetDateTime) -> String {
    stamp(at)[..16].to_string()
}

/// `HH:MM` on this Mac's clock.
pub fn clock(at: OffsetDateTime) -> String {
    stamp(at)[11..16].to_string()
}

/// A wall-clock time on this Mac as an instant. Takes `2026-10-16T18:30`,
/// with optional seconds, and the log's `2026-10-16 18:30:05`; anything after
/// is ignored. `None` when the text starts with no such time.
pub fn instant(text: &str) -> Option<OffsetDateTime> {
    let naive = parse(text)?;
    // The offset in force at that wall time: start from the offset at the
    // same digits read as UTC, then correct once for a change in between.
    let first = offset_at(naive.assume_utc());
    let second = offset_at(naive.assume_offset(first));
    Some(naive.assume_offset(second))
}

fn parse(text: &str) -> Option<PrimitiveDateTime> {
    let b = text.as_bytes();
    let num = |from: usize, len: usize| -> Option<u32> {
        let part = b.get(from..from + len)?;
        part.iter()
            .all(u8::is_ascii_digit)
            .then(|| part.iter().fold(0, |n, d| n * 10 + u32::from(d - b'0')))
    };
    let sep = |at: usize, ok: &[u8]| b.get(at).is_some_and(|c| ok.contains(c));
    if !(sep(4, b"-") && sep(7, b"-") && sep(10, b"T ") && sep(13, b":")) {
        return None;
    }
    let second = if sep(16, b":") { num(17, 2)? } else { 0 };
    let date = Date::from_calendar_date(
        i32::try_from(num(0, 4)?).ok()?,
        Month::try_from(u8::try_from(num(5, 2)?).ok()?).ok()?,
        u8::try_from(num(8, 2)?).ok()?,
    )
    .ok()?;
    let time = Time::from_hms(
        u8::try_from(num(11, 2)?).ok()?,
        u8::try_from(num(14, 2)?).ok()?,
        u8::try_from(second).ok()?,
    )
    .ok()?;
    Some(PrimitiveDateTime::new(date, time))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_stamp_reads_back_as_the_same_instant() {
        let at = OffsetDateTime::from_unix_timestamp(1_792_303_394).unwrap();
        assert_eq!(instant(&stamp(at)), Some(at));
        let minute = minute(at);
        assert_eq!(minute.len(), 16);
        assert_eq!(instant(&minute), Some(at.replace_second(0).unwrap()));
        assert_eq!(clock(at), minute[11..]);
    }

    #[test]
    fn the_log_format_and_a_trailing_part_are_accepted() {
        let t = instant("2026-10-16T18:30").unwrap();
        assert_eq!(instant("2026-10-16 18:30:00"), Some(t));
        assert_eq!(instant("2026-10-16T18:30+07:00"), Some(t));
    }

    #[test]
    fn text_that_is_not_a_time_gives_none() {
        for bad in [
            "",
            "tomorrow",
            "2026-10-16",
            "2026-13-01T10:00",
            "2026-10-16T25:00",
            "2026/10/16 10:00",
        ] {
            assert_eq!(instant(bad), None, "{bad}");
        }
    }
}
