use task::time::{parse_due, parse_duration};
use chrono::{Duration, Local, TimeZone};

fn now() -> chrono::DateTime<Local> {
    Local.with_ymd_and_hms(2026, 5, 17, 10, 0, 0).unwrap()
}

#[test]
fn duration_10min_roundtrip() {
    let d = parse_duration("10min").unwrap();
    assert_eq!(d, Duration::minutes(10));
}

#[test]
fn duration_2h_roundtrip() {
    let d = parse_duration("2h").unwrap();
    assert_eq!(d, Duration::hours(2));
}

#[test]
fn due_tomorrow_is_next_day() {
    let now = now();
    let due = parse_due("tomorrow", now).unwrap();
    assert_eq!(due.date_naive(), (now + Duration::days(1)).date_naive());
}

#[test]
fn due_3d_is_three_days_ahead() {
    let now = now();
    let due = parse_due("3d", now).unwrap();
    assert!(due > now + Duration::days(2));
}

#[test]
fn due_jun15_picks_current_year() {
    let now = now(); // May 17
    let due = parse_due("jun15", now).unwrap();
    assert_eq!(due.month(), 6);
    assert_eq!(due.day(), 15);
    assert_eq!(due.year(), 2026);
}

#[test]
fn due_jan1_wraps_to_next_year() {
    let now = now(); // May 17, 2026
    let due = parse_due("jan1", now).unwrap();
    assert_eq!(due.year(), 2027);
    assert_eq!(due.month(), 1);
    assert_eq!(due.day(), 1);
}

use chrono::Datelike;
