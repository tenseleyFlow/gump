use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};

use crate::db::Database;

use super::{CmdError, Result};

/// Import directories from zoxide database.
pub fn run() -> Result<()> {
    // Try to get entries from zoxide CLI
    let output = Command::new("zoxide")
        .args(["query", "--list", "--score"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output();

    let output = match output {
        Ok(o) if o.status.success() => o,
        Ok(_) => {
            return Err(CmdError::Other(
                "zoxide query failed - is zoxide installed and initialized?".to_string(),
            ));
        }
        Err(_) => {
            return Err(CmdError::Other(
                "zoxide not found - please install zoxide first".to_string(),
            ));
        }
    };

    let mut db = Database::open()?;
    let mut imported = 0;
    let mut skipped = 0;

    let reader = BufReader::new(output.stdout.as_slice());
    for line in reader.lines() {
        let line = line?;
        let line = line.trim();

        if line.is_empty() {
            continue;
        }

        // Parse "  123.4 /path/to/dir" format
        if let Some((score_str, path)) = parse_zoxide_line(line) {
            let score: f64 = match score_str.parse() {
                Ok(s) => s,
                Err(_) => {
                    skipped += 1;
                    continue;
                }
            };

            // Import with the zoxide score
            if let Err(_) = db.import_entry(path, score) {
                skipped += 1;
            } else {
                imported += 1;
            }
        } else {
            skipped += 1;
        }
    }

    db.save()?;

    println!("Imported {} entries from zoxide", imported);
    if skipped > 0 {
        println!("Skipped {} entries (parse errors or excluded paths)", skipped);
    }

    Ok(())
}

/// Parse a zoxide output line: "  123.4 /path/to/dir"
fn parse_zoxide_line(line: &str) -> Option<(&str, &str)> {
    let line = line.trim_start();
    let space_idx = line.find(' ')?;
    let score = &line[..space_idx];
    let path = line[space_idx..].trim_start();

    if path.is_empty() {
        return None;
    }

    Some((score, path))
}
