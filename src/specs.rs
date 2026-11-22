use serde::{Deserialize, Serialize};
use std::{env, fs, path::PathBuf};

#[derive(Debug, Deserialize)]
pub struct CommandSpec {
    pub command: String,
    #[serde(default)]
    pub flags: Vec<FlagSpec>,
    #[serde(default)]
    pub arguments: Vec<ArgumentSpec>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct FlagSpec {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub values: Vec<String>,
    #[serde(default)]
    pub takes_value: bool,
}
#[derive(Debug, Deserialize)]
pub struct ArgumentSpec {
    pub name: String,
    #[serde(default)]
    pub values: Vec<String>,
}
#[derive(Clone, Debug)]
pub struct Completion {
    pub value: String,
    pub description: String,
}
fn spec_path() -> Option<PathBuf> {
    if let Some(path) = env::var_os("EITA_COMMAND_SPECS") { return Some(path.into()); }
    if let Some(path) = env::var_os("XDG_CONFIG_HOME") {
        return Some(PathBuf::from(path).join("eita/commands.json"));
    }
    env::var_os("HOME").map(|home| PathBuf::from(home).join(".config/eita/commands.json"))
}
pub fn load() -> Vec<CommandSpec> {
    spec_path().and_then(|path| fs::read_to_string(path).ok())
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}
/// Merge identical command paths, preferring the first source (Zsh, man, fallback).
pub fn merge(sources: impl IntoIterator<Item = CommandSpec>) -> Vec<CommandSpec> {
    let mut merged: Vec<CommandSpec> = Vec::new();
    for spec in sources {
        if let Some(existing) = merged.iter_mut().find(|item| item.command == spec.command) {
            for flag in spec.flags {
                if let Some(current) = existing.flags.iter_mut().find(|item| item.name == flag.name) {
                    if current.description.is_empty() { current.description = flag.description; }
                    current.takes_value |= flag.takes_value;
                    for value in flag.values {
                        if !current.values.contains(&value) { current.values.push(value); }
                    }
                } else { existing.flags.push(flag); }
            }
            for argument in spec.arguments {
                if !existing.arguments.iter().any(|item| item.name == argument.name) {
                    existing.arguments.push(argument);
                }
            }
        } else { merged.push(spec); }
    }
    merged
}

pub fn complete(specs: &[CommandSpec], line: &str) -> Vec<Completion> {
    let words: Vec<&str> = line.split_whitespace().collect();
    let trailing = line.chars().last().is_some_and(char::is_whitespace);
    let active = if trailing { "" } else { words.last().copied().unwrap_or("") };
    let completed = if trailing { &words[..] } else { &words[..words.len().saturating_sub(1)] };
    let mut result = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let longest_path = specs.iter().filter_map(|spec| {
        let path: Vec<&str> = spec.command.split_whitespace().collect();
        (completed.len() >= path.len() && completed[..path.len()] == path[..]).then_some(path.len())
    }).max().unwrap_or(0);
    // Suggest discovered child commands at the active path, not just options.
    for spec in specs {
        let path: Vec<&str> = spec.command.split_whitespace().collect();
        if path.len() <= completed.len() || path[..completed.len()] != completed[..] { continue; }
        let child = path[completed.len()];
        if child.starts_with(active) && seen.insert(child.to_owned()) {
            result.push(Completion { value: child.to_owned(), description: format!("Subcommand of {}", completed.join(" ")) });
        }
    }
    for spec in specs {
        let path: Vec<&str> = spec.command.split_whitespace().collect();
        if completed.len() < path.len() || completed[..path.len()] != path[..] { continue; }
        let tail = &completed[path.len()..];
        if let Some((flag_name, prefix)) = active.split_once('=') {
            if let Some(flag) = spec.flags.iter().find(|flag| flag.name == flag_name) {
                for value in &flag.values {
                    if value.starts_with(prefix) {
                        let suggestion = format!("{flag_name}={value}");
                        if seen.insert(suggestion.clone()) {
                            result.push(Completion { value: suggestion, description: flag.description.clone() });
                        }
                    }
                }
            }
            continue;
        }
        if let Some(previous) = completed.last() {
            if let Some(flag) = spec.flags.iter().find(|flag| flag.name == *previous && (flag.takes_value || !flag.values.is_empty())) {
                for value in &flag.values {
                    if value.starts_with(active) && seen.insert(value.clone()) {
                        result.push(Completion { value: value.clone(), description: format!("Value for {}", flag.name) });
                    }
                }
                continue;
            }
        }
        if active.starts_with('-') || active.is_empty() {
            for flag in &spec.flags {
                if flag.name.starts_with(active) && !tail.contains(&flag.name.as_str()) && seen.insert(flag.name.clone()) {
                    result.push(Completion { value: flag.name.clone(), description: flag.description.clone() });
                }
            }
        }
        if path.len() == longest_path && !active.starts_with('-') {
            for argument in &spec.arguments {
                for value in &argument.values {
                    if value.starts_with(active) && seen.insert(value.clone()) {
                        result.push(Completion { value: value.clone(), description: argument.name.clone() });
                    }
                }
            }
        }
    }
    result
}
#[cfg(test)]
mod tests {
    use super::*;
    fn fixtures() -> Vec<CommandSpec> {
        serde_json::from_str(r#"[{"command":"git commit","flags":[{"name":"--cleanup","values":["strip","verbatim"],"takes_value":true},{"name":"--amend"}],"arguments":[{"name":"message","values":["fix","feat"]}]}]"#).unwrap()
    }
    #[test]
    fn equals_values() {
        assert_eq!(complete(&fixtures(), "git commit --cleanup=st")[0].value, "--cleanup=strip");
    }
    #[test]
    fn separate_values() {
        assert_eq!(complete(&fixtures(), "git commit --cleanup ver")[0].value, "verbatim");
    }
    #[test]
    fn flag_scope() {
        assert_eq!(complete(&fixtures(), "git commit --am")[0].value, "--amend");
        assert!(complete(&fixtures(), "git status --am").is_empty());
    }
    #[test]
    fn inherits_global_flags() {
        let specs: Vec<CommandSpec> = serde_json::from_str(r#"[{"command":"git","flags":[{"name":"--version"}]},{"command":"git commit","flags":[{"name":"--amend"}]}]"#).unwrap();
        let values: Vec<_> = complete(&specs, "git commit --").into_iter().map(|item| item.value).collect();
        assert!(values.contains(&"--version".to_owned()));
        assert!(values.contains(&"--amend".to_owned()));
    }
    #[test]
    fn merges_sources_without_duplicate_flags() {
        let sources: Vec<CommandSpec> = serde_json::from_str(r#"[{"command":"git","flags":[{"name":"--version","description":"Zsh"}]},{"command":"git","flags":[{"name":"--version","description":"man"},{"name":"--help"}]}]"#).unwrap();
        let merged = merge(sources);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].flags.len(), 2);
        assert_eq!(merged[0].flags[0].description, "Zsh");
    }
    #[test]
    fn completes_child_commands() {
        let specs: Vec<CommandSpec> = serde_json::from_str(r#"[{"command":"git commit"},{"command":"git checkout"}]"#).unwrap();
        let values: Vec<_> = complete(&specs, "git com").into_iter().map(|item| item.value).collect();
        assert!(values.contains(&"commit".to_owned()));
        assert!(!values.contains(&"checkout".to_owned()));
    }
    #[test]
    fn positional_values() {
        assert_eq!(complete(&fixtures(), "git commit fe")[0].value, "feat");
    }
}
