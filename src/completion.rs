//! Offline, read-only indexing of literal Zsh completion specifications.
//!
//! This is deliberately not a Zsh interpreter. It extracts conservative, static
//! _arguments option specs and simple commands=(...) lists. Completion functions
//! are never evaluated and external commands are never executed.

use crate::catalog::Candidate;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::env;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

const FORMAT_VERSION: u8 = 2;
const MAX_FILES: usize = 2_500;
const MAX_SOURCE_BYTES: u64 = 256 * 1024;
const MAX_CANDIDATES: usize = 20_000;
const MAX_CACHE_BYTES: u64 = 8 * 1024 * 1024;

#[derive(Clone, Serialize, Deserialize)]
struct Snapshot {
    path: String,
    size: u64,
    modified_ns: u64,
    records: Vec<Candidate>,
}

#[derive(Serialize, Deserialize)]
struct Index {
    version: u8,
    scanned: usize,
    records: Vec<Candidate>,
    #[serde(default)]
    files: Vec<Snapshot>,
    #[serde(default)]
    fpath: Option<String>,
    #[serde(default)]
    manpath: Option<String>,
}

#[derive(Serialize)]
pub struct IndexReport {
    pub path: String,
    pub scanned_files: usize,
    pub candidates: usize,
    pub version: u8,
    pub reused_files: usize,
    pub updated_files: usize,
    pub removed_files: usize,
    pub unchanged: bool,
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
        Ok(index) if index.version == FORMAT_VERSION || index.version == 1 => index.records,
        _ => Vec::new(),
    }
}

/// Return original index roots without rescanning any source directories.
pub fn indexed_sources() -> (Option<String>, Option<String>) {
    let path = cache_path();
    let bytes = fs::metadata(&path)
        .ok()
        .filter(|meta| meta.len() <= MAX_CACHE_BYTES)
        .and_then(|_| fs::read(path).ok());
    let index = bytes.and_then(|bytes| serde_json::from_slice::<Index>(&bytes).ok());
    index.map_or((None, None), |index| (index.fpath, index.manpath))
}

pub fn status(path: &Path) -> Result<IndexReport, Box<dyn Error>> {
    let bytes = fs::read(path)?;
    let index: Index = serde_json::from_slice(&bytes)?;
    if index.version != FORMAT_VERSION && index.version != 1 {
        return Err("Unknown cache schema; run eita index --refresh".into());
    }
    Ok(IndexReport {
        path: path.display().to_string(),
        scanned_files: index.scanned,
        candidates: index.records.len(),
        version: index.version,
        reused_files: 0,
        updated_files: 0,
        removed_files: 0,
        unchanged: true,
    })
}

#[derive(Clone, Copy)]
enum Source {
    Zsh,
    Man,
}

fn source_files(fpath: Option<&str>, manpath: Option<&str>) -> Vec<(PathBuf, Source)> {
    let mut result = Vec::new();
    for (paths, kind) in [(fpath, Source::Zsh), (manpath, Source::Man)] {
        // The same directory may provide both _completion and foo.1 sources.
        let mut seen_dirs = HashSet::new();
        let Some(paths) = paths else { continue };
        for root in paths.split(':').filter(|path| !path.is_empty()).take(128) {
            let root = PathBuf::from(root);
            let dirs = if matches!(kind, Source::Man) {
                vec![root.clone(), root.join("man1"), root.join("man8")]
            } else {
                vec![root]
            };
            for directory in dirs {
                if !seen_dirs.insert(directory.clone()) {
                    continue;
                }
                let Ok(entries) = fs::read_dir(directory) else {
                    continue;
                };
                let mut entries: Vec<_> = entries.flatten().map(|item| item.path()).collect();
                entries.sort();
                for path in entries {
                    if result.len() >= MAX_FILES {
                        break;
                    }
                    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                        continue;
                    };
                    let wanted = match kind {
                        Source::Zsh => name.starts_with('_') && name.len() > 1,
                        Source::Man => crate::man::command_name(name).is_some(),
                    };
                    if wanted {
                        result.push((path, kind));
                    }
                }
            }
        }
    }
    result
}

fn timestamp(meta: &fs::Metadata) -> u64 {
    meta.modified()
        .ok()
        .and_then(|mtime| mtime.duration_since(UNIX_EPOCH).ok())
        .and_then(|duration| u64::try_from(duration.as_nanos()).ok())
        .unwrap_or(0)
}

fn report(
    destination: &Path,
    scanned: usize,
    candidates: usize,
    reused: usize,
    updated: usize,
    removed: usize,
    unchanged: bool,
) -> IndexReport {
    IndexReport {
        path: destination.display().to_string(),
        scanned_files: scanned,
        candidates,
        version: FORMAT_VERSION,
        reused_files: reused,
        updated_files: updated,
        removed_files: removed,
        unchanged,
    }
}

pub fn rebuild(
    fpath_override: Option<&str>,
    man_override: Option<&str>,
    destination: &Path,
) -> Result<IndexReport, Box<dyn Error>> {
    let old: Option<Index> = fs::metadata(destination)
        .ok()
        .filter(|meta| meta.len() <= MAX_CACHE_BYTES)
        .and_then(|_| fs::read(destination).ok())
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .filter(|index: &Index| index.version == FORMAT_VERSION);
    let fpath = fpath_override
        .map(str::to_owned)
        .or_else(|| env::var("EITA_FPATH").ok())
        .or_else(|| env::var("FPATH").ok())
        .or_else(|| old.as_ref().and_then(|cache| cache.fpath.clone()));
    // Preserve explicit manpage opt-in in the index across future auto refreshes.
    let manpath = man_override
        .map(str::to_owned)
        .or_else(|| env::var("EITA_MANPATH").ok())
        .or_else(|| old.as_ref().and_then(|cache| cache.manpath.clone()));
    let sources_unchanged = old
        .as_ref()
        .is_some_and(|cached| cached.fpath == fpath && cached.manpath == manpath);
    if fpath.is_none() && manpath.is_none() {
        return Err("No FPATH or man directory; pass --fpath or --man-dir".into());
    }

    let old_count = old.as_ref().map_or(0, |index| index.files.len());
    let previous: HashMap<String, Snapshot> = old
        .map(|index| {
            index
                .files
                .into_iter()
                .map(|item| (item.path.clone(), item))
                .collect()
        })
        .unwrap_or_default();

    let mut files = Vec::new();
    let mut reused = 0;
    let mut updated = 0;
    let mut all_records = Vec::new();
    let mut seen_values = HashSet::new();
    let mut seen_paths = HashSet::new();
    for (path, source) in source_files(fpath.as_deref(), manpath.as_deref()) {
        if all_records.len() >= MAX_CANDIDATES {
            break;
        }
        let Ok(meta) = fs::symlink_metadata(&path) else {
            continue;
        };
        if !meta.file_type().is_file() || meta.len() > MAX_SOURCE_BYTES {
            continue;
        }
        let key = path.to_string_lossy().into_owned();
        if !seen_paths.insert(key.clone()) {
            continue;
        }
        let stamp = timestamp(&meta);
        let (records, reused_file) = if let Some(cached) = previous.get(&key) {
            if cached.size == meta.len() && cached.modified_ns == stamp {
                (cached.records.clone(), true)
            } else {
                (parse_file(&path, source), false)
            }
        } else {
            (parse_file(&path, source), false)
        };
        if reused_file {
            reused += 1;
        } else {
            updated += 1;
        }
        for record in &records {
            if all_records.len() >= MAX_CANDIDATES {
                break;
            }
            if seen_values.insert(record.value.clone()) {
                all_records.push(record.clone());
            }
        }
        files.push(Snapshot {
            path: key,
            size: meta.len(),
            modified_ns: stamp,
            records,
        });
    }
    // Compare exact key sets instead of relying on directory iteration order.
    let removed = previous
        .keys()
        .filter(|key| !seen_paths.contains(*key))
        .count();
    let scanned = files.len();
    let unchanged = updated == 0 && removed == 0 && old_count == scanned && sources_unchanged;
    if unchanged {
        return Ok(report(
            destination,
            scanned,
            all_records.len(),
            reused,
            0,
            0,
            true,
        ));
    }

    all_records.sort_by(|a, b| a.value.cmp(&b.value));
    let index = Index {
        version: FORMAT_VERSION,
        scanned,
        records: all_records,
        files,
        fpath,
        manpath,
    };
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }
    let temporary = destination.with_extension(format!("{}.tmp", std::process::id()));
    let encoded = serde_json::to_vec(&index)?;
    if encoded.len() as u64 > MAX_CACHE_BYTES {
        return Err("Completion cache exceeds 8 MiB; reduce indexed directories".into());
    }
    fs::write(&temporary, encoded)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&temporary, fs::Permissions::from_mode(0o600))?;
    }
    fs::rename(&temporary, destination)?;
    Ok(report(
        destination,
        scanned,
        index.records.len(),
        reused,
        updated,
        removed,
        false,
    ))
}

fn parse_file(path: &Path, source: Source) -> Vec<Candidate> {
    match source {
        Source::Zsh => path
            .file_name()
            .and_then(|n| n.to_str())
            .and_then(|name| {
                fs::read_to_string(path)
                    .ok()
                    .map(|text| parse_source(name, &text))
            })
            .unwrap_or_default(),
        Source::Man => crate::man::read(path).unwrap_or_default(),
    }
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
                            description: description.clone(),
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

// Static _arguments subset: --output=[description]:label:_files.
// No function evaluation or shell execution.
fn option_spec(literal: &str) -> Option<(String, String)> {
    let mut spec = literal.trim().trim_start_matches(['+', '*']);
    if spec.starts_with('(') {
        spec = spec.split_once(')')?.1;
    }
    let (before, rest) = spec.split_once('[')?;
    let (description, tail) = rest.split_once(']')?;
    let description = description.trim();
    let name = before.trim_end_matches(['!', '+', '-']);
    let requires_value = name.ends_with('=') || (tail.starts_with(':') && !tail.starts_with("::"));
    let flag = name.trim_end_matches('=');
    if !flag.starts_with('-')
        || flag.len() < 2
        || flag.len() > 64
        || !flag[1..]
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '-')
    {
        return None;
    }
    if description.is_empty()
        || description.len() > 160
        || description.chars().any(char::is_control)
        || literal.contains(['$', '\u{60}'])
    {
        return None;
    }
    let mut help = description.to_owned();
    if requires_value {
        let kind = tail
            .trim_start_matches(':')
            .split(':')
            .next()
            .unwrap_or("value");
        let kind = if kind.len() <= 20
            && !kind.is_empty()
            && kind
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || ch == '-')
        {
            kind
        } else {
            "value"
        };
        help.push_str(&format!(" · expects <{kind}>"));
    }
    Some((
        format!("{flag}{}", if name.ends_with('=') { "=" } else { "" }),
        help,
    ))
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
    fn incremental_index_reuses_unchanged_sources_and_detects_removal() {
        let root = std::env::temp_dir().join(format!(
            "eita-index-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("_orbit"), FIXTURE).unwrap();
        let destination = root.join("index.json");
        let directory = root.to_str().unwrap();
        let first = rebuild(Some(directory), None, &destination).unwrap();
        assert_eq!(first.updated_files, 1);
        assert_eq!(first.reused_files, 0);
        let second = rebuild(Some(directory), None, &destination).unwrap();
        assert!(second.unchanged);
        assert_eq!(second.reused_files, 1);
        assert_eq!(second.updated_files, 0);
        fs::remove_file(root.join("_orbit")).unwrap();
        let third = rebuild(Some(directory), None, &destination).unwrap();
        assert_eq!(third.removed_files, 1);
        assert_eq!(third.candidates, 0);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn imports_manpages_only_from_explicit_directory() {
        let root = std::env::temp_dir().join(format!(
            "eita-man-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(root.join("man1")).unwrap();
        fs::write(
            root.join("man1/orbit.1"),
            include_str!("../tests/fixtures/orbit.1"),
        )
        .unwrap();
        let destination = root.join("index.json");
        let first = rebuild(None, Some(root.to_str().unwrap()), &destination).unwrap();
        assert!(first.updated_files >= 1);
        assert!(load_records(&destination)
            .iter()
            .any(|item| item.value == "orbit --verbose"));
        fs::remove_dir_all(root).unwrap();
    }

    fn load_records(path: &Path) -> Vec<Candidate> {
        let encoded = fs::read(path).unwrap();
        serde_json::from_slice::<Index>(&encoded).unwrap().records
    }

    #[test]
    fn preserves_flags_that_require_a_value() {
        let found = parse_source("_orbit", FIXTURE);
        let output = found
            .iter()
            .find(|item| item.value == "orbit --output=")
            .unwrap();
        assert!(output.description.contains("expects <path>"));
        assert!(option_spec("--evil[Injected]$(touch /tmp/hacked)").is_none());
        let flag = option_spec("--format[Select format]:format:(json yaml)").unwrap();
        assert_eq!(flag.0, "--format");
        assert!(flag.1.contains("expects <format>"));
    }

    #[test]
    fn filters_invalid_names_and_tokens() {
        assert!(option_spec("' --help'").is_none());
        assert!(option_spec("--config[Config file]").is_some());
        assert!(subcommand_spec("deploy:Deploy application").is_some());
        assert!(subcommand_spec("../evil:Path traversal").is_none());
    }
}
