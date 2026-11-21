use crate::catalog::Candidate;
use frizbee::{Config, Matcher};
use serde::Serialize;
use std::cmp::Reverse;

#[derive(Clone, Debug, Serialize)]
pub struct Suggestion {
    pub value: String,
    pub description: String,
    pub source: String,
    pub score: i64,
}

/// Suggest full replacement lines. The picker never runs a proposed command.
pub fn suggest(catalog: &[Candidate], line: &str, limit: usize) -> Vec<Suggestion> {
    if limit == 0 {
        return Vec::new();
    }
    let query = line.trim_start();
    let root = query.split_whitespace().next().unwrap_or("");
    let known_root = !root.is_empty() && catalog.iter().any(|candidate| candidate.root() == root);
    // Match the active token inside its parent command path rather than fuzzy
    // matching the entire suffix. Preserve earlier subcommands and flags.
    let tokens: Vec<&str> = query.split_whitespace().collect();
    let trailing_space = query.ends_with(' ');
    let parent_count = if trailing_space { tokens.len() } else { tokens.len().saturating_sub(1) };
    let parent = if known_root { tokens[..parent_count].join(" ") } else { String::new() };
    let active = if trailing_space { "" } else { tokens.last().copied().unwrap_or("") };
    let scoped_query = if known_root { active } else { query };

    let pool: Vec<&Candidate> = catalog
        .iter()
        .filter(|candidate| {
            if known_root {
                candidate.root() == root
                    && candidate.value != parent
                    && (parent.is_empty() || candidate.value.starts_with(&format!("{parent} ")))
            } else {
                !candidate.value.contains(' ')
            }
        })
        .collect();

    if pool.is_empty() {
        return Vec::new();
    }

    let labels: Vec<&str> = pool
        .iter()
        .map(|candidate| {
            if known_root {
                candidate
                    .value
                    .strip_prefix(&parent)
                    .unwrap_or("")
                    .trim_start()
            } else {
                candidate.value.as_str()
            }
        })
        .collect();

    let fuzzy_matches: Vec<(usize, u16, bool)> = if scoped_query.is_empty() {
        (0..labels.len()).map(|index| (index, 0, false)).collect()
    } else {
        let config = Config {
            max_typos: Some(if scoped_query.chars().count() < 3 {
                0
            } else {
                2
            }),
            ..Config::default()
        };
        let mut matcher = Matcher::new(scoped_query, &config);
        matcher
            .match_list(&labels)
            .into_iter()
            .map(|matched| (matched.index as usize, matched.score, matched.exact))
            .collect()
    };

    let mut result: Vec<Suggestion> = fuzzy_matches
        .into_iter()
        .filter_map(|(index, fuzzy_score, exact)| {
            let candidate = pool.get(index)?;
            let label = labels[index];
            let mut score = i64::from(fuzzy_score);
            if exact {
                score += 2_000;
            }
            if !scoped_query.is_empty() && label.starts_with(scoped_query) {
                score += 300;
            }
            if candidate.source == "user" {
                score += 25;
            }
            score -= (label.len().saturating_sub(scoped_query.len()) as i64) / 4;
            Some(Suggestion {
                value: candidate.value.clone(),
                description: candidate.description.clone(),
                source: candidate.source.clone(),
                score,
            })
        })
        .collect();

    result.sort_by_key(|item| (Reverse(item.score), item.value.len(), item.value.clone()));
    result.truncate(limit);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidates() -> Vec<Candidate> {
        ["git", "git commit --amend", "git checkout", "docker"]
            .iter()
            .map(|value| Candidate {
                value: (*value).into(),
                description: (*value).into(),
                source: "builtin".into(),
            })
            .collect()
    }

    #[test]
    fn ranks_commit_in_git_namespace() {
        let result = suggest(&candidates(), "git com", 10);
        assert_eq!(result.first().unwrap().value, "git commit --amend");
    }

    #[test]
    fn fixes_mistyped_root() {
        let result = suggest(&candidates(), "got", 10);
        assert!(result.iter().any(|item| item.value == "git"));
    }

    #[test]
    fn understands_nested_git_flags() {
        let result = suggest(&candidates(), "git commit --am", 10);
        assert_eq!(result.first().unwrap().value, "git commit --amend");
    }

    #[test]
    fn does_not_cross_subcommand_boundaries() {
        let result = suggest(&candidates(), "git checkout ", 10);
        assert!(result.iter().all(|item| item.value.starts_with("git checkout ")));
    }

    #[test]
    fn bounds_results() {
        assert!(suggest(&candidates(), "", 2).len() <= 2);
        assert!(suggest(&candidates(), "git", 0).is_empty());
    }

    #[test]
    fn stays_in_known_root_namespace() {
        let result = suggest(&candidates(), "git ", 10);
        assert!(result.iter().all(|item| item.value.starts_with("git ")));
    }
}
