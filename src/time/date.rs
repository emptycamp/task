use crate::error::{Error, Result};
use crate::time::duration::parse_duration;
use chrono::{DateTime, Datelike, Duration, Local, NaiveDate, TimeZone, Weekday};

pub fn parse_due(s: &str, now: DateTime<Local>) -> Result<DateTime<Local>> {
    let s = s.trim().to_lowercase();

    if let Some(dt) = try_keyword(&s, now) {
        return validate(dt, now);
    }

    if let Some(dt) = try_weekday(&s, now) {
        return validate(dt, now);
    }

    if let Some(dt) = try_month_day(&s, now) {
        return validate(dt, now);
    }

    let dur = parse_duration(&s)?;
    validate(now + dur, now)
}

fn try_keyword(s: &str, now: DateTime<Local>) -> Option<DateTime<Local>> {
    match s {
        "today" | "tonight" => Some(local_at(now, 9, 0)),
        "tomorrow" => Some(local_at(now + Duration::days(1), 9, 0)),
        _ => None,
    }
}

fn local_at(base: DateTime<Local>, hour: u32, min: u32) -> DateTime<Local> {
    base.date_naive()
        .and_hms_opt(hour, min, 0)
        .and_then(|ndt| Local.from_local_datetime(&ndt).single())
        .unwrap_or(base)
}

fn try_weekday(s: &str, now: DateTime<Local>) -> Option<DateTime<Local>> {
    let target = parse_weekday(s)?;
    let today = now.weekday();
    let days_ahead = days_until(today, target);
    let date = now + Duration::days(days_ahead as i64);
    Some(local_at(date, 9, 0))
}

fn parse_weekday(s: &str) -> Option<Weekday> {
    match s {
        "mon" | "monday" => Some(Weekday::Mon),
        "tue" | "tuesday" => Some(Weekday::Tue),
        "wed" | "wednesday" => Some(Weekday::Wed),
        "thu" | "thursday" => Some(Weekday::Thu),
        "fri" | "friday" => Some(Weekday::Fri),
        "sat" | "saturday" => Some(Weekday::Sat),
        "sun" | "sunday" => Some(Weekday::Sun),
        _ => None,
    }
}

fn days_until(from: Weekday, to: Weekday) -> u32 {
    let from_num = from.num_days_from_monday();
    let to_num = to.num_days_from_monday();
    if to_num > from_num {
        to_num - from_num
    } else {
        7 - from_num + to_num
    }
}

fn try_month_day(s: &str, now: DateTime<Local>) -> Option<DateTime<Local>> {
    let months = [
        ("january", 1),
        ("jan", 1),
        ("february", 2),
        ("feb", 2),
        ("march", 3),
        ("mar", 3),
        ("april", 4),
        ("apr", 4),
        ("may", 5),
        ("june", 6),
        ("jun", 6),
        ("july", 7),
        ("jul", 7),
        ("august", 8),
        ("aug", 8),
        ("september", 9),
        ("sep", 9),
        ("october", 10),
        ("oct", 10),
        ("november", 11),
        ("nov", 11),
        ("december", 12),
        ("dec", 12),
    ];

    for (name, month) in &months {
        if let Some(rest) = s.strip_prefix(name) {
            let rest = rest.trim_start_matches(|c: char| c == ' ' || c == '-' || c == '/');
            if rest.is_empty() {
                return None;
            }
            let day: u32 = rest.parse().ok()?;
            let year = now.year();
            let candidate = NaiveDate::from_ymd_opt(year, *month, day)?;
            let dt = Local
                .from_local_datetime(&candidate.and_hms_opt(9, 0, 0)?)
                .single()?;
            if dt > now {
                return Some(dt);
            }
            let next_year = NaiveDate::from_ymd_opt(year + 1, *month, day)?;
            return Local
                .from_local_datetime(&next_year.and_hms_opt(9, 0, 0)?)
                .single();
        }
    }
    None
}

fn validate(dt: DateTime<Local>, now: DateTime<Local>) -> Result<DateTime<Local>> {
    let limit = now + Duration::days(366);
    if dt > limit {
        return Err(Error::Parse(
            "due date must be within 12 months".into(),
        ));
    }
    Ok(dt)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn make_now(year: i32, month: u32, day: u32) -> DateTime<Local> {
        Local
            .from_local_datetime(
                &chrono::NaiveDate::from_ymd_opt(year, month, day)
                    .unwrap()
                    .and_hms_opt(10, 0, 0)
                    .unwrap(),
            )
            .unwrap()
    }

    #[test]
    fn keyword_today() {
        let now = make_now(2026, 5, 17);
        let due = parse_due("today", now).unwrap();
        assert_eq!(due.date_naive(), now.date_naive());
    }

    #[test]
    fn keyword_tonight() {
        let now = make_now(2026, 5, 17);
        let due = parse_due("tonight", now).unwrap();
        assert_eq!(due.date_naive(), now.date_naive());
    }

    #[test]
    fn keyword_tomorrow() {
        let now = make_now(2026, 5, 17);
        let due = parse_due("tomorrow", now).unwrap();
        assert_eq!(due.date_naive(), (now + Duration::days(1)).date_naive());
    }

    #[test]
    fn weekday_next_wednesday_from_wednesday() {
        // On a Wednesday, "wed" returns next Wednesday (7 days)
        let now = make_now(2026, 5, 20); // May 20, 2026 is a Wednesday
        let due = parse_due("wed", now).unwrap();
        let expected = make_now(2026, 5, 27);
        assert_eq!(due.date_naive(), expected.date_naive());
    }

    #[test]
    fn weekday_friday_from_monday() {
        let now = make_now(2026, 5, 18); // Monday
        let due = parse_due("fri", now).unwrap();
        let expected = make_now(2026, 5, 22);
        assert_eq!(due.date_naive(), expected.date_naive());
    }

    #[test]
    fn weekday_full_name() {
        let now = make_now(2026, 5, 18); // Monday
        let due = parse_due("friday", now).unwrap();
        let expected = make_now(2026, 5, 22);
        assert_eq!(due.date_naive(), expected.date_naive());
    }

    #[test]
    fn month_day_future_same_year() {
        let now = make_now(2026, 5, 17);
        let due = parse_due("jun1", now).unwrap();
        assert_eq!(due.year(), 2026);
        assert_eq!(due.month(), 6);
        assert_eq!(due.day(), 1);
    }

    #[test]
    fn month_day_past_same_year_wraps_to_next_year() {
        let now = make_now(2026, 5, 17);
        let due = parse_due("mar2", now).unwrap();
        assert_eq!(due.year(), 2027);
        assert_eq!(due.month(), 3);
        assert_eq!(due.day(), 2);
    }

    #[test]
    fn month_full_name_with_space() {
        let now = make_now(2026, 5, 17);
        let due = parse_due("june 1", now).unwrap();
        assert_eq!(due.month(), 6);
        assert_eq!(due.day(), 1);
    }

    #[test]
    fn duration_fallthrough() {
        let now = make_now(2026, 5, 17);
        let due = parse_due("2h", now).unwrap();
        let expected = now + Duration::hours(2);
        assert_eq!(due, expected);
    }

    #[test]
    fn beyond_12_months_returns_error() {
        let now = make_now(2026, 5, 17);
        assert!(parse_due("400d", now).is_err());
    }
}
