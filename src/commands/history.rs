use crate::confirm::Prompt;
use crate::error::{Error, Result};
use crate::format::{format_history, RenderOptions};
use crate::store::revert::HistoryEntry;
use crate::store::Store;

pub fn list(store: &Store, opts: &RenderOptions) -> Result<String> {
    let entries = store.history()?;
    Ok(format_history(&entries, opts))
}

/// Result of a cascade revert: every event that was rolled back, newest first.
pub type RevertSummary = Vec<(u64, String)>;

/// Revert event `from_id` and every event newer than it, in newest-first order.
///
/// History events are layered: a later event was applied on top of earlier state. To
/// undo an older event cleanly we have to first undo every newer event, otherwise we'd
/// be reverting a task to a state it was never in. The function asks for confirmation
/// once (showing the full cascade), then applies the reverts.
pub fn revert(
    from_id: u64,
    yes: bool,
    store: &mut Store,
    prompt: &dyn Prompt,
) -> Result<RevertSummary> {
    let cascade = collect_cascade(store, from_id)?;

    if !yes && !prompt.confirm(&confirm_message(&cascade))? {
        return Err(Error::Cancelled);
    }

    let mut summaries = Vec::with_capacity(cascade.len());
    for (id, entry) in &cascade {
        let summary = entry.op.summary();
        store.history_revert(*id)?;
        summaries.push((*id, summary));
    }
    Ok(summaries)
}

/// Collect every history entry with `id >= from_id`, newest-first. Returns an error
/// if the target id doesn't exist.
pub fn collect_cascade(store: &Store, from_id: u64) -> Result<Vec<(u64, HistoryEntry)>> {
    let mut entries = store.history()?;
    if !entries.iter().any(|(id, _)| *id == from_id) {
        return Err(Error::HistoryNotFound(from_id));
    }
    entries.retain(|(id, _)| *id >= from_id);
    entries.sort_by(|a, b| b.0.cmp(&a.0));
    Ok(entries)
}

fn confirm_message(cascade: &[(u64, HistoryEntry)]) -> String {
    if cascade.len() == 1 {
        let (id, e) = &cascade[0];
        return format!("Revert event #{id} ({})?", e.op.summary());
    }
    let mut msg = format!(
        "Reverting an older event also rolls back every newer event.\n\
         This will revert {} events (newest first):\n",
        cascade.len()
    );
    for (id, e) in cascade {
        msg.push_str(&format!("  #{id}  {}\n", e.op.summary()));
    }
    msg.push_str("Continue?");
    msg
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::FakeClock;
    use crate::confirm::AutoConfirm;
    use crate::model::{Priority, Status, Task};
    use chrono::{TimeZone, Utc};
    use tempfile::tempdir;

    fn make_clock() -> FakeClock {
        FakeClock::new(Utc.with_ymd_and_hms(2026, 5, 17, 12, 0, 0).unwrap())
    }

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
    fn revert_unknown_event_id_errors() {
        let dir = tempdir().unwrap();
        let mut store = Store::open(dir.path()).unwrap();
        assert!(matches!(
            revert(999, true, &mut store, &AutoConfirm),
            Err(Error::HistoryNotFound(999))
        ));
    }

    #[test]
    fn revert_latest_only_reverts_one_event() {
        let dir = tempdir().unwrap();
        let mut store = Store::open(dir.path()).unwrap();
        let clock = make_clock();
        store.add_task_with_revert(make_task(1), &clock).unwrap();
        store.add_task_with_revert(make_task(2), &clock).unwrap();

        let entries = store.history().unwrap();
        let latest_id = entries.iter().map(|(id, _)| *id).max().unwrap();
        let result = revert(latest_id, true, &mut store, &AutoConfirm).unwrap();
        assert_eq!(result.len(), 1);
        // The newer task is gone; the older one survives.
        assert!(store.get_task(2).is_err());
        assert!(store.get_task(1).is_ok());
    }

    #[test]
    fn revert_older_event_cascades_to_all_newer_events() {
        let dir = tempdir().unwrap();
        let mut store = Store::open(dir.path()).unwrap();
        let clock = make_clock();
        store.add_task_with_revert(make_task(1), &clock).unwrap();
        store.add_task_with_revert(make_task(2), &clock).unwrap();
        store.add_task_with_revert(make_task(3), &clock).unwrap();

        let entries = store.history().unwrap();
        let oldest_id = entries.iter().map(|(id, _)| *id).min().unwrap();

        let result = revert(oldest_id, true, &mut store, &AutoConfirm).unwrap();
        assert_eq!(result.len(), 3, "expected the full cascade to revert");
        // All three tasks gone, history empty
        assert!(store.get_task(1).is_err());
        assert!(store.get_task(2).is_err());
        assert!(store.get_task(3).is_err());
        assert!(store.history().unwrap().is_empty());
    }

    #[test]
    fn revert_middle_event_cascades_to_newer_but_not_older() {
        let dir = tempdir().unwrap();
        let mut store = Store::open(dir.path()).unwrap();
        let clock = make_clock();
        store.add_task_with_revert(make_task(1), &clock).unwrap();
        store.add_task_with_revert(make_task(2), &clock).unwrap();
        store.add_task_with_revert(make_task(3), &clock).unwrap();

        let mut entries = store.history().unwrap();
        entries.sort_by_key(|(id, _)| *id);
        let middle_id = entries[1].0;

        let result = revert(middle_id, true, &mut store, &AutoConfirm).unwrap();
        assert_eq!(result.len(), 2, "expected middle + newest to be reverted");
        // Oldest task survives, middle and newer are gone.
        assert!(store.get_task(1).is_ok());
        assert!(store.get_task(2).is_err());
        assert!(store.get_task(3).is_err());
    }

    #[test]
    fn confirm_message_single_event_is_concise() {
        let entry = HistoryEntry {
            op: crate::store::revert::RevertOp::Added { id: 5 },
            timestamp: Utc::now(),
        };
        let msg = confirm_message(&[(7, entry)]);
        assert!(msg.contains("#7"));
        assert!(msg.contains("added #5"));
    }

    #[test]
    fn confirm_message_cascade_lists_every_event() {
        let now = Utc::now();
        let entries = vec![
            (
                9,
                HistoryEntry {
                    op: crate::store::revert::RevertOp::Added { id: 3 },
                    timestamp: now,
                },
            ),
            (
                8,
                HistoryEntry {
                    op: crate::store::revert::RevertOp::Added { id: 2 },
                    timestamp: now,
                },
            ),
            (
                7,
                HistoryEntry {
                    op: crate::store::revert::RevertOp::Added { id: 1 },
                    timestamp: now,
                },
            ),
        ];
        let msg = confirm_message(&entries);
        assert!(msg.contains("3 events"));
        assert!(msg.contains("#9"));
        assert!(msg.contains("#8"));
        assert!(msg.contains("#7"));
        assert!(msg.contains("older event"));
    }
}
