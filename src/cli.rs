use crate::model::TaskId;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "task", about = "Personal task manager")]
pub struct Cli {
    #[arg(long, global = true, hide = true)]
    pub test: bool,
    #[command(subcommand)]
    pub cmd: Option<Cmd>,
}

#[derive(Subcommand)]
pub enum Cmd {
    Add {
        args: Vec<String>,
    },
    #[command(visible_aliases = ["ls"])]
    List {
        #[arg(short, long)]
        all: bool,
        which: Option<String>,
    },
    #[command(visible_aliases = ["update"])]
    Edit {
        id: TaskId,
        args: Vec<String>,
    },
    #[command(visible_aliases = ["del", "remove", "rm"])]
    Delete {
        id: TaskId,
    },
    #[command(visible_aliases = ["done"])]
    Complete {
        id: TaskId,
    },
    #[command(visible_aliases = ["show"])]
    Info {
        id: TaskId,
    },
    Revert {
        #[arg(short = 'y')]
        yes: bool,
    },
}
