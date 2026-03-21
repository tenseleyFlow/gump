use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::DirEntry;

/// Default maximum total score before aging is applied.
const DEFAULT_MAX_AGE: f64 = 10000.0;

/// Database schema version for future migrations.
const SCHEMA_VERSION: u32 = 1;

#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Bincode(#[from] bincode::Error),

    #[error("Failed to get data directory")]
    NoDataDir,

    #[error("Path not found in database: {0}")]
    PathNotFound(PathBuf),
}

/// The on-disk database format.
#[derive(Debug, Serialize, Deserialize)]
struct DatabaseFile {
    version: u32,
    entries: HashMap<PathBuf, DirEntry>,
    last_cleanup: DateTime<Utc>,
}

impl Default for DatabaseFile {
    fn default() -> Self {
        Self {
            version: SCHEMA_VERSION,
            entries: HashMap::new(),
            last_cleanup: Utc::now(),
        }
    }
}

/// The gump database for tracking directory frecency.
pub struct Database {
    path: PathBuf,
    data: DatabaseFile,
    max_age: f64,
}

impl Database {
    /// Open or create the database at the default location.
    pub fn open() -> Result<Self, DatabaseError> {
        let path = Self::default_path()?;
        Self::open_at(path)
    }

    /// Open or create the database at a specific path.
    pub fn open_at(path: PathBuf) -> Result<Self, DatabaseError> {
        let max_age = std::env::var("GUMP_MAXAGE")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(DEFAULT_MAX_AGE);

        let data = if path.exists() {
            Self::read_file(&path)?
        } else {
            DatabaseFile::default()
        };

        Ok(Self { path, data, max_age })
    }

    /// Get the default database path.
    fn default_path() -> Result<PathBuf, DatabaseError> {
        // Check for override first
        if let Ok(dir) = std::env::var("GUMP_DATA_DIR") {
            return Ok(PathBuf::from(dir).join("db.bin"));
        }

        // Use directories crate for XDG compliance
        let dirs = directories::ProjectDirs::from("", "", "gump")
            .ok_or(DatabaseError::NoDataDir)?;

        let data_dir = dirs.data_dir();
        Ok(data_dir.join("db.bin"))
    }

    /// Read and deserialize the database file.
    fn read_file(path: &Path) -> Result<DatabaseFile, DatabaseError> {
        let mut file = File::open(path)?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;

        if buffer.is_empty() {
            return Ok(DatabaseFile::default());
        }

        let data: DatabaseFile = bincode::deserialize(&buffer)?;
        Ok(data)
    }

    /// Save the database to disk atomically.
    pub fn save(&self) -> Result<(), DatabaseError> {
        // Ensure parent directory exists
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Serialize to bytes
        let bytes = bincode::serialize(&self.data)?;

        // Write to temp file first
        let temp_path = self.path.with_extension("bin.tmp");
        {
            let mut file = OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(&temp_path)?;
            file.write_all(&bytes)?;
            file.sync_all()?;
        }

        // Atomic rename
        fs::rename(&temp_path, &self.path)?;

        Ok(())
    }

    /// Add or update a directory in the database.
    pub fn add<P: AsRef<Path>>(&mut self, path: P) -> Result<(), DatabaseError> {
        let path = self.canonicalize_path(path)?;

        // Check exclusions
        if self.is_excluded(&path) {
            return Ok(());
        }

        // Add or update entry
        if let Some(entry) = self.data.entries.get_mut(&path) {
            entry.record_access();
        } else {
            self.data.entries.insert(path, DirEntry::new());
        }

        // Apply aging if needed
        self.maybe_age();

        Ok(())
    }

    /// Import an entry with a specific score (for importing from zoxide).
    pub fn import_entry<P: AsRef<Path>>(&mut self, path: P, score: f64) -> Result<(), DatabaseError> {
        let path = self.canonicalize_path(path)?;

        // Skip if excluded
        if self.is_excluded(&path) {
            return Ok(());
        }

        // Only import if path exists
        if !path.exists() {
            return Ok(());
        }

        // Add or merge with existing entry
        if let Some(entry) = self.data.entries.get_mut(&path) {
            // Merge scores (take the higher one)
            if score > entry.score {
                entry.score = score;
            }
        } else {
            self.data.entries.insert(path, DirEntry::with_score(score));
        }

        Ok(())
    }

    /// Remove a directory from the database.
    pub fn remove<P: AsRef<Path>>(&mut self, path: P) -> Result<(), DatabaseError> {
        let path = self.canonicalize_path(path)?;

        if self.data.entries.remove(&path).is_none() {
            return Err(DatabaseError::PathNotFound(path));
        }

        Ok(())
    }

    /// Get all entries as (path, entry) pairs.
    pub fn entries(&self) -> impl Iterator<Item = (&PathBuf, &DirEntry)> {
        self.data.entries.iter()
    }

    /// Remove entries for directories that no longer exist.
    pub fn clean(&mut self) -> Vec<PathBuf> {
        let mut removed = Vec::new();

        self.data.entries.retain(|path, entry| {
            if !path.exists() && entry.is_stale() {
                removed.push(path.clone());
                false
            } else {
                true
            }
        });

        self.data.last_cleanup = Utc::now();

        removed
    }

    /// Canonicalize a path for storage.
    fn canonicalize_path<P: AsRef<Path>>(&self, path: P) -> Result<PathBuf, DatabaseError> {
        let path = path.as_ref();

        // Handle "." specially
        let path = if path == Path::new(".") {
            std::env::current_dir()?
        } else {
            path.to_path_buf()
        };

        // Try to canonicalize, but allow non-existent paths
        match fs::canonicalize(&path) {
            Ok(p) => Ok(p),
            Err(_) => {
                // If canonicalize fails, try to make it absolute
                if path.is_absolute() {
                    Ok(path)
                } else {
                    let cwd = std::env::current_dir()?;
                    Ok(cwd.join(path))
                }
            }
        }
    }

    /// Check if a path should be excluded.
    fn is_excluded(&self, path: &Path) -> bool {
        if let Ok(exclude) = std::env::var("GUMP_EXCLUDE") {
            for excluded in exclude.split(':') {
                if path.starts_with(excluded) {
                    return true;
                }
            }
        }
        false
    }

    /// Apply aging if total score exceeds max_age.
    fn maybe_age(&mut self) {
        let total: f64 = self.data.entries.values().map(|e| e.score).sum();

        if total > self.max_age {
            let k = total / (self.max_age * 0.9);

            // Divide all scores and remove those below 1.0
            self.data.entries.retain(|_, entry| {
                entry.score /= k;
                entry.score >= 1.0
            });
        }
    }

    /// Get the number of entries in the database.
    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.data.entries.len()
    }

    /// Check if the database is empty.
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.data.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn test_db() -> Database {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.bin");
        Database::open_at(path).unwrap()
    }

    #[test]
    fn test_add_and_list() {
        let mut db = test_db();
        let temp = tempdir().unwrap();
        let test_path = temp.path().to_path_buf();

        db.add(&test_path).unwrap();
        assert_eq!(db.len(), 1);

        let entries: Vec<_> = db.entries().collect();
        assert_eq!(entries.len(), 1);
    }

    #[test]
    fn test_remove() {
        let mut db = test_db();
        let temp = tempdir().unwrap();
        let test_path = temp.path().to_path_buf();

        db.add(&test_path).unwrap();
        assert_eq!(db.len(), 1);

        db.remove(&test_path).unwrap();
        assert_eq!(db.len(), 0);
    }

    #[test]
    fn test_save_and_load() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.bin");
        let temp = tempdir().unwrap();
        let test_path = temp.path().to_path_buf();

        {
            let mut db = Database::open_at(db_path.clone()).unwrap();
            db.add(&test_path).unwrap();
            db.save().unwrap();
        }

        {
            let db = Database::open_at(db_path).unwrap();
            assert_eq!(db.len(), 1);
        }
    }
}
