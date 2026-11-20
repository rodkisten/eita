//! Exact-prefix ghost text. Fuzzy corrections are intentionally excluded.
use crate::catalog::Candidate;
use crate::context::Context;
use crate::engine;
use crate::usage::Usage;

pub fn suffix(catalog: &[Candidate], left: &str, usage: &Usage) -> Option<String> {
    if left.len() > 1024 || left.chars().count() < 3 {
        return None;
    }
    let context = Context::new(left, "");
    let query = context.query()?;
    if query.is_empty() {
        return None;
    }

    engine::suggest_ranked(catalog, query, 32, usage)
        .into_iter()
        .find_map(|suggestion| {
            let applied = context.apply(&suggestion.value);
            let suffix = applied.replacement.strip_prefix(left)?;
            if suffix.is_empty() || suffix.len() > 160 || suffix.chars().any(char::is_control) {
                return None;
            }
            Some(suffix.to_owned())
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidates() -> Vec<Candidate> {
        ["git", "git commit", "git commit --amend", "git checkout"]
            .iter()
            .map(|value| Candidate {
                value: (*value).into(),
                description: "test".into(),
                source: "builtin".into(),
            })
            .collect()
    }

    #[test]
    fn only_returns_what_can_be_appended() {
        let result = suffix(&candidates(), "git com", &Usage::default());
        assert!(result.is_some());
        assert!(result.unwrap().starts_with("mit"));
    }

    #[test]
    fn does_not_replace_typos_as_ghost_text() {
        assert!(suffix(&candidates(), "got", &Usage::default()).is_none());
    }

    #[test]
    fn preserves_the_segment_prefix() {
        let result = suffix(&candidates(), "echo hi && git com", &Usage::default());
        assert!(result.is_some());
        assert!(result.unwrap().starts_with("mit"));
    }

    #[test]
    fn refuses_unsafe_shell_context() {
        assert!(suffix(&candidates(), "git 'com", &Usage::default()).is_none());
        assert!(suffix(&candidates(), "gi", &Usage::default()).is_none());
    }
}
