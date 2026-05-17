use task::clock::FakeClock;
use task::model::{Priority, Status, Task};
use task::store::Store;
use task::store::gc::sweep;
use chrono::{Duration, TimeZone, Utc};
use tempfile::tempdir;

fn make_task(id: u32, priority: Priority, status: Status, created_at: chrono::DateTime<Utc>) -> Task {
    Task {
        id,
        text: format!("gc task {id}"),
        priority,
        due: created_at,
        est_secs: 1800,
        status,
        created_at,
        completed_at: if status == Status::Completed { Some(created_at) } else { None },
        deleted_at: if status == Status::SoftDeleted { Some(created_at) } else { None },
    }
}

fn base() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap()
}

#[test]
fn gc_priority_c_deleted_after_3_days() {
    let dir = tempdir().unwrap();
    let mut store = Store::open(dir.path()).unwrap();
    store.add_task(make_task(1, Priority::C, Status::Active, base())).unwrap();
    let count = sweep(&mut store, &FakeClock::new(base() + Duration::days(4))).unwrap();
    assert_eq!(count, 1);
}

#[test]
fn gc_priority_b_survives_6_days() {
    let dir = tempdir().unwrap();
    let mut store = Store::open(dir.path()).unwrap();
    store.add_task(make_task(1, Priority::B, Status::Active, base())).unwrap();
    let count = sweep(&mut store, &FakeClock::new(base() + Duration::days(6))).unwrap();
    assert_eq!(count, 0);
}

#[test]
fn gc_priority_a_never_deleted() {
    let dir = tempdir().unwrap();
    let mut store = Store::open(dir.path()).unwrap();
    store.add_task(make_task(1, Priority::A, Status::Active, base())).unwrap();
    let count = sweep(&mut store, &FakeClock::new(base() + Duration::days(365))).unwrap();
    assert_eq!(count, 0);
}

#[test]
fn gc_completed_deleted_after_14_days() {
    let dir = tempdir().unwrap();
    let mut store = Store::open(dir.path()).unwrap();
    store.add_task(make_task(1, Priority::B, Status::Completed, base())).unwrap();
    let count = sweep(&mut store, &FakeClock::new(base() + Duration::days(15))).unwrap();
    assert_eq!(count, 1);
}

#[test]
fn gc_soft_deleted_removed_after_1_day() {
    let dir = tempdir().unwrap();
    let mut store = Store::open(dir.path()).unwrap();
    store.add_task(make_task(1, Priority::B, Status::SoftDeleted, base())).unwrap();
    let count = sweep(&mut store, &FakeClock::new(base() + Duration::days(2))).unwrap();
    assert_eq!(count, 1);
}
