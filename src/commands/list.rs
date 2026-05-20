use crate::error::Result;
use crate::format::{format_list, ListMode, RenderOptions};
use crate::model::{Status, Task};
use crate::store::Store;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Filter {
    Active,
    Completed,
    Deleted,
    All,
}

/// What the user explicitly asked for on the command line.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct FilterChoice {
    pub filter: Filter,
    /// True if the user passed any of --active/--completed/--deleted/--all explicitly.
    pub explicit: bool,
}

pub fn resolve_filter(active: bool, completed: bool, deleted: bool, all: bool) -> FilterChoice {
    if all {
        FilterChoice { filter: Filter::All, explicit: true }
    } else if completed {
        FilterChoice { filter: Filter::Completed, explicit: true }
    } else if deleted {
        FilterChoice { filter: Filter::Deleted, explicit: true }
    } else if active {
        FilterChoice { filter: Filter::Active, explicit: true }
    } else {
        FilterChoice { filter: Filter::Active, explicit: false }
    }
}

pub fn run(store: &Store, choice: FilterChoice, opts: &RenderOptions) -> Result<(String, u32)> {
    run_with_gc_count(store, choice, opts, 0)
}

pub fn run_with_gc_count(
    store: &Store,
    choice: FilterChoice,
    opts: &RenderOptions,
    gc_count: u32,
) -> Result<(String, u32)> {
    let tasks = store.all_tasks()?;

    let filtered: Vec<Task> = tasks
        .into_iter()
        .filter(|t| matches_filter(t.status, choice.filter))
        .collect();

    // Compact view only applies to the implicit default. Any explicit flag (including
    // --active) shows the full list.
    let mode = if choice.explicit { ListMode::Full } else { ListMode::Compact };
    let output = format_list(&filtered, opts, mode);
    Ok((output, gc_count))
}

fn matches_filter(status: Status, filter: Filter) -> bool {
    match filter {
        Filter::Active => status == Status::Active,
        Filter::Completed => status == Status::Completed,
        Filter::Deleted => status == Status::SoftDeleted,
        Filter::All => true,
    }
}

pub fn format_with_footer(output: &str, gc_count: u32) -> String {
    if gc_count > 0 {
        format!("{output}  ({gc_count} task{} aged out)\n", if gc_count == 1 { "" } else { "s" })
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

    fn default_choice() -> FilterChoice {
        FilterChoice { filter: Filter::Active, explicit: false }
    }

    #[test]
    fn list_active_only_by_default() {
        let dir = tempdir().unwrap();
        let mut store = Store::open(dir.path()).unwrap();
        store.add_task(make_task(1, Status::Active)).unwrap();
        store.add_task(make_task(2, Status::Completed)).unwrap();

        let opts = RenderOptions::no_color();
        let (output, _) = run(&store, default_choice(), &opts).unwrap();
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
        let choice = FilterChoice { filter: Filter::Completed, explicit: true };
        let (output, _) = run(&store, choice, &opts).unwrap();
        assert!(!output.contains("task 1"));
        assert!(output.contains("task 2"));
    }

    #[test]
    fn list_deleted_shows_only_deleted() {
        let dir = tempdir().unwrap();
        let mut store = Store::open(dir.path()).unwrap();
        store.add_task(make_task(1, Status::Active)).unwrap();
        store.add_task(make_task(2, Status::SoftDeleted)).unwrap();

        let opts = RenderOptions::no_color();
        let choice = FilterChoice { filter: Filter::Deleted, explicit: true };
        let (output, _) = run(&store, choice, &opts).unwrap();
        assert!(!output.contains("task 1"));
        assert!(output.contains("task 2"));
    }

    #[test]
    fn list_all_shows_every_status() {
        let dir = tempdir().unwrap();
        let mut store = Store::open(dir.path()).unwrap();
        store.add_task(make_task(1, Status::Active)).unwrap();
        store.add_task(make_task(2, Status::Completed)).unwrap();
        store.add_task(make_task(3, Status::SoftDeleted)).unwrap();

        let opts = RenderOptions::no_color();
        let choice = FilterChoice { filter: Filter::All, explicit: true };
        let (output, _) = run(&store, choice, &opts).unwrap();
        assert!(output.contains("task 1"));
        assert!(output.contains("task 2"));
        assert!(output.contains("task 3"));
    }

    #[test]
    fn list_default_compact_truncates_to_4_per_day() {
        let dir = tempdir().unwrap();
        let mut store = Store::open(dir.path()).unwrap();
        for i in 1..=6 {
            store.add_task(make_task(i, Status::Active)).unwrap();
        }

        let opts = RenderOptions::no_color();
        let (output, _) = run(&store, default_choice(), &opts).unwrap();
        // Two tasks should be hidden under the +2 marker.
        assert!(output.contains("+2"));
    }

    #[test]
    fn list_explicit_active_does_not_truncate() {
        let dir = tempdir().unwrap();
        let mut store = Store::open(dir.path()).unwrap();
        for i in 1..=6 {
            store.add_task(make_task(i, Status::Active)).unwrap();
        }

        let opts = RenderOptions::no_color();
        let choice = FilterChoice { filter: Filter::Active, explicit: true };
        let (output, _) = run(&store, choice, &opts).unwrap();
        assert!(!output.contains("+"));
    }

    #[test]
    fn gc_footer_appears_when_count_nonzero() {
        let out = format_with_footer("tasks\n", 3);
        assert!(out.contains("3 tasks aged out"));
    }

    #[test]
    fn gc_footer_absent_when_count_zero() {
        let out = format_with_footer("tasks\n", 0);
        assert!(!out.contains("aged out"));
    }

    #[test]
    fn resolve_filter_default_is_implicit_active() {
        let r = resolve_filter(false, false, false, false);
        assert_eq!(r.filter, Filter::Active);
        assert!(!r.explicit);
    }

    #[test]
    fn resolve_filter_active_flag_is_explicit() {
        let r = resolve_filter(true, false, false, false);
        assert_eq!(r.filter, Filter::Active);
        assert!(r.explicit);
    }

    #[test]
    fn resolve_filter_completed_flag() {
        let r = resolve_filter(false, true, false, false);
        assert_eq!(r.filter, Filter::Completed);
        assert!(r.explicit);
    }

    #[test]
    fn resolve_filter_deleted_flag() {
        let r = resolve_filter(false, false, true, false);
        assert_eq!(r.filter, Filter::Deleted);
        assert!(r.explicit);
    }

    #[test]
    fn resolve_filter_all_flag() {
        let r = resolve_filter(false, false, false, true);
        assert_eq!(r.filter, Filter::All);
        assert!(r.explicit);
    }
}
