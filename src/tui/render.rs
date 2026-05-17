use crate::model::{Priority, Task};
use crate::tui::events::PendingChange;
use crate::tui::App;
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};
use std::collections::HashMap;

pub fn draw(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(1)])
        .split(frame.area());

    let items: Vec<ListItem> = app
        .tasks
        .iter()
        .enumerate()
        .map(|(i, task)| make_item(task, i == app.cursor, &app.pending))
        .collect();

    let mut state = ListState::default();
    state.select(Some(app.cursor));

    let task_list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Tasks"))
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED));

    ratatui::widgets::StatefulWidget::render(task_list, chunks[0], frame.buffer_mut(), &mut state);

    let help = Paragraph::new("↑↓ navigate | c complete | d delete | e edit | Shift+A/B/C priority | Esc quit");
    frame.render_widget(help, chunks[1]);
}

fn make_item(
    task: &Task,
    _selected: bool,
    pending: &HashMap<u32, Vec<PendingChange>>,
) -> ListItem<'static> {
    let changes = pending.get(&task.id).map(|v| v.as_slice()).unwrap_or(&[]);

    let has_complete = changes.iter().any(|c| matches!(c, PendingChange::ToggleComplete(_)));
    let has_delete = changes.iter().any(|c| matches!(c, PendingChange::ToggleDelete(_)));
    let pending_priority = changes.iter().find_map(|c| {
        if let PendingChange::SetPriority(_, p) = c {
            Some(*p)
        } else {
            None
        }
    });

    let display_priority = pending_priority.unwrap_or(task.priority);
    let priority_char = display_priority.to_string();

    let color = match display_priority {
        Priority::A => Color::Red,
        Priority::B => Color::Yellow,
        Priority::C => Color::DarkGray,
    };

    let prefix = if has_complete {
        "✓ "
    } else if has_delete {
        "✗ "
    } else {
        "  "
    };

    let text = format!(
        "{}{:>3}  [{}]  {}",
        prefix, task.id, priority_char, task.text
    );

    let style = Style::default().fg(color);
    let style = if has_delete {
        style.add_modifier(Modifier::CROSSED_OUT)
    } else if has_complete {
        style.fg(Color::Green)
    } else {
        style
    };

    ListItem::new(Line::from(Span::styled(text, style)))
}
