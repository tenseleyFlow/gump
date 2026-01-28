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

    // Require terms unless --all is specified
    if terms.is_empty() && !show_all {
        eprintln!("gump: no search terms provided");
        std::process::exit(1);
    }

    if cwd_mode {
        return run_cwd_mode(&mut matcher, &terms, show_all);
    }

    let db = Database::open()?;

    // If no terms, just list all entries sorted by frecency
    if terms.is_empty() {
        let mut entries: Vec<_> = db.entries().collect();
        entries.sort_by(|a, b| b.1.frecency().partial_cmp(&a.1.frecency()).unwrap_or(std::cmp::Ordering::Equal));

        if entries.is_empty() {
            std::process::exit(1);
        }

        for (path, entry) in entries {
            if show_score {
                println!("{:>8.2}  {}", entry.frecency(), path.display());
            } else {
                println!("{}", path.display());
            }
        }
        return Ok(());
    }

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
/// Matches against directory NAMES only (not full paths) to avoid false positives
/// from parent directory names appearing in the path.
fn run_cwd_mode(matcher: &mut Matcher, terms: &[String], show_all: bool) -> Result<()> {
    let cwd = std::env::current_dir()?;

    // Read directory entries, filter to directories only
    // Store both the full path (for result) and just the name (for matching)
    let dirs: Vec<(PathBuf, PathBuf)> = std::fs::read_dir(&cwd)?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false))
        .map(|entry| {
            let full_path = entry.path();
            let name_only = PathBuf::from(entry.file_name());
            (full_path, name_only)
        })
        .collect();

    if dirs.is_empty() {
        std::process::exit(1);
    }

    // Match against directory NAMES only, not full paths
    let paths = dirs.iter().map(|(_, name)| (name.as_path(), 1.0));

    let matches = matcher.rank(paths, terms);

    if matches.is_empty() {
        std::process::exit(1);
    }

    // Find the full path for each matched name
    if show_all {
        for m in &matches {
            // Find the full path that corresponds to this matched name
            if let Some((full_path, _)) = dirs.iter().find(|(_, name)| name.as_path() == m.path) {
                println!("{}", full_path.display());
            }
        }
    } else {
        // Find the full path for the best match
        let matched_name = matches[0].path;
        if let Some((full_path, _)) = dirs.iter().find(|(_, name)| name.as_path() == matched_name) {
            println!("{}", full_path.display());
        }
    }

    Ok(())
}
