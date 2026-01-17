use std::path::PathBuf;

use crate::db::Database;

use super::Result;

/// Add a directory to the database.
pub fn run(path: PathBuf) -> Result<()> {
    let mut db = Database::open()?;
    db.add(&path)?;
    db.save()?;
    Ok(())
}
