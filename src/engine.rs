use crate::catalog::Candidate;
use crate::usage::Usage;
use frizbee::{Config, Matcher};
use serde::{Deserialize, Serialize};
use std::cmp::Reverse;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Suggestion {
    pub value: String,
    pub description: String,
    pub source: String,
    pub score: i64,
}

/// Suggest full replacement lines. The picker never runs a proposed command.
pub fn suggest(catalog: &[Candidate], line: &str, limit: usize) -> Vec<Suggestion> {
    suggest_ranked(catalog, line, limit, &Usage::default())
}

pub fn suggest_ranked(
    catalog: &[Candidate],
    line: &str,
    limit: usize,
    usage: &Usage,
) -> Vec<Suggestion> {
    if limit == 0 {
        return Vec::new();
    }
    let query = line.trim_start();
    let root = query.split_whitespace().next().unwrap_or("");
    let known_root = !root.is_empty() && catalog.iter().any(|candidate| candidate.root() == root);
    let scoped_query = if known_root {
        query.strip_prefix(root).unwrap_or("").trim_start()
    } else {
        query
    };

    let pool: Vec<&Candidate> = catalog
        .iter()
        .filter(|candidate| {
            if known_root {
                candidate.root() == root && !(query.ends_with(' ') && candidate.value == root)
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
                    .strip_prefix(root)
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
            if candidate.source == "zsh-completion" {
                score += 15;
            }
            score += usage.bonus(&candidate.value);
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
    fn bounds_results() {
        assert!(suggest(&candidates(), "", 2).len() <= 2);
        assert!(suggest(&candidates(), "git", 0).is_empty());
    }

    #[test]
    fn maintains_exact_prefix_priority() {
        let result = suggest(&candidates(), "git comm", 5);
        assert!(result.first().unwrap().value.starts_with("git commit"));
    }

    #[test]
    fn stays_in_known_root_namespace() {
        let result = suggest(&candidates(), "git ", 10);
        assert!(result.iter().all(|item| item.value.starts_with("git ")));
    }
}
