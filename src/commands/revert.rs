use crate::confirm::Prompt;
use crate::error::{Error, Result};
use crate::store::Store;

pub fn run(yes: bool, store: &mut Store, prompt: &dyn Prompt) -> Result<()> {
    let op = store.peek_revert()?;
    if op.is_none() {
        return Err(Error::NothingToRevert);
    }

    if !yes && !prompt.confirm("Revert last operation?")? {
        return Err(Error::Cancelled);
    }

    let op = store.pop_revert()?.ok_or(Error::NothingToRevert)?;
    store.apply_revert_op(op)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::confirm::AutoConfirm;
    use crate::model::{Priority, Status, Task};
    use chrono::Utc;
    use tempfile::tempdir;

    fn make_task(id: u32) -> Task {
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
    fn revert_nothing_returns_error() {
        let dir = tempdir().unwrap();
        let mut store = Store::open(dir.path()).unwrap();
        assert!(matches!(
            run(true, &mut store, &AutoConfirm),
            Err(Error::NothingToRevert)
        ));
    }

    #[test]
    fn revert_added_task_hard_deletes_it() {
        let dir = tempdir().unwrap();
        let mut store = Store::open(dir.path()).unwrap();
        store.add_task_with_revert(make_task(1)).unwrap();
        run(true, &mut store, &AutoConfirm).unwrap();
        assert!(store.get_task(1).is_err());
    }

    #[test]
    fn revert_twice_undoes_two_operations() {
        let dir = tempdir().unwrap();
        let mut store = Store::open(dir.path()).unwrap();
        store.add_task_with_revert(make_task(1)).unwrap();
        store.add_task_with_revert(make_task(2)).unwrap();
        run(true, &mut store, &AutoConfirm).unwrap();
        run(true, &mut store, &AutoConfirm).unwrap();
        assert!(store.get_task(1).is_err());
        assert!(store.get_task(2).is_err());
    }
}
