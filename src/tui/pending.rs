use crate::clock::Clock;
use crate::editor::EditorLauncher;
use crate::error::Result;
use crate::model::TaskId;
use crate::store::Store;
use crate::tui::events::PendingChange;
use std::collections::HashMap;

pub fn apply(
    pending: &HashMap<TaskId, Vec<PendingChange>>,
    store: &mut Store,
    clock: &dyn Clock,
    editor: &dyn EditorLauncher,
) -> Result<()> {
    for (id, changes) in pending {
        let id = *id;

        // Apply in order: SetPriority → EditFromEditor → ToggleComplete → ToggleDelete
        for change in changes.iter().filter(|c| matches!(c, PendingChange::SetPriority(_, _))) {
            if let PendingChange::SetPriority(_, priority) = change {
                let task = store.get_task(id)?;
                let mut updated = task.clone();
                updated.priority = *priority;
                store.update_task_with_revert(task, updated)?;
            }
        }

        for _change in changes.iter().filter(|c| matches!(c, PendingChange::EditFromEditor(_))) {
            crate::commands::edit::run(id, &[], store, clock, editor)?;
        }

        for _change in changes.iter().filter(|c| matches!(c, PendingChange::ToggleComplete(_))) {
            crate::commands::complete::run(id, store, clock)?;
        }

        for _change in changes.iter().filter(|c| matches!(c, PendingChange::ToggleDelete(_))) {
            crate::commands::delete::run(id, store, clock)?;
        }
    }
    Ok(())
}
