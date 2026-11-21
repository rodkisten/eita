//! Read-only typo recovery against real Zsh symbols supplied as TSV.
use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Clone, Debug, Serialize)]
pub struct Correction {
    pub name: String,
    pub kind: String,
    pub description: String,
    pub distance: usize,
    pub score: i64,
}

fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 96
        && !name.chars().any(|ch| {
            ch.is_whitespace()
                || ch.is_control()
                || matches!(
                    ch as u32,
                    0x2f | 0x5c | 0x24 | 0x3b | 0x26 | 0x7c | 0x27 | 0x22 | 0x60
                )
        })
}

fn kind_allowed(kind: &str, requested: &str) -> bool {
    matches!(
        kind,
        "command" | "alias" | "function" | "builtin" | "reserved" | "variable"
    ) && (requested == "all" || (requested == "command" && kind != "variable") || requested == kind)
}

pub fn read_symbols(path: &Path) -> Result<Vec<Correction>, Box<dyn std::error::Error>> {
    const MAX_BYTES: u64 = 4 * 1024 * 1024;
    if fs::metadata(path)?.len() > MAX_BYTES {
        return Err("Symbol list exceeds the 4 MiB limit".into());
    }
    let mut result = Vec::new();
    for line in fs::read_to_string(path)?.lines().take(50_000) {
        let mut parts = line.splitn(3, '\t');
        let (Some(kind), Some(name), Some(description)) =
            (parts.next(), parts.next(), parts.next())
        else {
            continue;
        };
        if !valid_name(name) || !kind_allowed(kind, "all") {
            continue;
        }
        result.push(Correction {
            name: name.to_owned(),
            kind: kind.to_owned(),
            description: description
                .chars()
                .filter(|ch| !ch.is_control())
                .take(100)
                .collect(),
            distance: 0,
            score: 0,
        });
    }
    Ok(result)
}

fn osa_distance(left: &str, right: &str) -> usize {
    let a: Vec<char> = left.to_lowercase().chars().collect();
    let b: Vec<char> = right.to_lowercase().chars().collect();
    let n = a.len();
    let m = b.len();
    let mut matrix = vec![vec![0; m + 1]; n + 1];
    for (i, row) in matrix.iter_mut().enumerate() {
        row[0] = i;
    }
    for (j, cell) in matrix[0].iter_mut().enumerate() {
        *cell = j;
    }
    for i in 1..=n {
        for j in 1..=m {
            let substitution = usize::from(a[i - 1] != b[j - 1]);
            let mut distance = (matrix[i - 1][j] + 1)
                .min(matrix[i][j - 1] + 1)
                .min(matrix[i - 1][j - 1] + substitution);
            if i > 1 && j > 1 && a[i - 1] == b[j - 2] && a[i - 2] == b[j - 1] {
                distance = distance.min(matrix[i - 2][j - 2] + 1);
            }
            matrix[i][j] = distance;
        }
    }
    matrix[n][m]
}

fn kind_bonus(kind: &str) -> i64 {
    match kind {
        "alias" | "function" => 16,
        "command" => 12,
        "builtin" => 10,
        "reserved" => 8,
        "variable" => 9,
        _ => 0,
    }
}

pub fn recover(
    typed: &str,
    symbols: &[Correction],
    requested_kind: &str,
    limit: usize,
) -> Vec<Correction> {
    let word = typed.strip_prefix('$').unwrap_or(typed);
    if !valid_name(word) || limit == 0 || word.chars().count() <= 2 {
        return Vec::new();
    }
    let max_distance = match word.chars().count() {
        0..=4 => 1,
        5..=8 => 2,
        _ => 3,
    };
    let mut merged = HashMap::<String, Correction>::new();
    for item in symbols {
        if !kind_allowed(&item.kind, requested_kind) {
            continue;
        }
        let distance = osa_distance(word, &item.name);
        if distance > max_distance || distance == 0 {
            if distance == 0 {
                return Vec::new();
            }
            continue;
        }
        let prefix = i64::from(item.name.chars().next() == word.chars().next());
        let score =
            300 - i64::try_from(distance).unwrap_or(300) * 90 + kind_bonus(&item.kind) + prefix * 5;
        let mut found = item.clone();
        found.distance = distance;
        found.score = score;
        match merged.get(&found.name) {
            Some(previous) if previous.score >= found.score => {}
            _ => {
                merged.insert(found.name.clone(), found);
            }
        }
    }
    let mut matches: Vec<_> = merged.into_values().collect();
    matches.sort_by(|a, b| {
        a.distance
            .cmp(&b.distance)
            .then_with(|| b.score.cmp(&a.score))
            .then_with(|| a.name.cmp(&b.name))
    });
    matches.truncate(limit.min(15));
    matches
}

#[cfg(test)]
mod tests {
    use super::*;

    fn items() -> Vec<Correction> {
        [
            ("command", "git"),
            ("command", "docker"),
            ("alias", "gco"),
            ("function", "deploy"),
            ("variable", "EDITOR"),
            ("builtin", "echo"),
        ]
        .iter()
        .map(|(kind, name)| Correction {
            name: (*name).into(),
            kind: (*kind).into(),
            description: "known".into(),
            distance: 0,
            score: 0,
        })
        .collect()
    }

    #[test]
    fn corrects_transposed_and_substituted_commands() {
        assert_eq!(recover("gti", &items(), "command", 5)[0].name, "git");
        assert_eq!(recover("got", &items(), "command", 5)[0].name, "git");
        assert_eq!(recover("ecoh", &items(), "command", 5)[0].name, "echo");
        assert_eq!(recover("dcoker", &items(), "command", 5)[0].name, "docker");
    }

    #[test]
    fn respects_known_and_missing_symbols() {
        assert!(recover("git", &items(), "command", 5).is_empty());
        assert!(recover("zzzzz", &items(), "command", 5).is_empty());
        assert!(recover("gi", &items(), "command", 5).is_empty());
    }

    #[test]
    fn reports_alias_function_variable_without_contents() {
        let aliases = recover("gcp", &items(), "command", 4);
        assert!(aliases
            .iter()
            .any(|item| item.name == "gco" && item.kind == "alias"));
        let vars = recover("$EDTIOR", &items(), "variable", 4);
        assert_eq!(vars.first().map(|item| item.name.as_str()), Some("EDITOR"));
        assert!(recover("EDTIOR", &items(), "command", 4).is_empty());
    }

    #[test]
    fn does_not_accept_shell_metacharacters() {
        assert!(recover("foo;bar", &items(), "command", 5).is_empty());
    }

    #[test]
    fn two_edits_can_match_when_typo_has_five_chars() {
        let symbols = vec![Correction {
            name: "search".into(),
            kind: "command".into(),
            description: "find".into(),
            distance: 0,
            score: 0,
        }];
        assert_eq!(recover("earxh", &symbols, "command", 5)[0].name, "search");
    }
}
