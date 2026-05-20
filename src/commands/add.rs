use crate::clock::Clock;
use crate::error::{Error, Result};
use crate::model::{Priority, Status, Task};
use crate::store::Store;
use crate::time::{parse_due, parse_duration};
use chrono::Local;

pub fn run(args: &[String], store: &mut Store, clock: &dyn Clock) -> Result<Task> {
    let now_utc = clock.now();
    let now_local: chrono::DateTime<Local> = now_utc.into();

    let mut text_parts: Vec<String> = Vec::new();
    let mut priority = Priority::B;
    let mut due = now_utc + chrono::Duration::minutes(5);
    let mut est_secs: i64 = 1800;

    for arg in args {
        if let Some(rest) = arg.strip_prefix("p:") {
            priority = rest
                .parse()
                .map_err(|e: String| Error::Parse(e))?;
        } else if let Some(rest) = arg.strip_prefix("due:") {
            due = parse_due(rest, now_local)?.with_timezone(&chrono::Utc);
        } else if let Some(rest) = arg.strip_prefix("est:") {
            est_secs = parse_duration(rest)?.num_seconds();
        } else {
            text_parts.push(arg.clone());
        }
    }

    if text_parts.is_empty() {
        return Err(Error::Parse("task text is required".into()));
    }

    let text = text_parts.join(" ");
    let id = store.next_id()?;

    let task = Task {
        id,
        text,
        priority,
        due,
        est_secs,
        status: Status::Active,
        created_at: now_utc,
        completed_at: None,
        deleted_at: None,
    };

    store.add_task_with_revert(task, clock)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::FakeClock;
    use chrono::{TimeZone, Utc};
    use tempfile::tempdir;

    fn make_clock() -> FakeClock {
        FakeClock::new(Utc.with_ymd_and_hms(2026, 5, 17, 12, 0, 0).unwrap())
    }

    fn open_store(dir: &std::path::Path) -> Store {
        Store::open(dir).unwrap()
    }

    #[test]
    fn add_basic_task() {
        let dir = tempdir().unwrap();
        let mut store = open_store(dir.path());
        let clock = make_clock();
        let args: Vec<String> = vec!["Buy milk".into()];
        let task = run(&args, &mut store, &clock).unwrap();
        assert_eq!(task.text, "Buy milk");
        assert_eq!(task.priority, Priority::B);
        assert_eq!(task.id, 1);
    }

    #[test]
    fn add_task_with_priority() {
        let dir = tempdir().unwrap();
        let mut store = open_store(dir.path());
        let clock = make_clock();
        let args: Vec<String> = vec!["Read book".into(), "p:a".into()];
        let task = run(&args, &mut store, &clock).unwrap();
        assert_eq!(task.priority, Priority::A);
    }

    #[test]
    fn add_task_with_est() {
        let dir = tempdir().unwrap();
        let mut store = open_store(dir.path());
        let clock = make_clock();
        let args: Vec<String> = vec!["Read book".into(), "est:1h".into()];
        let task = run(&args, &mut store, &clock).unwrap();
        assert_eq!(task.est_secs, 3600);
    }

    #[test]
    fn add_task_no_text_returns_error() {
        let dir = tempdir().unwrap();
        let mut store = open_store(dir.path());
        let clock = make_clock();
        let args: Vec<String> = vec!["p:a".into()];
        assert!(run(&args, &mut store, &clock).is_err());
    }

    #[test]
    fn add_assigns_incremental_ids() {
        let dir = tempdir().unwrap();
        let mut store = open_store(dir.path());
        let clock = make_clock();
        let t1 = run(&["Task one".to_string()], &mut store, &clock).unwrap();
        let t2 = run(&["Task two".to_string()], &mut store, &clock).unwrap();
        assert_eq!(t1.id, 1);
        assert_eq!(t2.id, 2);
    }
}
