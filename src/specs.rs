use serde::Deserialize;
use std::{env, fs, path::PathBuf};

#[derive(Debug, Deserialize)]
pub struct CommandSpec {
    pub command: String,
    #[serde(default)]
    pub flags: Vec<FlagSpec>,
    #[serde(default)]
    pub arguments: Vec<ArgumentSpec>,
}
#[derive(Debug, Deserialize)]
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
pub fn complete(specs: &[CommandSpec], line: &str) -> Vec<Completion> {
    let words: Vec<&str> = line.split_whitespace().collect();
    let trailing = line.ends_with(char::is_whitespace);
    let active = if trailing { "" } else { words.last().copied().unwrap_or("") };
    let completed = if trailing { &words[..] } else { &words[..words.len().saturating_sub(1)] };
    let mut result = Vec::new();
    for spec in specs {
        let path: Vec<&str> = spec.command.split_whitespace().collect();
        if completed.len() < path.len() || completed[..path.len()] != path[..] { continue; }
        let tail = &completed[path.len()..];
        // --flag=value: preserve the flag and complete its declared values.
        if let Some((flag_name, prefix)) = active.split_once('=') {
            if let Some(flag) = spec.flags.iter().find(|flag| flag.name == flag_name) {
                for value in &flag.values {
                    if value.starts_with(prefix) {
                        result.push(Completion { value: format!("{flag_name}={value}"), description: flag.description.clone() });
                    }
                }
            }
            continue;
        }
        // A value-taking flag consumes the next token.
        if let Some(previous) = tail.last() {
            if let Some(flag) = spec.flags.iter().find(|flag| flag.name == *previous && (flag.takes_value || !flag.values.is_empty())) {
                for value in &flag.values {
                    if value.starts_with(active) {
                        result.push(Completion { value: value.clone(), description: format!("Value for {}", flag.name) });
                    }
                }
                continue;
            }
        }
        if active.starts_with('-') || active.is_empty() {
            for flag in &spec.flags {
                if flag.name.starts_with(active) && !tail.contains(&flag.name.as_str()) {
                    result.push(Completion { value: flag.name.clone(), description: flag.description.clone() });
                }
            }
        }
        if !active.starts_with('-') {
            for argument in &spec.arguments {
                for value in &argument.values {
                    if value.starts_with(active) {
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
    fn positional_values() {
        assert_eq!(complete(&fixtures(), "git commit fe")[0].value, "feat");
    }
}
