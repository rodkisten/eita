//! Offline, read-only indexing of literal Zsh completion specifications.
//!
//! This is deliberately not a Zsh interpreter. It extracts conservative, static
//! _arguments option specs and simple commands=(...) lists. Completion functions
//! are never evaluated and external commands are never executed.

use crate::catalog::Candidate;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::env;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

const FORMAT_VERSION: u8 = 1;
const MAX_FILES: usize = 2_500;
const MAX_SOURCE_BYTES: u64 = 256 * 1024;
const MAX_CANDIDATES: usize = 20_000;
const MAX_CACHE_BYTES: u64 = 8 * 1024 * 1024;

#[derive(Serialize, Deserialize)]
struct Index {
    version: u8,
    scanned: usize,
    records: Vec<Candidate>,
}

#[derive(Serialize)]
pub struct IndexReport {
    pub path: String,
    pub scanned_files: usize,
    pub candidates: usize,
    pub version: u8,
}

pub fn cache_path() -> PathBuf {
    if let Some(path) = env::var_os("EITA_COMPLETION_CACHE") {
        return PathBuf::from(path);
    }
    let base = env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".cache")))
        .unwrap_or_else(env::temp_dir);
    base.join("eita/zsh-completions.json")
}

pub fn load() -> Vec<Candidate> {
    let path = cache_path();
    let Ok(metadata) = fs::metadata(&path) else {
        return Vec::new();
    };
    if metadata.len() > MAX_CACHE_BYTES {
        return Vec::new();
    }
    let Ok(bytes) = fs::read(path) else {
        return Vec::new();
    };
    match serde_json::from_slice::<Index>(&bytes) {
        Ok(index) if index.version == FORMAT_VERSION => index.records,
        _ => Vec::new(),
    }
}

pub fn status(path: &Path) -> Result<IndexReport, Box<dyn Error>> {
    let bytes = fs::read(path)?;
    let index: Index = serde_json::from_slice(&bytes)?;
    if index.version != FORMAT_VERSION {
        return Err("Cache schema incompatível; rode eita index --refresh".into());
    }
    Ok(IndexReport {
        path: path.display().to_string(),
        scanned_files: index.scanned,
        candidates: index.records.len(),
        version: index.version,
    })
}

pub fn rebuild(
    fpath_override: Option<&str>,
    destination: &Path,
) -> Result<IndexReport, Box<dyn Error>> {
    let fpath = fpath_override
        .map(str::to_owned)
        .or_else(|| env::var("EITA_FPATH").ok())
        .or_else(|| env::var("FPATH").ok())
        .ok_or("FPATH ausente. Carregue o plugin Zsh ou use --fpath /diretório")?;
    let mut scanned = 0;
    let mut records: Vec<Candidate> = Vec::new();
    let mut seen = HashSet::<String>::new();
    let mut visited = HashSet::<PathBuf>::new();

    for directory in fpath.split(':').filter(|path| !path.is_empty()).take(128) {
        let directory = PathBuf::from(directory);
        if !visited.insert(directory.clone()) {
            continue;
        }
        let Ok(entries) = fs::read_dir(directory) else {
            continue;
        };
        for entry in entries.flatten() {
            if scanned >= MAX_FILES || records.len() >= MAX_CANDIDATES {
                break;
            }
            let path = entry.path();
            let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            if !name.starts_with('_') || name.len() < 2 {
                continue;
            }
            let Ok(meta) = fs::metadata(&path) else {
                continue;
            };
            if !meta.is_file() || meta.len() > MAX_SOURCE_BYTES {
                continue;
            }
            let Ok(source) = fs::read_to_string(&path) else {
                continue;
            };
            scanned += 1;
            for record in parse_source(name, &source) {
                if records.len() >= MAX_CANDIDATES {
                    break;
                }
                if seen.insert(record.value.clone()) {
                    records.push(record);
                }
            }
        }
    }
    records.sort_by(|a, b| a.value.cmp(&b.value));
    let index = Index {
        version: FORMAT_VERSION,
        scanned,
        records,
    };
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }
    let temporary = destination.with_extension(format!("{}.tmp", std::process::id()));
    fs::write(&temporary, serde_json::to_vec(&index)?)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&temporary, fs::Permissions::from_mode(0o600))?;
    }
    fs::rename(&temporary, destination)?;
    status(destination)
}

fn parse_source(filename: &str, source: &str) -> Vec<Candidate> {
    let Some(stem) = filename.strip_prefix('_') else {
        return Vec::new();
    };
    let mut commands = Vec::new();
    for line in source.lines().take(24) {
        let trimmed = line.trim();
        if let Some(names) = trimmed.strip_prefix("#compdef ") {
            commands.extend(
                names
                    .split_whitespace()
                    .take(12)
                    .filter(is_command_name)
                    .map(str::to_owned),
            );
            break;
        }
    }
    if commands.is_empty() && valid_name(stem) {
        commands.push(stem.to_owned());
    }
    if commands.is_empty() {
        return Vec::new();
    }
    let mut result = Vec::new();
    let mut inside_commands = false;
    let mut inside_arguments = false;
    // We only handle one-line literal arguments; dynamic Zsh constructs are skipped.
    for line in source.lines().take(8_000) {
        let trimmed = line.trim_start();
        if trimmed.starts_with('#') {
            continue;
        }
        let begins_commands = trimmed.contains("commands=(")
            || trimmed.contains("subcommands=(")
            || trimmed.contains("cmds=(");
        if begins_commands {
            inside_commands = true;
        }
        if trimmed.starts_with("_arguments ") || trimmed == "_arguments" {
            inside_arguments = true;
        }
        for literal in literals(line) {
            if inside_arguments {
                if let Some((flag, description)) = option_spec(&literal) {
                    for command in &commands {
                        result.push(Candidate {
                            value: format!("{command} {flag}"),
                            description: description.to_owned(),
                            source: "zsh-completion".to_owned(),
                        });
                    }
                }
            }
            if inside_commands {
                if let Some((name, description)) = subcommand_spec(&literal) {
                    for command in &commands {
                        result.push(Candidate {
                            value: format!("{command} {name}"),
                            description: description.to_owned(),
                            source: "zsh-completion".to_owned(),
                        });
                    }
                }
            }
        }
        if inside_commands && trimmed.contains(')') {
            inside_commands = false;
        }
        inside_arguments = inside_arguments && trimmed.ends_with('\\');
        if result.len() > 1_000 {
            break;
        }
    }
    result
}

fn is_command_name(name: &&str) -> bool {
    valid_name(name)
}

fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && name
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.' | '+'))
        && !name.starts_with('-')
}

fn option_spec(literal: &str) -> Option<(&str, &str)> {
    let mut spec = literal.trim().trim_start_matches(['+', '*']);
    if spec.starts_with('(') {
        spec = spec.split_once(')')?.1;
    }
    let (before, rest) = spec.split_once('[')?;
    let description = rest.split_once(']')?.0.trim();
    let flag = before.trim_end_matches(['!', '+', '-', '=']);
    if !flag.starts_with('-') || flag.len() < 2 || flag.len() > 64 {
        return None;
    }
    if !flag[1..]
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '-')
    {
        return None;
    }
    if description.is_empty() || description.len() > 160 {
        return None;
    }
    Some((flag, description))
}

fn subcommand_spec(literal: &str) -> Option<(&str, &str)> {
    let (name, description) = literal.split_once(':')?;
    if !valid_name(name) || description.trim().len() < 3 || description.len() > 160 {
        return None;
    }
    Some((name, description.trim()))
}

fn literals(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut quote = None;
    let mut escaped = false;
    let mut current = String::new();
    for ch in line.chars() {
        if let Some(q) = quote {
            if escaped {
                current.push(ch);
                escaped = false;
            } else if q == '"' && ch == '\\' {
                escaped = true;
            } else if ch == q {
                if !current.is_empty() {
                    out.push(std::mem::take(&mut current));
                }
                quote = None;
            } else {
                current.push(ch);
            }
        } else if matches!(ch, '\'' | '"') {
            quote = Some(ch);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = include_str!("../tests/fixtures/_orbit");

    #[test]
    fn extracts_literal_flags_and_subcommands_without_evaluation() {
        let result = parse_source("_orbit", FIXTURE);
        assert!(result.iter().any(
            |item| item.value == "orbit --verbose" && item.description == "Print more details"
        ));
        assert!(result
            .iter()
            .any(|item| item.value == "orbit -q" && item.description == "Quiet mode"));
        assert!(result.iter().any(
            |item| item.value == "orbit deploy" && item.description == "Deploy an application"
        ));
        assert!(!result.iter().any(|item| item.value.contains("--dangerous")));
    }

    #[test]
    fn parses_multiple_compdef_names() {
        let text = "#compdef hello bonjour\n_arguments '--help[Show help]'";
        let result = parse_source("_hello", text);
        assert!(result.iter().any(|item| item.value == "hello --help"));
        assert!(result.iter().any(|item| item.value == "bonjour --help"));
    }

    #[test]
    fn ignores_evaluated_completion_code() {
        let text = "#compdef orbit\n_arguments \"$DYNAMIC\" \"$(evil)\"";
        assert!(parse_source("_orbit", text).is_empty());
    }

    #[test]
    fn filters_invalid_names_and_tokens() {
        assert!(option_spec("' --help'").is_none());
        assert!(option_spec("--config[Config file]").is_some());
        assert!(subcommand_spec("deploy:Deploy application").is_some());
        assert!(subcommand_spec("../evil:Path traversal").is_none());
    }
}
