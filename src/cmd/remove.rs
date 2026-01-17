use std::path::PathBuf;

use crate::db::Database;

use super::Result;

/// Remove a directory from the database.
pub fn run(path: PathBuf) -> Result<()> {
    let mut db = Database::open()?;
    db.remove(&path)?;
    db.save()?;
    Ok(())
}
