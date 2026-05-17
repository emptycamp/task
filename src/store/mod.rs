pub mod codec;
pub mod gc;
pub mod revert;
pub mod tasks;

use crate::error::{Error, Result};
use crate::model::{Task, TaskId};
use crate::store::revert::{MetaDb, RevertDb, RevertOp};
use crate::store::tasks::TasksDb;
use directories::ProjectDirs;
use heed::{Env, EnvOpenOptions};
use std::path::{Path, PathBuf};

pub struct Store {
    env: Env,
    tasks_db: TasksDb,
    revert_db: RevertDb,
    meta_db: MetaDb,
}

impl Store {
    pub fn open(path: &Path) -> Result<Self> {
        std::fs::create_dir_all(path).map_err(Error::Io)?;
        let env = unsafe {
            EnvOpenOptions::new()
                .map_size(64 * 1024 * 1024)
                .max_dbs(4)
                .open(path)
                .map_err(Error::Db)?
        };

        let mut txn = env.write_txn()?;
        let tasks_db = env.create_database(&mut txn, Some("tasks"))?;
        let revert_db = env.create_database(&mut txn, Some("revert"))?;
        let meta_db = env.create_database(&mut txn, Some("meta"))?;
        txn.commit()?;

        Ok(Self {
            env,
            tasks_db,
            revert_db,
            meta_db,
        })
    }

    pub fn default_path(test_mode: bool) -> PathBuf {
        if let Ok(dir) = std::env::var("TASK_DATA_DIR") {
            return PathBuf::from(dir).join("db");
        }
        let name = if test_mode { "task-test" } else { "task" };
        ProjectDirs::from("", "", name)
            .expect("could not determine data directory")
            .data_dir()
            .join("db")
    }

    pub fn all_tasks(&self) -> Result<Vec<Task>> {
        let txn = self.env.read_txn()?;
        tasks::all(&txn, self.tasks_db)
    }

    pub fn get_task(&self, id: TaskId) -> Result<Task> {
        let txn = self.env.read_txn()?;
        tasks::get(&txn, self.tasks_db, id)
    }

    pub fn add_task(&mut self, task: Task) -> Result<Task> {
        let mut txn = self.env.write_txn()?;
        tasks::put(&mut txn, self.tasks_db, &task)?;
        txn.commit()?;
        Ok(task)
    }

    pub fn add_task_with_revert(&mut self, task: Task) -> Result<Task> {
        let mut txn = self.env.write_txn()?;
        tasks::put(&mut txn, self.tasks_db, &task)?;
        revert::push(&mut txn, self.revert_db, self.meta_db, RevertOp::Added { id: task.id })?;
        txn.commit()?;
        Ok(task)
    }

    pub fn next_id(&self) -> Result<TaskId> {
        let txn = self.env.read_txn()?;
        tasks::next_id(&txn, self.tasks_db)
    }

    pub fn update_task(&mut self, task: Task) -> Result<()> {
        let mut txn = self.env.write_txn()?;
        tasks::put(&mut txn, self.tasks_db, &task)?;
        txn.commit()?;
        Ok(())
    }

    pub fn update_task_with_revert(&mut self, before: Task, after: Task) -> Result<()> {
        let mut txn = self.env.write_txn()?;
        tasks::put(&mut txn, self.tasks_db, &after)?;
        revert::push(&mut txn, self.revert_db, self.meta_db, RevertOp::Edited { before })?;
        txn.commit()?;
        Ok(())
    }

    pub fn soft_delete_task_with_revert(&mut self, before: Task, after: Task) -> Result<()> {
        let mut txn = self.env.write_txn()?;
        tasks::put(&mut txn, self.tasks_db, &after)?;
        revert::push(&mut txn, self.revert_db, self.meta_db, RevertOp::Deleted { before })?;
        txn.commit()?;
        Ok(())
    }

    pub fn complete_task_with_revert(&mut self, before: Task, after: Task) -> Result<()> {
        let mut txn = self.env.write_txn()?;
        tasks::put(&mut txn, self.tasks_db, &after)?;
        revert::push(&mut txn, self.revert_db, self.meta_db, RevertOp::Completed { before })?;
        txn.commit()?;
        Ok(())
    }

    pub fn hard_delete(&mut self, id: TaskId) -> Result<()> {
        let mut txn = self.env.write_txn()?;
        tasks::delete(&mut txn, self.tasks_db, id)?;
        txn.commit()?;
        Ok(())
    }

    pub fn pop_revert(&mut self) -> Result<Option<RevertOp>> {
        let mut txn = self.env.write_txn()?;
        let op = revert::pop(&mut txn, self.revert_db, self.meta_db)?;
        txn.commit()?;
        Ok(op)
    }

    pub fn peek_revert(&self) -> Result<Option<RevertOp>> {
        let txn = self.env.read_txn()?;
        revert::peek(&txn, self.revert_db)
    }

    pub fn apply_revert_op(&mut self, op: RevertOp) -> Result<()> {
        match op {
            RevertOp::Added { id } => {
                self.hard_delete(id)?;
            }
            RevertOp::Edited { before } => {
                self.update_task(before)?;
            }
            RevertOp::Deleted { mut before } => {
                before.status = crate::model::Status::Active;
                before.deleted_at = None;
                self.update_task(before)?;
            }
            RevertOp::Completed { mut before } => {
                before.status = crate::model::Status::Active;
                before.completed_at = None;
                self.update_task(before)?;
            }
        }
        Ok(())
    }
}
