use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "gump")]
#[command(author, version, about = "A smarter cd command - zoxide without the z")]
#[command(propagate_version = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Add a directory to the database
    Add {
        /// Path to add (defaults to current directory)
        #[arg(default_value = ".")]
        path: PathBuf,
    },

    /// Query the database for matching directories
    Query {
        /// Search terms
        #[arg(required = true)]
        terms: Vec<String>,

        /// Show frecency scores
        #[arg(short, long)]
        score: bool,

        /// Show all matches instead of just the best
        #[arg(short, long)]
        all: bool,

        /// Match against current directory contents instead of database
        #[arg(long)]
        cwd: bool,
    },

    /// Remove a directory from the database
    Remove {
        /// Path to remove
        path: PathBuf,
    },

    /// List all directories in the database
    List {
        /// Show frecency scores
        #[arg(short, long)]
        score: bool,
    },

    /// Remove directories that no longer exist
    Clean,

    /// Generate shell integration code
    Init {
        /// Shell to generate code for
        shell: Shell,

        /// Custom command name (default: g)
        #[arg(long, default_value = "g")]
        cmd: String,

        /// When to update the database
        #[arg(long, default_value = "pwd")]
        hook: Hook,

        /// Only output hooks, no command aliases
        #[arg(long)]
        no_cmd: bool,
    },

    /// Import directories from zoxide
    Import,

    /// Edit the database in your editor
    Edit,
}

#[derive(Clone, ValueEnum)]
pub enum Shell {
    Bash,
    Zsh,
    Fish,
}

#[derive(Clone, ValueEnum)]
pub enum Hook {
    /// Update on every prompt
    Prompt,
    /// Update when directory changes
    Pwd,
}
