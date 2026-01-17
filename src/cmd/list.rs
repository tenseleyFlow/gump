use crate::db::Database;

use super::Result;

/// List all directories in the database.
pub fn run(show_score: bool) -> Result<()> {
    let db = Database::open()?;

    // Collect and sort by frecency (descending)
    let mut entries: Vec<_> = db.entries().collect();
    entries.sort_by(|a, b| {
        b.1.frecency()
            .partial_cmp(&a.1.frecency())
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    for (path, entry) in entries {
        if show_score {
            println!("{:>8.2}  {}", entry.frecency(), path.display());
        } else {
            println!("{}", path.display());
        }
    }

    Ok(())
}
