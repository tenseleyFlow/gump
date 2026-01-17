mod cli;
mod cmd;
mod db;
mod matcher;

use clap::Parser;
use cli::{Cli, Commands};

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Add { path } => cmd::add::run(path),
        Commands::Query { terms, score, all } => cmd::query::run(terms, score, all),
        Commands::Remove { path } => cmd::remove::run(path),
        Commands::List { score } => cmd::list::run(score),
        Commands::Clean => cmd::clean::run(),
        Commands::Init { shell, cmd, hook, no_cmd } => {
            cmd::init::run(shell, cmd, hook, no_cmd)
        }
        Commands::Import => cmd::import::run(),
        Commands::Edit => cmd::edit::run(),
    };

    if let Err(e) = result {
        eprintln!("gump: {}", e);
        std::process::exit(1);
    }
}
