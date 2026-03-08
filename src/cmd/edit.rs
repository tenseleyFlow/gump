use std::fs;
use std::io::{Read, Write};
use std::process::Command;

use serde::{Deserialize, Serialize};

use crate::db::Database;

use super::{CmdError, Result};

#[derive(Serialize, Deserialize)]
struct JsonEntry {
    path: String,
    score: f64,
}

#[derive(Serialize, Deserialize)]
struct JsonDatabase {
    entries: Vec<JsonEntry>,
}

/// Edit the database in the user's editor.
pub fn run() -> Result<()> {
    let db = Database::open()?;

    // Export to JSON
    let json_db = JsonDatabase {
        entries: db
            .entries()
            .map(|(path, entry)| JsonEntry {
                path: path.to_string_lossy().to_string(),
                score: entry.score,
            })
            .collect(),
    };

    let json = serde_json::to_string_pretty(&json_db)
        .map_err(|e| CmdError::Other(format!("Failed to serialize: {}", e)))?;

    // Write to temp file
    let temp_path = std::env::temp_dir().join("gump-edit.json");
    {
        let mut file = fs::File::create(&temp_path)?;
        file.write_all(json.as_bytes())?;
    }

    // Get editor
    let editor = std::env::var("EDITOR")
        .or_else(|_| std::env::var("VISUAL"))
        .unwrap_or_else(|_| "vi".to_string());

    // Open in editor
    let status = Command::new(&editor)
        .arg(&temp_path)
        .status()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                CmdError::Other(format!(
                    "editor '{}' not found. Set $EDITOR or $VISUAL to an installed editor",
                    editor
                ))
            } else {
                CmdError::Io(e)
            }
        })?;

    if !status.success() {
        return Err(CmdError::Other("Editor exited with error".to_string()));
    }

    // Read modified file
    let mut modified = String::new();
    {
        let mut file = fs::File::open(&temp_path)?;
        file.read_to_string(&mut modified)?;
    }

    // Parse modified JSON
    let modified_db: JsonDatabase = serde_json::from_str(&modified)
        .map_err(|e| CmdError::Other(format!("Failed to parse JSON: {}", e)))?;

    // Create new database with modified entries
    let mut new_db = Database::open()?;

    // Clear existing entries by removing each one
    let existing: Vec<_> = new_db.entries().map(|(p, _)| p.clone()).collect();
    for path in existing {
        let _ = new_db.remove(&path);
    }

    // Add modified entries
    for entry in modified_db.entries {
        new_db.import_entry(&entry.path, entry.score)?;
    }

    new_db.save()?;

    // Clean up temp file
    let _ = fs::remove_file(&temp_path);

    println!("Database updated");
    Ok(())
}
