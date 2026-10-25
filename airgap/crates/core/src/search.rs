use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use rayon::prelude::*;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuickOpenMatch {
    pub path: PathBuf,
    pub score: i64,
}

/// Fuzzy quick open matching across relative note paths.
pub fn quick_open_search(entries: &[PathBuf], query: &str) -> Vec<QuickOpenMatch> {
    if query.trim().is_empty() {
        return entries
            .iter()
            .map(|p| QuickOpenMatch {
                path: p.clone(),
                score: 0,
            })
            .collect();
    }

    let matcher = SkimMatcherV2::default();
    let mut matches: Vec<QuickOpenMatch> = entries
        .iter()
        .filter_map(|p| {
            let path_str = p.to_string_lossy();
            matcher
                .fuzzy_match(&path_str, query)
                .map(|score| QuickOpenMatch {
                    path: p.clone(),
                    score,
                })
        })
        .collect();

    // Sort descending by score
    matches.sort_by_key(|a| std::cmp::Reverse(a.score));
    matches
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchResult {
    pub relative_path: PathBuf,
    pub line_number: usize,
    pub line_content: String,
}

/// Multi-threaded full-text search across all notes in a vault.
pub fn global_search(vault_root: &Path, query: &str, notes: &[PathBuf]) -> Vec<SearchResult> {
    if query.is_empty() {
        return Vec::new();
    }

    let query_lower = query.to_lowercase();

    notes
        .par_iter()
        .flat_map(|rel_path| {
            let abs_path = vault_root.join(rel_path);
            let mut results = Vec::new();

            if let Ok(content) = fs::read_to_string(&abs_path) {
                for (idx, line) in content.lines().enumerate() {
                    if line.to_lowercase().contains(&query_lower) {
                        results.push(SearchResult {
                            relative_path: rel_path.clone(),
                            line_number: idx + 1,
                            line_content: line.trim().to_string(),
                        });
                    }
                }
            }

            results
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quick_open_search() {
        let entries = vec![
            PathBuf::from("Notes/Rust_Guide.md"),
            PathBuf::from("Work/Meeting.md"),
            PathBuf::from("Journal/2026-10-24.md"),
        ];

        let results = quick_open_search(&entries, "rust");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].path, PathBuf::from("Notes/Rust_Guide.md"));

        let empty_results = quick_open_search(&entries, "");
        assert_eq!(empty_results.len(), 3);
    }
}
