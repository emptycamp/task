//! Interactive picker for `task history`.
//!
//! Behaves like `task` view's pending-change model:
//! - `u` or Enter toggles a "mark for undo" anchor on the selected event.
//! - Marking an event also marks every newer event (the cascade) — they're shown
//!   struck-through in red so the user sees exactly what's about to happen.
//! - Esc / Ctrl+C exits and applies all marks (newest first). `q` does nothing,
//!   matching `task` view's keymap.

use crate::error::{Error, Result};
use crate::format::format_relative_past;
use crate::store::revert::HistoryEntry;
use crate::store::Store;
use chrono::Utc;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};
use ratatui::{Frame, Terminal};
use std::io;

pub struct App {
    /// Events sorted newest-first.
    pub entries: Vec<(u64, HistoryEntry)>,
    pub cursor: usize,
    /// The oldest event marked for undo. Everything with id >= anchor is also marked
    /// (the cascade). `None` means nothing is marked.
    pub anchor: Option<u64>,
    pub error: Option<String>,
}

impl App {
    pub fn from_entries(entries: Vec<(u64, HistoryEntry)>) -> Self {
        Self {
            entries,
            cursor: 0,
            anchor: None,
            error: None,
        }
    }

    pub fn is_marked(&self, id: u64) -> bool {
        self.anchor.map(|a| id >= a).unwrap_or(false)
    }

    /// Toggle the mark anchor at the cursor's event. Pressing on the current anchor
    /// clears all marks; pressing anywhere else moves the anchor (which may extend or
    /// narrow the cascade depending on where the user is).
    pub fn toggle_mark_at_cursor(&mut self) {
        let Some((id, _)) = self.entries.get(self.cursor) else {
            return;
        };
        let id = *id;
        if self.anchor == Some(id) {
            self.anchor = None;
        } else {
            self.anchor = Some(id);
        }
        self.error = None;
    }

    /// Return ids to revert in apply order (newest first). Empty if nothing is marked.
    pub fn cascade_ids(&self) -> Vec<u64> {
        let Some(anchor) = self.anchor else {
            return Vec::new();
        };
        let mut ids: Vec<u64> = self
            .entries
            .iter()
            .map(|(id, _)| *id)
            .filter(|id| *id >= anchor)
            .collect();
        ids.sort_by(|a, b| b.cmp(a));
        ids
    }
}

fn load_entries(store: &Store) -> Result<Vec<(u64, HistoryEntry)>> {
    let mut entries = store.history()?;
    entries.sort_by(|a, b| b.0.cmp(&a.0));
    Ok(entries)
}

pub fn run(store: &mut Store) -> Result<()> {
    let mut app = App::from_entries(load_entries(store)?);

    enable_raw_mode().map_err(Error::Io)?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen).map_err(Error::Io)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).map_err(Error::Io)?;

    let result = run_loop(&mut terminal, &mut app);

    let _ = disable_raw_mode();
    let _ = execute!(terminal.backend_mut(), LeaveAlternateScreen);
    let _ = terminal.show_cursor();

    result?;

    // Apply all marked reverts, newest first. Errors bubble up so the user sees them
    // rather than silently leaving half-state.
    for id in app.cascade_ids() {
        store.history_revert(id)?;
    }
    Ok(())
}

fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> Result<()> {
    loop {
        terminal.draw(|f| draw(f, app)).map_err(Error::Io)?;

        let key = match event::read().map_err(Error::Io)? {
            Event::Key(k) if k.kind == KeyEventKind::Press => k,
            _ => continue,
        };

        match (key.code, key.modifiers) {
            // Quit keys match `task` view exactly. `q` deliberately does nothing.
            (KeyCode::Esc, _) | (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                return Ok(());
            }
            (KeyCode::Up, _) => {
                app.cursor = app.cursor.saturating_sub(1);
            }
            (KeyCode::Down, _) => {
                if app.cursor + 1 < app.entries.len() {
                    app.cursor += 1;
                }
            }
            (KeyCode::Char('u'), KeyModifiers::NONE) | (KeyCode::Enter, _) => {
                app.toggle_mark_at_cursor();
            }
            _ => {}
        }
    }
}

fn draw(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // header
            Constraint::Min(3),    // list
            Constraint::Length(1), // status
            Constraint::Length(1), // help
        ])
        .split(frame.area());

    let header = Paragraph::new(Span::styled(
        "   ID  When         Event",
        Style::default()
            .fg(Color::DarkGray)
            .add_modifier(Modifier::BOLD),
    ));
    frame.render_widget(header, chunks[0]);

    let row_width = chunks[1].width.saturating_sub(2) as usize;
    let now = Utc::now();

    let items: Vec<ListItem> = if app.entries.is_empty() {
        vec![ListItem::new(Line::from(Span::styled(
            "  No history.",
            Style::default().fg(Color::DarkGray),
        )))]
    } else {
        app.entries
            .iter()
            .map(|(id, entry)| {
                let marked = app.is_marked(*id);
                let prefix = if marked { "✗ " } else { "  " };
                let when = format_relative_past(entry.timestamp, now);
                let summary = entry.op.summary();
                let mut text = format!("{}{:>4}  {:<11}  {}", prefix, id, when, summary);
                while text.chars().count() < row_width {
                    text.push(' ');
                }
                let style = if marked {
                    Style::default()
                        .fg(Color::Red)
                        .add_modifier(Modifier::CROSSED_OUT)
                } else {
                    Style::default()
                };
                ListItem::new(Line::from(Span::styled(text, style)))
            })
            .collect()
    };

    let mut state = ListState::default();
    if !app.entries.is_empty() {
        state.select(Some(app.cursor));
    }

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL))
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        );
    ratatui::widgets::StatefulWidget::render(list, chunks[1], frame.buffer_mut(), &mut state);

    let status_line: Line = if let Some(err) = &app.error {
        Line::from(Span::styled(
            format!(" ! {err}"),
            Style::default().fg(Color::Red),
        ))
    } else if let Some(anchor) = app.anchor {
        let count = app.cascade_ids().len();
        let msg = if count == 1 {
            format!(" 1 event marked (#{anchor})")
        } else {
            format!(" {count} events marked — cascades from #{anchor} forward")
        };
        Line::from(Span::styled(msg, Style::default().fg(Color::Yellow)))
    } else {
        Line::from(Span::raw(""))
    };
    frame.render_widget(Paragraph::new(status_line), chunks[2]);

    let help = Paragraph::new(Span::styled(
        " ↑↓ navigate · u/Enter toggle undo · Esc apply & quit ",
        Style::default().fg(Color::DarkGray),
    ));
    frame.render_widget(help, chunks[3]);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::revert::RevertOp;
    use chrono::Utc;

    fn entries(ids: &[u64]) -> Vec<(u64, HistoryEntry)> {
        let now = Utc::now();
        ids.iter()
            .rev() // newest first
            .map(|id| {
                (
                    *id,
                    HistoryEntry {
                        op: RevertOp::Added { id: *id as u32 },
                        timestamp: now,
                    },
                )
            })
            .collect()
    }

    #[test]
    fn empty_cascade_when_no_anchor() {
        let app = App::from_entries(entries(&[1, 2, 3]));
        assert!(app.cascade_ids().is_empty());
    }

    #[test]
    fn marking_cursor_event_sets_anchor() {
        let mut app = App::from_entries(entries(&[1, 2, 3]));
        // Newest-first: entries[0] = (3, ...), entries[1] = (2, ...), entries[2] = (1, ...).
        app.cursor = 1; // event id 2
        app.toggle_mark_at_cursor();
        assert_eq!(app.anchor, Some(2));
    }

    #[test]
    fn cascade_includes_target_and_all_newer() {
        let mut app = App::from_entries(entries(&[1, 2, 3, 4, 5]));
        // Move cursor to event id 3 (the middle).
        // Newest-first order means (5,4,3,2,1), so cursor 2 = id 3.
        app.cursor = 2;
        app.toggle_mark_at_cursor();
        let cascade = app.cascade_ids();
        assert_eq!(cascade, vec![5, 4, 3]);
    }

    #[test]
    fn marking_older_extends_cascade() {
        let mut app = App::from_entries(entries(&[1, 2, 3]));
        app.cursor = 0; // id 3
        app.toggle_mark_at_cursor();
        assert_eq!(app.cascade_ids(), vec![3]);
        // Move down to older id and re-anchor.
        app.cursor = 2; // id 1
        app.toggle_mark_at_cursor();
        assert_eq!(app.cascade_ids(), vec![3, 2, 1]);
    }

    #[test]
    fn pressing_on_current_anchor_clears_marks() {
        let mut app = App::from_entries(entries(&[1, 2, 3]));
        app.cursor = 1; // id 2
        app.toggle_mark_at_cursor();
        assert_eq!(app.anchor, Some(2));
        app.toggle_mark_at_cursor();
        assert_eq!(app.anchor, None);
        assert!(app.cascade_ids().is_empty());
    }

    #[test]
    fn is_marked_checks_cascade_membership() {
        let mut app = App::from_entries(entries(&[1, 2, 3]));
        app.cursor = 1; // id 2
        app.toggle_mark_at_cursor();
        assert!(app.is_marked(3));
        assert!(app.is_marked(2));
        assert!(!app.is_marked(1));
    }
}
