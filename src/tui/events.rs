use crate::model::{Priority, TaskId};
use crate::tui::App;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[derive(Debug, Clone, PartialEq)]
pub enum PendingChange {
    ToggleComplete(TaskId),
    ToggleDelete(TaskId),
    SetPriority(TaskId, Priority),
    EditFromEditor(TaskId),
}

pub fn handle(app: &mut App, key: KeyEvent) -> bool {
    match (key.code, key.modifiers) {
        (KeyCode::Up, _) => {
            app.cursor = app.cursor.saturating_sub(1);
        }
        (KeyCode::Down, _) => {
            if app.cursor + 1 < app.tasks.len() {
                app.cursor += 1;
            }
        }
        (KeyCode::Char('c'), KeyModifiers::NONE) => {
            if let Some(task) = app.tasks.get(app.cursor) {
                toggle_change(app, PendingChange::ToggleComplete(task.id));
            }
        }
        (KeyCode::Char('d'), KeyModifiers::NONE) => {
            if let Some(task) = app.tasks.get(app.cursor) {
                toggle_change(app, PendingChange::ToggleDelete(task.id));
            }
        }
        (KeyCode::Char('e'), KeyModifiers::NONE) | (KeyCode::Enter, _) => {
            if let Some(task) = app.tasks.get(app.cursor) {
                let id = task.id;
                toggle_change(app, PendingChange::EditFromEditor(id));
            }
        }
        (KeyCode::Char('A'), KeyModifiers::SHIFT) => {
            if let Some(task) = app.tasks.get(app.cursor) {
                set_priority(app, task.id, Priority::A);
            }
        }
        (KeyCode::Char('B'), KeyModifiers::SHIFT) => {
            if let Some(task) = app.tasks.get(app.cursor) {
                set_priority(app, task.id, Priority::B);
            }
        }
        (KeyCode::Char('C'), KeyModifiers::SHIFT) => {
            if let Some(task) = app.tasks.get(app.cursor) {
                set_priority(app, task.id, Priority::C);
            }
        }
        (KeyCode::Esc, _) | (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
            return true;
        }
        _ => {}
    }
    false
}

fn toggle_change(app: &mut App, change: PendingChange) {
    let id = match &change {
        PendingChange::ToggleComplete(id) => *id,
        PendingChange::ToggleDelete(id) => *id,
        PendingChange::EditFromEditor(id) => *id,
        PendingChange::SetPriority(id, _) => *id,
    };
    let changes = app.pending.entry(id).or_default();
    if let Some(pos) = changes.iter().position(|c| c == &change) {
        changes.remove(pos);
    } else {
        changes.push(change);
    }
}

fn set_priority(app: &mut App, id: TaskId, p: Priority) {
    let changes = app.pending.entry(id).or_default();
    changes.retain(|c| !matches!(c, PendingChange::SetPriority(_, _)));
    changes.push(PendingChange::SetPriority(id, p));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Priority, Status, Task};
    use crate::tui::App;
    use chrono::Utc;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

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

    fn make_app() -> App {
        App {
            tasks: vec![make_task(1), make_task(2)],
            cursor: 0,
            pending: std::collections::HashMap::new(),
            should_quit: false,
        }
    }

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn shift_key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::SHIFT)
    }

    #[test]
    fn up_moves_cursor_up_clamped() {
        let mut app = make_app();
        handle(&mut app, key(KeyCode::Up));
        assert_eq!(app.cursor, 0);
    }

    #[test]
    fn down_moves_cursor_down() {
        let mut app = make_app();
        handle(&mut app, key(KeyCode::Down));
        assert_eq!(app.cursor, 1);
    }

    #[test]
    fn down_clamped_at_last_item() {
        let mut app = make_app();
        app.cursor = 1;
        handle(&mut app, key(KeyCode::Down));
        assert_eq!(app.cursor, 1);
    }

    #[test]
    fn c_key_adds_toggle_complete_pending() {
        let mut app = make_app();
        handle(&mut app, key(KeyCode::Char('c')));
        let changes = app.pending.get(&1).unwrap();
        assert!(changes.contains(&PendingChange::ToggleComplete(1)));
    }

    #[test]
    fn c_key_twice_removes_pending() {
        let mut app = make_app();
        handle(&mut app, key(KeyCode::Char('c')));
        handle(&mut app, key(KeyCode::Char('c')));
        let changes = app.pending.get(&1).map(|v| v.len()).unwrap_or(0);
        assert_eq!(changes, 0);
    }

    #[test]
    fn d_key_adds_toggle_delete_pending() {
        let mut app = make_app();
        handle(&mut app, key(KeyCode::Char('d')));
        let changes = app.pending.get(&1).unwrap();
        assert!(changes.contains(&PendingChange::ToggleDelete(1)));
    }

    #[test]
    fn shift_a_sets_priority_a() {
        let mut app = make_app();
        handle(&mut app, shift_key(KeyCode::Char('A')));
        let changes = app.pending.get(&1).unwrap();
        assert!(changes.contains(&PendingChange::SetPriority(1, Priority::A)));
    }

    #[test]
    fn shift_b_replaces_existing_priority() {
        let mut app = make_app();
        handle(&mut app, shift_key(KeyCode::Char('A')));
        handle(&mut app, shift_key(KeyCode::Char('B')));
        let changes = app.pending.get(&1).unwrap();
        assert!(!changes.contains(&PendingChange::SetPriority(1, Priority::A)));
        assert!(changes.contains(&PendingChange::SetPriority(1, Priority::B)));
    }

    #[test]
    fn esc_signals_quit() {
        let mut app = make_app();
        let quit = handle(&mut app, key(KeyCode::Esc));
        assert!(quit);
    }

    #[test]
    fn ctrl_c_signals_quit() {
        let mut app = make_app();
        let quit = handle(
            &mut app,
            KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
        );
        assert!(quit);
    }
}
