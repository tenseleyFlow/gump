use crate::db::Database;
use crate::matcher::Matcher;

use super::Result;

/// Query the database for matching directories using fuzzy matching.
///
/// Returns the best match by default, or all matches with --all.
/// Matches are ranked by combined frecency + fuzzy score.
pub fn run(terms: Vec<String>, show_score: bool, show_all: bool) -> Result<()> {
    let db = Database::open()?;
    let mut matcher = Matcher::new();

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
