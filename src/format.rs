use crate::model::{Priority, Status, Task};
use chrono::{DateTime, Local, Utc};
use crossterm::style::{Color, Stylize};
use std::io::IsTerminal;

pub struct RenderOptions {
    pub color: bool,
}

impl RenderOptions {
    pub fn detect() -> Self {
        let color = std::io::stdout().is_terminal()
            && std::env::var("NO_COLOR").is_err();
        Self { color }
    }

    pub fn no_color() -> Self {
        Self { color: false }
    }
}

pub fn format_list(tasks: &[Task], opts: &RenderOptions) -> String {
    if tasks.is_empty() {
        return "No tasks.\n".to_string();
    }

    let mut out = String::new();
    for task in tasks {
        out.push_str(&format_list_row(task, opts));
        out.push('\n');
    }
    out
}

pub fn format_list_row(task: &Task, opts: &RenderOptions) -> String {
    let now: DateTime<Utc> = Utc::now();
    let due_local: DateTime<Local> = std::cmp::max(task.due, now).into();
    let due_str = due_local.format("%m/%d %H:%M").to_string();
    let est_str = format_est(task.est_secs);
    let text = truncate(&task.text, 40);

    let row = format!(
        "{:>3}  {:1}  {:40}  {}  {}",
        task.id, task.priority, text, due_str, est_str
    );

    if opts.color {
        let color = priority_color(task.priority);
        format!("{}", row.with(color))
    } else {
        row
    }
}

pub fn format_info(task: &Task, opts: &RenderOptions) -> String {
    let now: DateTime<Utc> = Utc::now();
    let due_local: DateTime<Local> = std::cmp::max(task.due, now).into();
    let created_local: DateTime<Local> = task.created_at.into();
    let status = match task.status {
        Status::Active => "active",
        Status::Completed => "completed",
        Status::SoftDeleted => "deleted",
    };

    let mut out = format!(
        "Task #{}\n  Text:     {}\n  Priority: {}\n  Status:   {}\n  Due:      {}\n  Est:      {}\n  Created:  {}\n",
        task.id,
        task.text,
        task.priority,
        status,
        due_local.format("%Y-%m-%d %H:%M"),
        format_est(task.est_secs),
        created_local.format("%Y-%m-%d %H:%M"),
    );

    if let Some(t) = task.completed_at {
        let local: DateTime<Local> = t.into();
        out.push_str(&format!("  Completed:{}\n", local.format("%Y-%m-%d %H:%M")));
    }
    if let Some(t) = task.deleted_at {
        let local: DateTime<Local> = t.into();
        out.push_str(&format!("  Deleted:  {}\n", local.format("%Y-%m-%d %H:%M")));
    }

    if opts.color {
        let color = priority_color(task.priority);
        format!("{}", out.with(color))
    } else {
        out
    }
}

fn priority_color(p: Priority) -> Color {
    match p {
        Priority::A => Color::Red,
        Priority::B => Color::Yellow,
        Priority::C => Color::DarkGrey,
    }
}

fn truncate(s: &str, width: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= width {
        s.to_string()
    } else {
        format!("{}...", &chars[..width - 3].iter().collect::<String>())
    }
}

fn format_est(secs: i64) -> String {
    if secs % 3600 == 0 {
        format!("{}h", secs / 3600)
    } else if secs % 60 == 0 {
        format!("{}m", secs / 60)
    } else {
        format!("{secs}s")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Priority, Status};
    use chrono::Utc;

    fn make_task(id: u32, text: &str, priority: Priority) -> Task {
        Task {
            id,
            text: text.to_string(),
            priority,
            due: Utc::now(),
            est_secs: 1800,
            status: Status::Active,
            created_at: Utc::now(),
            completed_at: None,
            deleted_at: None,
        }
    }

    #[test]
    fn format_list_no_color_contains_task_id() {
        let task = make_task(1, "Buy milk", Priority::B);
        let opts = RenderOptions::no_color();
        let output = format_list(&[task], &opts);
        assert!(output.contains("  1"));
        assert!(output.contains("Buy milk"));
    }

    #[test]
    fn format_list_empty_returns_no_tasks_message() {
        let opts = RenderOptions::no_color();
        let output = format_list(&[], &opts);
        assert_eq!(output, "No tasks.\n");
    }

    #[test]
    fn truncate_long_text_appends_ellipsis() {
        let long = "a".repeat(50);
        let result = truncate(&long, 40);
        assert!(result.ends_with("..."));
        assert_eq!(result.chars().count(), 40);
    }

    #[test]
    fn truncate_short_text_unchanged() {
        let short = "hello";
        assert_eq!(truncate(short, 40), "hello");
    }

    #[test]
    fn format_est_minutes() {
        assert_eq!(format_est(1800), "30m");
    }

    #[test]
    fn format_est_hours() {
        assert_eq!(format_est(7200), "2h");
    }

    #[test]
    fn format_est_seconds() {
        assert_eq!(format_est(90), "90s");
    }

    #[test]
    fn format_info_no_color_contains_text() {
        let task = make_task(42, "Test task", Priority::A);
        let opts = RenderOptions::no_color();
        let output = format_info(&task, &opts);
        assert!(output.contains("Test task"));
        assert!(output.contains("42"));
        assert!(output.contains("active"));
    }
}
