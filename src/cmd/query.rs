use std::path::PathBuf;

use crate::db::Database;
use crate::matcher::Matcher;

use super::Result;

/// Query the database for matching directories using fuzzy matching.
///
/// Returns the best match by default, or all matches with --all.
/// Matches are ranked by combined frecency + fuzzy score.
pub fn run(terms: Vec<String>, show_score: bool, show_all: bool, cwd_mode: bool) -> Result<()> {
    let mut matcher = Matcher::new();

    if cwd_mode {
        return run_cwd_mode(&mut matcher, &terms, show_all);
    }

    let db = Database::open()?;

    // Collect paths with their frecency scores
    let paths = db.entries().map(|(path, entry)| (path.as_path(), entry.frecency()));

    // Get ranked matches
    let matches = matcher.rank(paths, &terms);

    if matches.is_empty() {
        // Exit with code 1 to signal no match (used by shell integration)
        std::process::exit(1);
    }

    if show_all {
        for m in &matches {
            if show_score {
                println!("{:>8.2}  {}", m.combined_score, m.path.display());
            } else {
                println!("{}", m.path.display());
            }
        }
    } else {
        // Just print the best match
        let best = &matches[0];
        if show_score {
            println!("{:>8.2}  {}", best.combined_score, best.path.display());
        } else {
            println!("{}", best.path.display());
        }
    }

    Ok(())
}

/// Match against directories in the current working directory.
fn run_cwd_mode(matcher: &mut Matcher, terms: &[String], show_all: bool) -> Result<()> {
    let cwd = std::env::current_dir()?;

    // Read directory entries, filter to directories only
    let dirs: Vec<PathBuf> = std::fs::read_dir(&cwd)?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false))
        .map(|entry| entry.path())
        .collect();

    if dirs.is_empty() {
        std::process::exit(1);
    }

    // Use equal frecency (1.0) for all CWD entries - fuzzy score determines ranking
    let paths = dirs.iter().map(|p| (p.as_path(), 1.0));

    let matches = matcher.rank(paths, terms);

    if matches.is_empty() {
        std::process::exit(1);
    }

    if show_all {
        for m in &matches {
            println!("{}", m.path.display());
        }
    } else {
        println!("{}", matches[0].path.display());
    }

    Ok(())
}
