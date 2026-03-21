use std::fs;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Command, Stdio};

use crate::db::Database;

use super::Result;

/// Maximum score for imported entries (prevents one source from dominating).
const MAX_IMPORT_SCORE: f64 = 100.0;

/// Import directories from various tools (zoxide, autojump, z, fasd).
pub fn run() -> Result<()> {
    let mut db = Database::open()?;
    let mut total_imported = 0;

    // Collect entries from each source with their raw scores
    let mut entries: Vec<(String, f64)> = Vec::new();

    collect_zoxide(&mut entries)?;
    collect_autojump(&mut entries)?;
    collect_z(&mut entries)?;
    collect_fasd(&mut entries)?;

    if entries.is_empty() {
        println!("No databases found to import from");
        return Ok(());
    }

    // Find max score for normalization
    let max_score = entries.iter().map(|(_, s)| *s).fold(0.0f64, f64::max);

    let total_found = entries.len();

    // Normalize and import
    let mut skipped = 0;
    for (path, score) in entries {
        // Scale score to 0-MAX_IMPORT_SCORE range
        let normalized = if max_score > 0.0 {
            (score / max_score) * MAX_IMPORT_SCORE
        } else {
            1.0
        };

        match db.import_entry(&path, normalized.max(1.0)) {
            Ok(()) => total_imported += 1,
            Err(_) => skipped += 1,
        }
    }

    if total_imported > 0 {
        db.save()?;
    }

    println!("Found {} entries, imported {} (scores normalized to 1-{})",
        total_found, total_imported, MAX_IMPORT_SCORE as u32);
    if skipped > 0 {
        println!("Skipped {} entries (excluded or unresolvable paths)", skipped);
    }

    Ok(())
}

/// Collect entries from zoxide using its CLI.
fn collect_zoxide(entries: &mut Vec<(String, f64)>) -> Result<()> {
    let output = Command::new("zoxide")
        .args(["query", "--list", "--score"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output();

    let output = match output {
        Ok(o) if o.status.success() => o,
        _ => return Ok(()),
    };

    let reader = BufReader::new(output.stdout.as_slice());
    let mut count = 0;

    for line in reader.lines() {
        let line = line?;
        if let Some((score, path)) = parse_score_path(&line) {
            if std::path::Path::new(path).exists() {
                entries.push((path.to_string(), score));
                count += 1;
            }
        }
    }

    if count > 0 {
        println!("  zoxide: {} entries", count);
    }
    Ok(())
}

/// Collect entries from autojump (~/.local/share/autojump/autojump.txt).
fn collect_autojump(entries: &mut Vec<(String, f64)>) -> Result<()> {
    let path = dirs_autojump();
    if !path.exists() {
        return Ok(());
    }

    let content = fs::read_to_string(&path)?;
    let mut count = 0;

    // Format: "score\tpath" (tab-separated)
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        if let Some((score_str, dir_path)) = line.split_once('\t') {
            if let Ok(score) = score_str.parse::<f64>() {
                if std::path::Path::new(dir_path).exists() {
                    entries.push((dir_path.to_string(), score));
                    count += 1;
                }
            }
        }
    }

    if count > 0 {
        println!("  autojump: {} entries", count);
    }
    Ok(())
}

/// Collect entries from z/z.lua/zsh-z (~/.z).
fn collect_z(entries: &mut Vec<(String, f64)>) -> Result<()> {
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

    let mut count = 0;

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
                let dir_path = parts[0];
                if let Ok(score) = parts[1].parse::<f64>() {
                    if std::path::Path::new(dir_path).exists() {
                        entries.push((dir_path.to_string(), score));
                        count += 1;
                    }
                }
            }
        }
    }

    if count > 0 {
        println!("  z/z.lua: {} entries", count);
    }
    Ok(())
}

/// Collect entries from fasd (~/.fasd).
fn collect_fasd(entries: &mut Vec<(String, f64)>) -> Result<()> {
    let path = dirs_fasd();
    if !path.exists() {
        return Ok(());
    }

    let content = fs::read_to_string(&path)?;
    let mut count = 0;

    // Format: "path|score|timestamp" (similar to z)
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let parts: Vec<&str> = line.split('|').collect();
        if parts.len() >= 2 {
            let dir_path = parts[0];
            if let Ok(score) = parts[1].parse::<f64>() {
                // fasd tracks files too, only import directories
                if std::path::Path::new(dir_path).is_dir() {
                    entries.push((dir_path.to_string(), score));
                    count += 1;
                }
            }
        }
    }

    if count > 0 {
        println!("  fasd: {} entries", count);
    }
    Ok(())
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
