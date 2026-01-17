use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A directory entry in the gump database.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirEntry {
    /// Raw access score (increments by 1 on each access)
    pub score: f64,

    /// Last time this directory was accessed
    pub last_accessed: DateTime<Utc>,

    /// Total number of times this directory has been accessed
    pub access_count: u64,
}

impl DirEntry {
    /// Create a new entry with initial values.
    pub fn new() -> Self {
        Self {
            score: 1.0,
            last_accessed: Utc::now(),
            access_count: 1,
        }
    }

    /// Record an access to this directory, updating score and timestamp.
    pub fn record_access(&mut self) {
        self.score += 1.0;
        self.last_accessed = Utc::now();
        self.access_count += 1;
    }

    /// Calculate the frecency score based on recency.
    ///
    /// The frecency is the raw score multiplied by a time-based factor:
    /// - Last hour: score × 4
    /// - Last day: score × 2
    /// - Last week: score ÷ 2
    /// - Older: score ÷ 4
    pub fn frecency(&self) -> f64 {
        let now = Utc::now();
        let duration = now.signed_duration_since(self.last_accessed);
        let hours = duration.num_hours();

        let multiplier = if hours < 1 {
            4.0
        } else if hours < 24 {
            2.0
        } else if hours < 24 * 7 {
            0.5
        } else {
            0.25
        };

        self.score * multiplier
    }

    /// Check if this entry is stale (not accessed in 90+ days and path doesn't exist).
    pub fn is_stale(&self) -> bool {
        let now = Utc::now();
        let duration = now.signed_duration_since(self.last_accessed);
        duration.num_days() > 90
    }
}

impl Default for DirEntry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_entry() {
        let entry = DirEntry::new();
        assert_eq!(entry.score, 1.0);
        assert_eq!(entry.access_count, 1);
    }

    #[test]
    fn test_record_access() {
        let mut entry = DirEntry::new();
        entry.record_access();
        assert_eq!(entry.score, 2.0);
        assert_eq!(entry.access_count, 2);
    }

    #[test]
    fn test_frecency_recent() {
        let entry = DirEntry::new();
        // Just created, should have 4x multiplier
        let frecency = entry.frecency();
        assert!((frecency - 4.0).abs() < 0.01);
    }
}
