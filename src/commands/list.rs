use crate::error::Result;
use crate::format::{format_list, RenderOptions};
use crate::model::{Status, Task};
use crate::store::Store;

pub fn run(store: &Store, all: bool, which: Option<&str>, opts: &RenderOptions) -> Result<(String, u32)> {
    run_with_gc_count(store, all, which, opts, 0)
}

pub fn run_with_gc_count(
    store: &Store,
    all: bool,
    which: Option<&str>,
    opts: &RenderOptions,
    gc_count: u32,
) -> Result<(String, u32)> {
    let tasks = store.all_tasks()?;

    let show_completed = which
        .map(|w| matches!(w, "complete" | "completed" | "done"))
        .unwrap_or(false);

    let filtered: Vec<Task> = tasks
        .into_iter()
        .filter(|t| {
            if show_completed {
                t.status == Status::Completed
            } else if all {
                t.status != Status::SoftDeleted
            } else {
                t.status == Status::Active
            }
        })
        .collect();

    let output = format_list(&filtered, opts);
    Ok((output, gc_count))
}

pub fn format_with_footer(output: &str, gc_count: u32) -> String {
    if gc_count > 0 {
        format!("{output}({gc_count} task{} aged out)\n", if gc_count == 1 { "" } else { "s" })
    } else {
        output.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Priority, Status, Task};
    use chrono::Utc;
    use tempfile::tempdir;

    fn make_task(id: u32, status: Status) -> Task {
        Task {
            id,
            text: format!("task {id}"),
            priority: Priority::B,
            due: Utc::now(),
            est_secs: 1800,
            status,
            created_at: Utc::now(),
            completed_at: None,
            deleted_at: None,
        }
    }

    #[test]
    fn list_active_only_by_default() {
        let dir = tempdir().unwrap();
        let mut store = Store::open(dir.path()).unwrap();
        store.add_task(make_task(1, Status::Active)).unwrap();
        store.add_task(make_task(2, Status::Completed)).unwrap();

        let opts = RenderOptions::no_color();
        let (output, _) = run(&store, false, None, &opts).unwrap();
        assert!(output.contains("task 1"));
        assert!(!output.contains("task 2"));
    }

    #[test]
    fn list_completed_shows_only_completed() {
        let dir = tempdir().unwrap();
        let mut store = Store::open(dir.path()).unwrap();
        store.add_task(make_task(1, Status::Active)).unwrap();
        store.add_task(make_task(2, Status::Completed)).unwrap();

        let opts = RenderOptions::no_color();
        let (output, _) = run(&store, false, Some("done"), &opts).unwrap();
        assert!(!output.contains("task 1"));
        assert!(output.contains("task 2"));
    }

    #[test]
    fn list_all_shows_active_and_completed() {
        let dir = tempdir().unwrap();
        let mut store = Store::open(dir.path()).unwrap();
        store.add_task(make_task(1, Status::Active)).unwrap();
        store.add_task(make_task(2, Status::Completed)).unwrap();

        let opts = RenderOptions::no_color();
        let (output, _) = run(&store, true, None, &opts).unwrap();
        assert!(output.contains("task 1"));
        assert!(output.contains("task 2"));
    }

    #[test]
    fn gc_footer_appears_when_count_nonzero() {
        let out = format_with_footer("tasks\n", 3);
        assert!(out.contains("3 tasks aged out"));
    }

    #[test]
    fn gc_footer_singular_when_count_is_one() {
        let out = format_with_footer("tasks\n", 1);
        assert!(out.contains("1 task aged out"));
    }

    #[test]
    fn gc_footer_absent_when_count_zero() {
        let out = format_with_footer("tasks\n", 0);
        assert!(!out.contains("aged out"));
    }
}
