use crate::error::{Error, Result};
use chrono::Duration;

pub fn parse_duration(s: &str) -> Result<Duration> {
    let s = s.trim().to_lowercase();
    if s.is_empty() {
        return Err(Error::Parse("empty duration".into()));
    }

    let split_pos = s.find(|c: char| c.is_alphabetic()).unwrap_or(s.len());
    let (num_str, suffix) = s.split_at(split_pos);

    let num_str = num_str.trim();
    let value: f64 = if num_str.is_empty() {
        1.0
    } else {
        num_str
            .parse()
            .map_err(|_| Error::Parse(format!("invalid number '{num_str}'")))?
    };

    let secs = match suffix {
        "s" | "sec" | "secs" | "second" | "seconds" => value,
        "m" | "min" | "mins" | "minute" | "minutes" | "" => value * 60.0,
        "h" | "hr" | "hrs" | "hour" | "hours" => value * 3600.0,
        "d" | "day" | "days" => value * 86400.0,
        "w" | "week" | "weeks" => value * 604800.0,
        "mo" | "month" | "months" => value * 2592000.0,
        "y" | "yr" | "year" | "years" => {
            return Err(Error::Parse("years not supported in duration".into()))
        }
        other => return Err(Error::Parse(format!("unknown duration unit '{other}'"))),
    };

    Ok(Duration::seconds(secs as i64))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_seconds() {
        assert_eq!(parse_duration("30s").unwrap(), Duration::seconds(30));
    }

    #[test]
    fn parse_sec_long() {
        assert_eq!(parse_duration("1second").unwrap(), Duration::seconds(1));
    }

    #[test]
    fn parse_seconds_plural() {
        assert_eq!(parse_duration("10seconds").unwrap(), Duration::seconds(10));
    }

    #[test]
    fn parse_minutes_m() {
        assert_eq!(parse_duration("10m").unwrap(), Duration::minutes(10));
    }

    #[test]
    fn parse_minutes_min() {
        assert_eq!(parse_duration("5min").unwrap(), Duration::minutes(5));
    }

    #[test]
    fn parse_minutes_long() {
        assert_eq!(parse_duration("2minutes").unwrap(), Duration::minutes(2));
    }

    #[test]
    fn parse_hours_h() {
        assert_eq!(parse_duration("2h").unwrap(), Duration::hours(2));
    }

    #[test]
    fn parse_hours_hr() {
        assert_eq!(parse_duration("1hr").unwrap(), Duration::hours(1));
    }

    #[test]
    fn parse_hours_long() {
        assert_eq!(parse_duration("3hours").unwrap(), Duration::hours(3));
    }

    #[test]
    fn parse_days() {
        assert_eq!(parse_duration("1d").unwrap(), Duration::days(1));
    }

    #[test]
    fn parse_days_long() {
        assert_eq!(parse_duration("7days").unwrap(), Duration::days(7));
    }

    #[test]
    fn parse_weeks() {
        assert_eq!(parse_duration("2w").unwrap(), Duration::weeks(2));
    }

    #[test]
    fn parse_weeks_long() {
        assert_eq!(parse_duration("1week").unwrap(), Duration::weeks(1));
    }

    #[test]
    fn parse_months() {
        assert_eq!(
            parse_duration("1month").unwrap(),
            Duration::seconds(2592000)
        );
    }

    #[test]
    fn parse_months_mo() {
        assert_eq!(parse_duration("2mo").unwrap(), Duration::seconds(5184000));
    }

    #[test]
    fn parse_years_returns_error() {
        assert!(parse_duration("1y").is_err());
    }

    #[test]
    fn parse_unknown_suffix_returns_error() {
        assert!(parse_duration("5x").is_err());
    }

    #[test]
    fn parse_empty_returns_error() {
        assert!(parse_duration("").is_err());
    }

    #[test]
    fn parse_bare_number_defaults_to_minutes() {
        assert_eq!(parse_duration("15").unwrap(), Duration::minutes(15));
    }

    #[test]
    fn parse_fractional_hours() {
        assert_eq!(parse_duration("1.5h").unwrap(), Duration::minutes(90));
    }
}
