pub mod events;
pub mod pending;
pub mod render;

use crate::clock::Clock;
use crate::editor::TaskEditor;
use crate::error::Result;
use crate::format::sort_key;
use chrono::Local;
use crate::model::{Priority, Status, Task, TaskId};
use crate::store::Store;
use crate::tui::events::{Action, PendingChange};
use chrono::Duration;
use crossterm::{
    event::{self, Event, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::collections::HashMap;
use std::io;

pub struct App {
    pub tasks: Vec<Task>,
    pub cursor: usize,
    pub pending: HashMap<TaskId, Vec<PendingChange>>,
    pub should_quit: bool,
}

impl App {
    fn new(tasks: Vec<Task>) -> Self {
        Self {
            tasks,
            cursor: 0,
            pending: HashMap::new(),
            should_quit: false,
        }
    }
}

pub fn run(store: &mut Store, clock: &dyn Clock, editor: &dyn TaskEditor) -> Result<()> {
    let mut app = App::new(load_active_tasks(store)?);

    enter_screen()?;
    let mut terminal = build_terminal()?;

    let result = run_loop(&mut terminal, &mut app, store, clock, editor);

    leave_screen(&mut terminal);

    result?;

    pending::apply(&app.pending, store, clock)?;
    Ok(())
}

fn load_active_tasks(store: &Store) -> Result<Vec<Task>> {
    let mut tasks: Vec<Task> = store
        .all_tasks()?
        .into_iter()
        .filter(|t| t.status == Status::Active)
        .collect();
    // Same canonical ordering as `task list`, so what the user sees in the TUI matches.
    let today = Local::now().date_naive();
    tasks.sort_by_key(|t| sort_key(t, today));
    Ok(tasks)
}

fn build_terminal() -> Result<Terminal<CrosstermBackend<io::Stdout>>> {
    let backend = CrosstermBackend::new(io::stdout());
    Terminal::new(backend).map_err(crate::error::Error::Io)
}

fn enter_screen() -> Result<()> {
    enable_raw_mode().map_err(crate::error::Error::Io)?;
    execute!(io::stdout(), EnterAlternateScreen).map_err(crate::error::Error::Io)?;
    Ok(())
}

fn leave_screen(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) {
    let _ = disable_raw_mode();
    let _ = execute!(terminal.backend_mut(), LeaveAlternateScreen);
    let _ = terminal.show_cursor();
}

fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    store: &mut Store,
    clock: &dyn Clock,
    editor: &dyn TaskEditor,
) -> Result<()> {
    loop {
        terminal
            .draw(|f| render::draw(f, app))
            .map_err(crate::error::Error::Io)?;

        let key = match event::read().map_err(crate::error::Error::Io)? {
            Event::Key(k) if k.kind == KeyEventKind::Press => k,
            _ => continue,
        };

        match events::handle(app, key) {
            Action::Continue => {}
            Action::Quit => return Ok(()),
            Action::EditTask(id) => {
                let edit_result = with_paused_terminal(terminal, || {
                    edit_existing(id, store, clock, editor)
                });
                edit_result?;
                refresh_tasks(app, store)?;
            }
            Action::AddTask => {
                let add_result = with_paused_terminal(terminal, || {
                    add_new(store, clock, editor)
                });
                add_result?;
                refresh_tasks(app, store)?;
            }
        }
    }
}

/// Pause the TUI (drop alt screen + raw mode), run `f`, then resume the TUI. We always
/// resume even if `f` errored so the user is never stranded.
fn with_paused_terminal<F, T>(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    f: F,
) -> Result<T>
where
    F: FnOnce() -> Result<T>,
{
    let _ = disable_raw_mode();
    let _ = execute!(terminal.backend_mut(), LeaveAlternateScreen);
    let _ = terminal.show_cursor();

    let result = f();

    let _ = enable_raw_mode();
    let _ = execute!(terminal.backend_mut(), EnterAlternateScreen);
    let _ = terminal.clear();

    result
}

fn edit_existing(
    id: TaskId,
    store: &mut Store,
    clock: &dyn Clock,
    editor: &dyn TaskEditor,
) -> Result<()> {
    let task = store.get_task(id)?;
    let mut baseline = task.clone();
    let mut save = |proposed: Task| -> Result<Task> {
        if proposed == baseline {
            return Ok(proposed);
        }
        store.update_task_with_revert(baseline.clone(), proposed.clone(), clock)?;
        baseline = proposed.clone();
        Ok(proposed)
    };
    editor.edit(&task, &mut save)
}

fn add_new(store: &mut Store, clock: &dyn Clock, editor: &dyn TaskEditor) -> Result<()> {
    let now = clock.now();
    let template = Task {
        id: 0,
        text: String::new(),
        priority: Priority::B,
        due: now + Duration::minutes(5),
        est_secs: 1800,
        status: Status::Active,
        created_at: now,
        completed_at: None,
        deleted_at: None,
    };
    let mut baseline: Option<Task> = None;
    let mut save = |proposed: Task| -> Result<Task> {
        match &baseline {
            None => {
                // First save — actually create the task with a real ID.
                let mut t = proposed;
                t.id = store.next_id()?;
                let created = store.add_task_with_revert(t, clock)?;
                baseline = Some(created.clone());
                Ok(created)
            }
            Some(prev) => {
                if &proposed == prev {
                    return Ok(proposed);
                }
                store.update_task_with_revert(prev.clone(), proposed.clone(), clock)?;
                baseline = Some(proposed.clone());
                Ok(proposed)
            }
        }
    };
    editor.edit(&template, &mut save)
}

fn refresh_tasks(app: &mut App, store: &Store) -> Result<()> {
    app.tasks = load_active_tasks(store)?;
    if app.tasks.is_empty() {
        app.cursor = 0;
    } else if app.cursor >= app.tasks.len() {
        app.cursor = app.tasks.len() - 1;
    }
    Ok(())
}
