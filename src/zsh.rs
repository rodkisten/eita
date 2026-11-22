//! Passive Zsh completion inspection. Does not source or execute completion functions.
use crate::specs::{CommandSpec, FlagSpec};
use std::{collections::HashSet, env, fs, path::PathBuf, time::UNIX_EPOCH};
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
struct Cache { fingerprint: String, flags: Vec<FlagSpec> }

fn cache_file(root: &str) -> Option<PathBuf> {
    let base = env::var_os("EITA_CACHE_DIR").map(PathBuf::from)
        .or_else(|| env::var_os("XDG_CACHE_HOME").map(|p| PathBuf::from(p).join("eita")))
        .or_else(|| env::var_os("HOME").map(|p| PathBuf::from(p).join(".cache/eita")))?;
    Some(base.join(format!("zsh-{root}.json")))
}


fn directories() -> Vec<PathBuf> {
    let mut paths: Vec<PathBuf> = env::var_os("FPATH")
        .map(|value| env::split_paths(&value).collect()).unwrap_or_default();
    if paths.is_empty() {
        paths.extend(["/usr/share/zsh/functions/Completion", "/usr/local/share/zsh/site-functions", "/usr/share/zsh/site-functions", "/opt/homebrew/share/zsh/site-functions"].map(PathBuf::from));
    }
    paths
}
fn find_file(root: &str, directories: &[PathBuf]) -> Option<PathBuf> {
    for dir in directories {
        let direct = dir.join(format!("_{root}"));
        if direct.is_file() { return Some(direct); }
        // Common distributions organize completions into category subdirectories.
        let Ok(entries) = fs::read_dir(dir) else { continue };
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() { continue; }
            let candidate = path.join(format!("_{root}"));
            if candidate.is_file() { return Some(candidate); }
        }
    }
    None
}
fn extract(source: &str) -> Vec<FlagSpec> {
    let mut result = Vec::new();
    let mut seen = HashSet::new();
    // Extract quoted _arguments specifications, not arbitrary shell code.
    // Handles '--flag[description]', '--flag=-[description]' and '-f[description]'.
    for line in source.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') { continue; }
        for quote in ['\'', '"'] {
            for segment in trimmed.split(quote).skip(1).step_by(2) {
                let item = segment.trim_start_matches(|c| c == '*' || c == '+' || c == '(');
                if !item.starts_with('-') { continue; }
                let name = item.split(|c: char| c == '[' || c == '=' || c == ':' || c == ' ' || c == ')')
                    .next().unwrap_or("");
                if name.len() < 2 || !name.trim_start_matches('-').chars().all(|c| c.is_ascii_alphanumeric() || c == '-') { continue; }
                if !seen.insert(name.to_owned()) { continue; }
                let description = item.split_once('[').and_then(|(_, rest)| rest.split_once(']')).map(|(desc, _)| desc).unwrap_or("Zsh completion");
                result.push(FlagSpec { name: name.into(), description: description.into(), values: vec![], takes_value: item.contains('=') || item.contains(':') });
            }
        }
    }
    result
}
pub fn learn(root: &str) -> Vec<CommandSpec> {
    if root.is_empty() || !root.chars().all(|c| c.is_ascii_alphanumeric() || "-_+.".contains(c)) { return Vec::new(); }
    let Some(path) = find_file(root, &directories()) else { return Vec::new() };
    let Ok(metadata) = fs::metadata(&path) else { return Vec::new() };
    if metadata.len() > 2 * 1024 * 1024 { return Vec::new(); }
    let modified = metadata.modified().ok().and_then(|time| time.duration_since(UNIX_EPOCH).ok()).map(|time| time.as_nanos()).unwrap_or_default();
    let fingerprint = format!("{}:{}:{}", path.display(), metadata.len(), modified);
    let cache = cache_file(root);
    if let Some(ref file) = cache {
        if let Ok(bytes) = fs::read(file) {
            if let Ok(saved) = serde_json::from_slice::<Cache>(&bytes) {
                if saved.fingerprint == fingerprint {
                    return vec![CommandSpec { command: root.into(), flags: saved.flags, arguments: vec![] }];
                }
            }
        }
    }
    let Ok(source) = fs::read_to_string(&path) else { return Vec::new() };
    let flags = extract(&source);
    if let Some(file) = cache {
        if let Some(dir) = file.parent() {
            if fs::create_dir_all(dir).is_ok() {
                if let Ok(bytes) = serde_json::to_vec(&Cache { fingerprint, flags: flags.clone() }) {
                    let temp = file.with_extension(format!("{}.tmp", std::process::id()));
                    if fs::write(&temp, bytes).is_ok() { let _ = fs::rename(&temp, &file); }
                }
            }
        }
    }
    if flags.is_empty() { return Vec::new(); }
    vec![CommandSpec { command: root.into(), flags, arguments: vec![] }]
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_arguments_without_running_shell() {
        let result = extract("_arguments '--amend[Amend previous commit]' '--cleanup=-[Cleanup mode]' '-m+[Message]:message:'");
        assert!(result.iter().any(|flag| flag.name == "--amend"));
        assert!(result.iter().any(|flag| flag.name == "--cleanup" && flag.takes_value));
    }
    #[test]
    fn parses_value_taking_options() {
        let flags = extract("_arguments '--output=[Output file]:file:' '--verbose[Verbose]'");
        assert!(flags.iter().any(|flag| flag.name == "--output" && flag.takes_value));
        assert!(flags.iter().any(|flag| flag.name == "--verbose" && !flag.takes_value));
    }
    #[test]
    fn ignores_comments() {
        assert!(extract("# '--danger[not active]'").is_empty());
    }
}
