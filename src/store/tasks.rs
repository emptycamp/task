use crate::error::{Error, Result};
use crate::model::{Task, TaskId};
use crate::store::codec::Bincode;
use heed::types::U32;
use heed::{Database, RwTxn};

pub type TasksDb = Database<U32<heed::byteorder::BigEndian>, Bincode<Task>>;

pub fn next_id(txn: &heed::RoTxn<'_>, db: TasksDb) -> Result<TaskId> {
    let mut expected: u32 = 1;
    for result in db.iter(txn)? {
        let (key, _) = result?;
        if key != expected {
            return Ok(expected);
        }
        expected += 1;
    }
    Ok(expected)
}

pub fn put(txn: &mut RwTxn<'_>, db: TasksDb, task: &Task) -> Result<()> {
    db.put(txn, &task.id, task).map_err(Error::Db)
}

pub fn get(txn: &heed::RoTxn<'_>, db: TasksDb, id: TaskId) -> Result<Task> {
    db.get(txn, &id)?.ok_or(Error::NotFound(id))
}

pub fn delete(txn: &mut RwTxn<'_>, db: TasksDb, id: TaskId) -> Result<bool> {
    Ok(db.delete(txn, &id)?)
}

pub fn all(txn: &heed::RoTxn<'_>, db: TasksDb) -> Result<Vec<Task>> {
    db.iter(txn)?
        .map(|r| r.map(|(_, t)| t).map_err(Error::Db))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Priority, Status, Task};
    use crate::store::Store;
    use chrono::Utc;
    use tempfile::tempdir;

    fn make_task(id: TaskId) -> Task {
        Task {
            id,
            text: format!("task {id}"),
            priority: Priority::B,
            due: Utc::now(),
            est_secs: 1800,
            status: Status::Active,
            created_at: Utc::now(),
            completed_at: None,
            deleted_at: None,
        }
    }

    #[test]
    fn next_id_starts_at_one_when_empty() {
        let dir = tempdir().unwrap();
        let mut store = Store::open(dir.path()).unwrap();
        let id = store.next_id().unwrap();
        assert_eq!(id, 1);
    }

    #[test]
    fn next_id_reuses_lowest_gap() {
        let dir = tempdir().unwrap();
        let mut store = Store::open(dir.path()).unwrap();
        // Insert 1, 2, 4 (skipping 3) — lowest free ID should be 3
        store.add_task(make_task(1)).unwrap();
        store.add_task(make_task(2)).unwrap();
        store.add_task(make_task(4)).unwrap();
        let id = store.next_id().unwrap();
        assert_eq!(id, 3);
    }

    #[test]
    fn next_id_extends_when_no_gap() {
        let dir = tempdir().unwrap();
        let mut store = Store::open(dir.path()).unwrap();
        store.add_task(make_task(1)).unwrap();
        store.add_task(make_task(2)).unwrap();
        store.add_task(make_task(3)).unwrap();
        let id = store.next_id().unwrap();
        assert_eq!(id, 4);
    }
}
