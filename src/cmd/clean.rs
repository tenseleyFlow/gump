use crate::db::Database;

use super::Result;

/// Remove directories that no longer exist from the database.
pub fn run() -> Result<()> {
    let mut db = Database::open()?;
    let removed = db.clean();

    if removed.is_empty() {
        println!("No stale entries found");
    } else {
        println!("Removed {} stale entries:", removed.len());
        for path in &removed {
            println!("  {}", path.display());
        }
    }

    db.save()?;
    Ok(())
}
