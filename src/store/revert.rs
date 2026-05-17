use crate::error::{Error, Result};
use crate::model::{Task, TaskId};
use crate::store::codec::Bincode;
use heed::types::U64;
use heed::{Database, RoTxn, RwTxn};
use serde::{Deserialize, Serialize};

pub type RevertDb = Database<U64<heed::byteorder::BigEndian>, Bincode<RevertOp>>;
pub type MetaDb = Database<heed::types::Str, U64<heed::byteorder::BigEndian>>;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum RevertOp {
    Added { id: TaskId },
    Edited { before: Task },
    Deleted { before: Task },
    Completed { before: Task },
}

const SEQ_KEY: &str = "revert_seq";

pub fn push(
    txn: &mut RwTxn<'_>,
    revert_db: RevertDb,
    meta_db: MetaDb,
    op: RevertOp,
) -> Result<()> {
    let seq = meta_db.get(txn, SEQ_KEY)?.unwrap_or(0) + 1;
    meta_db.put(txn, SEQ_KEY, &seq)?;
    revert_db.put(txn, &seq, &op).map_err(Error::Db)
}

pub fn pop(
    txn: &mut RwTxn<'_>,
    revert_db: RevertDb,
    meta_db: MetaDb,
) -> Result<Option<RevertOp>> {
    let Some((key, op)) = revert_db.last(txn)? else {
        return Ok(None);
    };
    revert_db.delete(txn, &key)?;
    // Update seq to the new last key (or 0 if empty)
    let new_seq = revert_db
        .last(txn)?
        .map(|(k, _)| k)
        .unwrap_or(0);
    meta_db.put(txn, SEQ_KEY, &new_seq)?;
    Ok(Some(op))
}

pub fn peek(txn: &RoTxn<'_>, revert_db: RevertDb) -> Result<Option<RevertOp>> {
    Ok(revert_db.last(txn)?.map(|(_, op)| op))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Priority, Status};
    use chrono::Utc;
    use heed::EnvOpenOptions;
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

    fn open_dbs(dir: &std::path::Path) -> (heed::Env, RevertDb, MetaDb) {
        let env = unsafe {
            EnvOpenOptions::new()
                .map_size(10 * 1024 * 1024)
                .max_dbs(4)
                .open(dir)
                .unwrap()
        };
        let mut txn = env.write_txn().unwrap();
        let revert_db = env.create_database(&mut txn, Some("revert")).unwrap();
        let meta_db = env.create_database(&mut txn, Some("meta")).unwrap();
        txn.commit().unwrap();
        (env, revert_db, meta_db)
    }

    #[test]
    fn push_then_pop_returns_lifo_order() {
        let dir = tempdir().unwrap();
        let (env, rdb, mdb) = open_dbs(dir.path());

        let op1 = RevertOp::Added { id: 1 };
        let op2 = RevertOp::Deleted { before: make_task(2) };

        let mut txn = env.write_txn().unwrap();
        push(&mut txn, rdb, mdb, op1).unwrap();
        push(&mut txn, rdb, mdb, op2).unwrap();
        txn.commit().unwrap();

        let mut txn = env.write_txn().unwrap();
        let popped = pop(&mut txn, rdb, mdb).unwrap().unwrap();
        assert!(matches!(popped, RevertOp::Deleted { .. }));

        let popped2 = pop(&mut txn, rdb, mdb).unwrap().unwrap();
        assert!(matches!(popped2, RevertOp::Added { .. }));
    }

    #[test]
    fn pop_on_empty_returns_none() {
        let dir = tempdir().unwrap();
        let (env, rdb, mdb) = open_dbs(dir.path());
        let mut txn = env.write_txn().unwrap();
        assert!(pop(&mut txn, rdb, mdb).unwrap().is_none());
    }
}
