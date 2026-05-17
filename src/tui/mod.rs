pub mod events;
pub mod pending;
pub mod render;

use crate::clock::Clock;
use crate::editor::EditorLauncher;
use crate::error::Result;
use crate::model::{Status, Task, TaskId};
use crate::store::Store;
use crate::tui::events::PendingChange;
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

pub fn run(store: &mut Store, clock: &dyn Clock, editor: &dyn EditorLauncher) -> Result<()> {
    let tasks: Vec<Task> = store
        .all_tasks()?
        .into_iter()
        .filter(|t| t.status == Status::Active)
        .collect();

    let mut app = App::new(tasks);

    enable_raw_mode().map_err(crate::error::Error::Io)?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen).map_err(crate::error::Error::Io)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).map_err(crate::error::Error::Io)?;

    let result = run_loop(&mut terminal, &mut app);

    disable_raw_mode().map_err(crate::error::Error::Io)?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen).map_err(crate::error::Error::Io)?;
    terminal.show_cursor().map_err(crate::error::Error::Io)?;

    result?;

    pending::apply(&app.pending, store, clock, editor)?;
    Ok(())
}

fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> Result<()> {
    loop {
        terminal
            .draw(|f| render::draw(f, app))
            .map_err(crate::error::Error::Io)?;

        if let Event::Key(key) = event::read().map_err(crate::error::Error::Io)? {
            if key.kind == KeyEventKind::Press {
                let quit = events::handle(app, key);
                if quit {
                    break;
                }
            }
        }
    }
    Ok(())
}
