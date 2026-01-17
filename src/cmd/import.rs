use std::fs;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Command, Stdio};

use crate::db::Database;

use super::Result;

/// Import directories from various tools (zoxide, autojump, z, fasd).
pub fn run() -> Result<()> {
    let mut db = Database::open()?;
    let mut total_imported = 0;

    // Try each source
    total_imported += import_zoxide(&mut db)?;
    total_imported += import_autojump(&mut db)?;
    total_imported += import_z(&mut db)?;
    total_imported += import_fasd(&mut db)?;

    if total_imported > 0 {
        db.save()?;
        println!("Imported {} total entries", total_imported);
    } else {
        println!("No databases found to import from");
    }

    Ok(())
}

/// Import from zoxide using its CLI.
fn import_zoxide(db: &mut Database) -> Result<usize> {
    let output = Command::new("zoxide")
        .args(["query", "--list", "--score"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output();

    let output = match output {
        Ok(o) if o.status.success() => o,
        _ => return Ok(0),
    };

    let mut imported = 0;
    let reader = BufReader::new(output.stdout.as_slice());

    for line in reader.lines() {
        let line = line?;
        if let Some((score, path)) = parse_score_path(&line) {
            if db.import_entry(path, score).is_ok() {
                imported += 1;
            }
        }
    }

    if imported > 0 {
        println!("  zoxide: {} entries", imported);
    }
    Ok(imported)
}

/// Import from autojump (~/.local/share/autojump/autojump.txt).
fn import_autojump(db: &mut Database) -> Result<usize> {
    let path = dirs_autojump();
    if !path.exists() {
        return Ok(0);
    }

    let content = fs::read_to_string(&path)?;
    let mut imported = 0;

    // Format: "score\tpath" (tab-separated)
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        if let Some((score_str, path)) = line.split_once('\t') {
            if let Ok(score) = score_str.parse::<f64>() {
                if db.import_entry(path, score).is_ok() {
                    imported += 1;
                }
            }
        }
    }

    if imported > 0 {
        println!("  autojump: {} entries", imported);
    }
    Ok(imported)
}

/// Import from z/z.lua/zsh-z (~/.z).
fn import_z(db: &mut Database) -> Result<usize> {
    // Check both ~/.z and $Z_DATA / $_Z_DATA
    let paths: Vec<PathBuf> = [
        Some(dirs_z()),
        std::env::var("_Z_DATA").map(PathBuf::from).ok(),
        std::env::var("Z_DATA").map(PathBuf::from).ok(),
        std::env::var("ZSHZ_DATA").map(PathBuf::from).ok(),
    ]
    .into_iter()
    .flatten()
    .collect();

    let mut imported = 0;

    for path in paths {
        if !path.exists() {
            continue;
        }

        let content = match fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        // Format: "path|score|timestamp" (pipe-separated)
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            let parts: Vec<&str> = line.split('|').collect();
            if parts.len() >= 2 {
                let path = parts[0];
                if let Ok(score) = parts[1].parse::<f64>() {
                    if db.import_entry(path, score).is_ok() {
                        imported += 1;
                    }
                }
            }
        }
    }

    if imported > 0 {
        println!("  z/z.lua: {} entries", imported);
    }
    Ok(imported)
}

/// Import from fasd (~/.fasd).
fn import_fasd(db: &mut Database) -> Result<usize> {
    let path = dirs_fasd();
    if !path.exists() {
        return Ok(0);
    }

    let content = fs::read_to_string(&path)?;
    let mut imported = 0;

    // Format: "path|score|timestamp" (similar to z)
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let parts: Vec<&str> = line.split('|').collect();
        if parts.len() >= 2 {
            let path = parts[0];
            if let Ok(score) = parts[1].parse::<f64>() {
                // fasd tracks files too, only import directories
                if std::path::Path::new(path).is_dir() {
                    if db.import_entry(path, score).is_ok() {
                        imported += 1;
                    }
                }
            }
        }
    }

    if imported > 0 {
        println!("  fasd: {} entries", imported);
    }
    Ok(imported)
}

/// Parse "  123.4 /path/to/dir" format (zoxide output).
fn parse_score_path(line: &str) -> Option<(f64, &str)> {
    let line = line.trim_start();
    let space_idx = line.find(' ')?;
    let score_str = &line[..space_idx];
    let path = line[space_idx..].trim_start();

    if path.is_empty() {
        return None;
    }

    let score = score_str.parse().ok()?;
    Some((score, path))
}

fn dirs_autojump() -> PathBuf {
    if let Some(data) = dirs::data_local_dir() {
        data.join("autojump").join("autojump.txt")
    } else {
        PathBuf::from("~/.local/share/autojump/autojump.txt")
    }
}

fn dirs_z() -> PathBuf {
    dirs::home_dir()
        .map(|h| h.join(".z"))
        .unwrap_or_else(|| PathBuf::from("~/.z"))
}

fn dirs_fasd() -> PathBuf {
    dirs::home_dir()
        .map(|h| h.join(".fasd"))
        .unwrap_or_else(|| PathBuf::from("~/.fasd"))
}
