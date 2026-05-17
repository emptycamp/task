use crate::clock::Clock;
use crate::editor::EditorLauncher;
use crate::error::{Error, Result};
use crate::model::TaskId;
use crate::store::Store;
use crate::time::{parse_due, parse_duration};
use crate::yaml::{from_yaml, to_yaml};
use chrono::Local;
use std::io::{Read, Write};

pub fn run(
    id: TaskId,
    args: &[String],
    store: &mut Store,
    clock: &dyn Clock,
    editor: &dyn EditorLauncher,
) -> Result<()> {
    let task = store.get_task(id)?;

    if args.is_empty() {
        return run_editor(id, store, editor);
    }

    let now_utc = clock.now();
    let now_local: chrono::DateTime<Local> = now_utc.into();

    let mut updated = task.clone();
    let mut text_parts: Vec<String> = Vec::new();

    for arg in args {
        if let Some(rest) = arg.strip_prefix("p:") {
            updated.priority = rest
                .parse()
                .map_err(|e: String| Error::Parse(e))?;
        } else if let Some(rest) = arg.strip_prefix("due:") {
            updated.due = parse_due(rest, now_local)?.with_timezone(&chrono::Utc);
        } else if let Some(rest) = arg.strip_prefix("est:") {
            updated.est_secs = parse_duration(rest)?.num_seconds();
        } else {
            text_parts.push(arg.clone());
        }
    }

    if !text_parts.is_empty() {
        updated.text = text_parts.join(" ");
    }

    if updated == task {
        return Ok(());
    }

    store.update_task_with_revert(task, updated)
}

fn run_editor(id: TaskId, store: &mut Store, editor: &dyn EditorLauncher) -> Result<()> {
    let task = store.get_task(id)?;
    let yaml = to_yaml(&task)?;

    let mut tmp = tempfile::Builder::new()
        .suffix(".yaml")
        .tempfile()
        .map_err(Error::Io)?;

    tmp.write_all(yaml.as_bytes()).map_err(Error::Io)?;
    tmp.flush().map_err(Error::Io)?;

    let path = tmp.path().to_path_buf();
    editor.launch(&path)?;

    let mut content = String::new();
    std::fs::File::open(&path)
        .map_err(Error::Io)?
        .read_to_string(&mut content)
        .map_err(Error::Io)?;

    match from_yaml(&content, &task) {
        Ok(updated) => {
            if updated == task {
                return Ok(());
            }
            store.update_task_with_revert(task, updated)
        }
        Err(e) => {
            let error_yaml = format!("# ERROR: {e}\n{content}");
            std::fs::write(&path, &error_yaml).map_err(Error::Io)?;
            editor.launch(&path)?;

            let mut content2 = String::new();
            std::fs::File::open(&path)
                .map_err(Error::Io)?
                .read_to_string(&mut content2)
                .map_err(Error::Io)?;

            let updated = from_yaml(&content2, &task)?;
            if updated == task {
                return Ok(());
            }
            store.update_task_with_revert(task, updated)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::FakeClock;
    use crate::model::{Priority, Status, Task};
    use chrono::{TimeZone, Utc};
    use std::path::Path;
    use tempfile::tempdir;

    fn make_clock() -> FakeClock {
        FakeClock::new(Utc.with_ymd_and_hms(2026, 5, 17, 12, 0, 0).unwrap())
    }

    fn make_task(id: u32) -> Task {
        Task {
            id,
            text: "original".to_string(),
            priority: Priority::B,
            due: Utc::now(),
            est_secs: 1800,
            status: Status::Active,
            created_at: Utc::now(),
            completed_at: None,
            deleted_at: None,
        }
    }

    struct NoOpEditor;
    impl EditorLauncher for NoOpEditor {
        fn launch(&self, _path: &Path) -> crate::error::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn edit_priority_via_args() {
        let dir = tempdir().unwrap();
        let mut store = Store::open(dir.path()).unwrap();
        store.add_task(make_task(1)).unwrap();
        let clock = make_clock();
        run(1, &["p:a".to_string()], &mut store, &clock, &NoOpEditor).unwrap();
        let updated = store.get_task(1).unwrap();
        assert_eq!(updated.priority, Priority::A);
    }

    #[test]
    fn edit_text_via_args() {
        let dir = tempdir().unwrap();
        let mut store = Store::open(dir.path()).unwrap();
        store.add_task(make_task(1)).unwrap();
        let clock = make_clock();
        run(1, &["new text".to_string()], &mut store, &clock, &NoOpEditor).unwrap();
        let updated = store.get_task(1).unwrap();
        assert_eq!(updated.text, "new text");
    }

    #[test]
    fn edit_nonexistent_task_returns_error() {
        let dir = tempdir().unwrap();
        let mut store = Store::open(dir.path()).unwrap();
        let clock = make_clock();
        assert!(run(99, &["p:a".to_string()], &mut store, &clock, &NoOpEditor).is_err());
    }
}
