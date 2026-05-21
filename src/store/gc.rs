use crate::clock::Clock;
use crate::error::Result;
use crate::model::{Priority, Status};
use crate::store::Store;
use chrono::Duration;

pub fn sweep(store: &mut Store, clock: &dyn Clock) -> Result<u32> {
    let now = clock.now();
    let tasks = store.all_tasks()?;
    let mut count = 0u32;

    for task in tasks {
        let should_delete = match (task.status, task.priority) {
            (Status::Active, Priority::C) => now - task.created_at > Duration::days(3),
            (Status::Active, Priority::B) => now - task.created_at > Duration::days(7),
            (Status::Active, Priority::A) => false,
            (Status::Completed, _) => task
                .completed_at
                .map(|t| now - t > Duration::days(14))
                .unwrap_or(false),
            (Status::SoftDeleted, _) => task
                .deleted_at
                .map(|t| now - t > Duration::days(1))
                .unwrap_or(false),
        };

        if should_delete {
            store.hard_delete(task.id)?;
            count += 1;
        }
    }

    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::FakeClock;
    use crate::model::{Priority, Status, Task};
    use crate::store::Store;
    use chrono::{TimeZone, Utc};
    use tempfile::tempdir;

    fn make_task(
        id: u32,
        priority: Priority,
        status: Status,
        created_at: chrono::DateTime<Utc>,
    ) -> Task {
        Task {
            id,
            text: format!("task {id}"),
            priority,
            due: created_at,
            est_secs: 1800,
            status,
            created_at,
            completed_at: if status == Status::Completed {
                Some(created_at)
            } else {
                None
            },
            deleted_at: if status == Status::SoftDeleted {
                Some(created_at)
            } else {
                None
            },
        }
    }

    fn open_store(dir: &std::path::Path) -> Store {
        Store::open(dir).unwrap()
    }

    #[test]
    fn gc_deletes_priority_c_after_3_days() {
        let dir = tempdir().unwrap();
        let mut store = open_store(dir.path());
        let base = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        let task = make_task(1, Priority::C, Status::Active, base);
        store.add_task(task).unwrap();

        let clock = FakeClock::new(base + Duration::days(4));
        let count = sweep(&mut store, &clock).unwrap();
        assert_eq!(count, 1);
        assert!(store.get_task(1).is_err());
    }

    #[test]
    fn gc_does_not_delete_priority_c_before_3_days() {
        let dir = tempdir().unwrap();
        let mut store = open_store(dir.path());
        let base = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        let task = make_task(1, Priority::C, Status::Active, base);
        store.add_task(task).unwrap();

        let clock = FakeClock::new(base + Duration::days(2));
        let count = sweep(&mut store, &clock).unwrap();
        assert_eq!(count, 0);
        assert!(store.get_task(1).is_ok());
    }

    #[test]
    fn gc_deletes_priority_b_after_7_days() {
        let dir = tempdir().unwrap();
        let mut store = open_store(dir.path());
        let base = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        let task = make_task(1, Priority::B, Status::Active, base);
        store.add_task(task).unwrap();

        let clock = FakeClock::new(base + Duration::days(8));
        let count = sweep(&mut store, &clock).unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn gc_never_deletes_priority_a() {
        let dir = tempdir().unwrap();
        let mut store = open_store(dir.path());
        let base = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        let task = make_task(1, Priority::A, Status::Active, base);
        store.add_task(task).unwrap();

        let clock = FakeClock::new(base + Duration::days(365));
        let count = sweep(&mut store, &clock).unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn gc_deletes_completed_after_14_days() {
        let dir = tempdir().unwrap();
        let mut store = open_store(dir.path());
        let base = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        let mut task = make_task(1, Priority::B, Status::Completed, base);
        task.completed_at = Some(base);
        store.add_task(task).unwrap();

        let clock = FakeClock::new(base + Duration::days(15));
        let count = sweep(&mut store, &clock).unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn gc_deletes_soft_deleted_after_1_day() {
        let dir = tempdir().unwrap();
        let mut store = open_store(dir.path());
        let base = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        let mut task = make_task(1, Priority::B, Status::SoftDeleted, base);
        task.deleted_at = Some(base);
        store.add_task(task).unwrap();

        let clock = FakeClock::new(base + Duration::days(2));
        let count = sweep(&mut store, &clock).unwrap();
        assert_eq!(count, 1);
    }
}
