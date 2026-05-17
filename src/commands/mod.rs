pub mod add;
pub mod complete;
pub mod delete;
pub mod edit;
pub mod info;
pub mod list;
pub mod revert;

use crate::cli::{Cli, Cmd};
use crate::clock::Clock;
use crate::confirm::Prompt;
use crate::editor::EditorLauncher;
use crate::error::Result;
use crate::format::RenderOptions;
use crate::store::Store;
use crate::store::gc;
use crate::tui;

pub trait Tty {
    fn is_tty(&self) -> bool;
}

pub struct SystemTty;
impl Tty for SystemTty {
    fn is_tty(&self) -> bool {
        use std::io::IsTerminal;
        std::io::stdout().is_terminal()
    }
}

pub fn dispatch(
    cli: &Cli,
    store: &mut Store,
    clock: &dyn Clock,
    editor: &dyn EditorLauncher,
    prompt: &dyn Prompt,
    tty: &dyn Tty,
) -> Result<()> {
    let gc_count = gc::sweep(store, clock)?;

    let opts = if tty.is_tty() {
        RenderOptions::detect()
    } else {
        RenderOptions::no_color()
    };

    match &cli.cmd {
        None => {
            tui::run(store, clock, editor)?;
        }
        Some(Cmd::Add { args }) => {
            let task = add::run(args, store, clock)?;
            println!("Added task #{}: {}", task.id, task.text);
        }
        Some(Cmd::List { all, which }) => {
            let (output, _) = list::run_with_gc_count(store, *all, which.as_deref(), &opts, gc_count)?;
            let final_output = list::format_with_footer(&output, gc_count);
            print!("{final_output}");
        }
        Some(Cmd::Edit { id, args }) => {
            edit::run(*id, args, store, clock, editor)?;
            println!("Task #{id} updated.");
        }
        Some(Cmd::Delete { id }) => {
            delete::run(*id, store, clock)?;
            println!("Task #{id} deleted.");
        }
        Some(Cmd::Complete { id }) => {
            complete::run(*id, store, clock)?;
            println!("Task #{id} completed.");
        }
        Some(Cmd::Info { id }) => {
            let output = info::run(*id, store, &opts)?;
            print!("{output}");
        }
        Some(Cmd::Revert { yes }) => {
            revert::run(*yes, store, prompt)?;
            println!("Reverted.");
        }
    }
    Ok(())
}
