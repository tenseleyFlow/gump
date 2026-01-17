use std::path::Path;

use nucleo::{
    pattern::{CaseMatching, Normalization, Pattern},
    Matcher as NucleoMatcher, Utf32Str,
};

/// A match result with path and combined score.
#[derive(Debug, Clone)]
pub struct Match<'a> {
    pub path: &'a Path,
    #[allow(dead_code)]
    pub frecency: f64,
    #[allow(dead_code)]
    pub fuzzy_score: u32,
    pub combined_score: f64,
}

/// Fuzzy matcher using nucleo with frecency integration.
pub struct Matcher {
    nucleo: NucleoMatcher,
}

impl Default for Matcher {
    fn default() -> Self {
        Self::new()
    }
}

impl Matcher {
    pub fn new() -> Self {
        Self {
            nucleo: NucleoMatcher::new(nucleo::Config::DEFAULT),
        }
    }

    /// Match a path against search terms.
    ///
    /// Returns a fuzzy score if all terms match, or None if no match.
    /// Uses nucleo's fuzzy matching - terms don't need to be exact substrings.
    /// The score is boosted if the last term matches the last path component.
    pub fn score(&mut self, path: &Path, terms: &[String]) -> Option<u32> {
        let path_str = path.to_string_lossy();
        let mut haystack_buf = Vec::new();
        let haystack = Utf32Str::new(&path_str, &mut haystack_buf);

        // All terms must fuzzy-match
        let mut total_score: u32 = 0;

        for term in terms {
            let pattern = Pattern::new(
                term,
                CaseMatching::Ignore,
                Normalization::Smart,
                nucleo::pattern::AtomKind::Fuzzy,
            );

            match pattern.score(haystack, &mut self.nucleo) {
                Some(score) => {
                    total_score = total_score.saturating_add(score);
                }
                None => {
                    // Term didn't match at all
                    return None;
                }
            }
        }

        // Check if terms appear in order within the path (for multi-term queries)
        if terms.len() > 1 && !self.terms_in_order(&path_str.to_lowercase(), terms) {
            // Terms matched but not in order - reduce score significantly but still match
            total_score = total_score / 4;
        }

        // Boost score if last term matches last path component
        if let Some(last_term) = terms.last() {
            if let Some(last_component) = path.file_name() {
                let last_comp_lower = last_component.to_string_lossy().to_lowercase();
                let last_term_lower = last_term.to_lowercase();

                // Check fuzzy match on last component
                let mut comp_buf = Vec::new();
                let comp_haystack = Utf32Str::new(&last_comp_lower, &mut comp_buf);
                let pattern = Pattern::new(
                    &last_term_lower,
                    CaseMatching::Ignore,
                    Normalization::Smart,
                    nucleo::pattern::AtomKind::Fuzzy,
                );

                if pattern.score(comp_haystack, &mut self.nucleo).is_some() {
                    // Significant boost for last-component match
                    total_score = total_score.saturating_add(100);

                    // Extra boost for exact match
                    if last_comp_lower == last_term_lower {
                        total_score = total_score.saturating_add(50);
                    }
                }
            }
        }

        Some(total_score)
    }

    /// Check if terms appear in order within the path (loosely).
    fn terms_in_order(&self, path_lower: &str, terms: &[String]) -> bool {
        let mut last_pos = 0;

        for term in terms {
            let term_lower = term.to_lowercase();
            // Find any character from the term after last_pos
            if let Some(first_char) = term_lower.chars().next() {
                if let Some(pos) = path_lower[last_pos..].find(first_char) {
                    last_pos += pos + 1;
                } else {
                    return false;
                }
            }
        }

        true
    }

    /// Match and rank paths by combined frecency + fuzzy score.
    ///
    /// Returns matches sorted by combined score (descending).
    pub fn rank<'a, I>(&mut self, paths: I, terms: &[String]) -> Vec<Match<'a>>
    where
        I: Iterator<Item = (&'a Path, f64)>, // (path, frecency)
    {
        let mut matches: Vec<Match<'a>> = paths
            .filter_map(|(path, frecency)| {
                let fuzzy_score = self.score(path, terms)?;

                // Combined score: frecency is the primary factor, fuzzy score is secondary
                // Frecency typically ranges 0.25-40+, fuzzy score 0-500+
                // We normalize fuzzy score to 0-1 range and use it as a multiplier
                let fuzzy_factor = 1.0 + (fuzzy_score as f64 / 500.0).min(1.0);
                let combined_score = frecency * fuzzy_factor;

                Some(Match {
                    path,
                    frecency,
                    fuzzy_score,
                    combined_score,
                })
            })
            .collect();

        // Sort by combined score (descending)
        matches.sort_by(|a, b| {
            b.combined_score
                .partial_cmp(&a.combined_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        matches
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_single_term_match() {
        let mut matcher = Matcher::new();
        let path = PathBuf::from("/home/user/projects/myapp");

        let score = matcher.score(&path, &["myapp".to_string()]);
        assert!(score.is_some());
        assert!(score.unwrap() > 0);
    }

    #[test]
    fn test_multi_term_match() {
        let mut matcher = Matcher::new();
        let path = PathBuf::from("/home/user/projects/myapp");

        // Terms in order should match with higher score
        let score_ordered = matcher.score(&path, &["proj".to_string(), "app".to_string()]);
        assert!(score_ordered.is_some());

        // Terms out of order should still match but with lower score
        let score_unordered = matcher.score(&path, &["app".to_string(), "proj".to_string()]);
        assert!(score_unordered.is_some());

        // Ordered should score higher
        assert!(score_ordered.unwrap() > score_unordered.unwrap());
    }

    #[test]
    fn test_fuzzy_match() {
        let mut matcher = Matcher::new();
        let path = PathBuf::from("/home/user/projects/gump");

        // "gmp" should fuzzy-match "gump"
        let score = matcher.score(&path, &["gmp".to_string()]);
        assert!(score.is_some());

        // "prj" should fuzzy-match "projects"
        let score = matcher.score(&path, &["prj".to_string()]);
        assert!(score.is_some());
    }

    #[test]
    fn test_no_match() {
        let mut matcher = Matcher::new();
        let path = PathBuf::from("/home/user/projects/myapp");

        let score = matcher.score(&path, &["foobar".to_string()]);
        assert!(score.is_none());
    }

    #[test]
    fn test_last_component_boost() {
        let mut matcher = Matcher::new();
        let path1 = PathBuf::from("/home/user/myapp/src");
        let path2 = PathBuf::from("/home/user/projects/myapp");

        // "myapp" as last component should score higher
        let score1 = matcher.score(&path1, &["myapp".to_string()]);
        let score2 = matcher.score(&path2, &["myapp".to_string()]);

        assert!(score1.is_some());
        assert!(score2.is_some());
        assert!(score2.unwrap() > score1.unwrap()); // path2 has myapp as last component
    }

    #[test]
    fn test_case_insensitive() {
        let mut matcher = Matcher::new();
        let path = PathBuf::from("/home/user/MyApp");

        let score = matcher.score(&path, &["myapp".to_string()]);
        assert!(score.is_some());

        let score = matcher.score(&path, &["MYAPP".to_string()]);
        assert!(score.is_some());
    }

    #[test]
    fn test_rank() {
        let mut matcher = Matcher::new();

        let paths: Vec<(PathBuf, f64)> = vec![
            (PathBuf::from("/home/user/projects/webapp"), 10.0),
            (PathBuf::from("/home/user/projects/myapp"), 20.0),
            (PathBuf::from("/home/user/documents/myfiles"), 5.0),
        ];

        let terms = vec!["my".to_string()];
        let paths_iter = paths.iter().map(|(p, f)| (p.as_path(), *f));
        let matches = matcher.rank(paths_iter, &terms);

        // myapp should rank highest (higher frecency + matches)
        assert_eq!(matches.len(), 2); // webapp doesn't match "my"
        assert_eq!(
            matches[0].path,
            Path::new("/home/user/projects/myapp")
        );
    }
}
