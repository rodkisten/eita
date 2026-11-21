//! Passive local documentation discovery. Never executes the target command.
use crate::specs::{CommandSpec, FlagSpec};
use flate2::read::GzDecoder;
use serde::{Deserialize, Serialize};
use std::{env, fs, io::Read, path::{Path, PathBuf}, time::UNIX_EPOCH};

#[derive(Serialize, Deserialize)]
struct Cached {
    fingerprint: String,
    flags: Vec<FlagSpec>,
}
fn cache_dir() -> Option<PathBuf> {
    if let Some(p) = env::var_os("EITA_CACHE_DIR") { return Some(p.into()); }
    if let Some(p) = env::var_os("XDG_CACHE_HOME") { return Some(PathBuf::from(p).join("eita")); }
    env::var_os("HOME").map(|p| PathBuf::from(p).join(".cache/eita"))
}
fn read_page(path: &Path) -> Option<String> {
    let file = fs::File::open(path).ok()?;
    let mut bytes = Vec::new();
    if path.extension().is_some_and(|ext| ext == "gz") {
        GzDecoder::new(file).take(4 * 1024 * 1024).read_to_end(&mut bytes).ok()?;
    } else {
        file.take(4 * 1024 * 1024).read_to_end(&mut bytes).ok()?;
    }
    String::from_utf8(bytes).ok()
}
fn extract_flags(source: &str) -> Vec<FlagSpec> {
    let mut flags = Vec::new();
    for line in source.lines() {
        let raw = line.trim();
        let raw = [".B ", ".BI ", ".BR ", ".IR ", ".I ", ".TP ", ".It Fl "]
            .iter().find_map(|prefix| raw.strip_prefix(prefix));
        let Some(raw) = raw else { continue };
        let normalized = raw.replace(r"\-", "-").replace(r"\fB", "").replace(r"\fR", "");
        let raw = normalized.trim().trim_matches('"');
        let raw = if let Some(flag) = raw.strip_prefix("--") { format!("--{flag}") }
            else if line.trim().starts_with(".It Fl ") { format!("-{raw}") }
            else { raw.to_owned() };
        let flag = raw.split(|c: char| c.is_whitespace() || c == '=' || c == '[' || c == ',' || c == '\\').next().unwrap_or("");
        if !flag.starts_with('-') || flag.len() < 2 || !flag.trim_start_matches('-').chars().all(|c| c.is_ascii_alphanumeric() || c == '-') { continue; }
        if !flags.iter().any(|f: &FlagSpec| f.name == flag) {
            flags.push(FlagSpec { name: flag.to_owned(), description: "Discovered in local man page".into(), values: vec![], takes_value: raw.contains('=') });
        }
    }
    flags
}
fn page_paths(root: &str) -> Vec<PathBuf> {
    let bases: Vec<PathBuf> = env::var_os("MANPATH").map(|v| env::split_paths(&v).collect())
        .unwrap_or_else(|| vec!["/usr/share/man".into(), "/usr/local/share/man".into(), "/opt/homebrew/share/man".into()]);
    let mut paths = Vec::new();
    for base in bases {
        for section in ["1", "8"] {
            for extension in ["", ".gz"] {
                paths.push(base.join(format!("man{section}/{root}.{section}{extension}")));
            }
        }
    }
    paths
}
fn fingerprint(path: &Path) -> Option<String> {
    let meta = fs::metadata(path).ok()?;
    let modified = meta.modified().ok()?.duration_since(UNIX_EPOCH).ok()?;
    Some(format!("{}:{}:{}", path.display(), meta.len(), modified.as_nanos()))
}
pub fn learn(root: &str) -> Vec<CommandSpec> {
    if root.is_empty() || !root.chars().all(|c| c.is_ascii_alphanumeric() || "-_+.".contains(c)) { return Vec::new(); }
    for path in page_paths(root) {
        let Some(fingerprint) = fingerprint(&path) else { continue };
        let cache = cache_dir().map(|dir| dir.join(format!("man-{root}.json")));
        if let Some(ref cache) = cache {
            if let Ok(data) = fs::read(cache) {
                if let Ok(saved) = serde_json::from_slice::<Cached>(&data) {
                    if saved.fingerprint == fingerprint {
                        return vec![CommandSpec { command: root.into(), flags: saved.flags, arguments: vec![] }];
                    }
                }
            }
        }
        let Some(source) = read_page(&path) else { continue };
        let flags = extract_flags(&source);
        if flags.is_empty() { continue; }
        if let Some(cache) = cache {
            if let Some(dir) = cache.parent() {
                if fs::create_dir_all(dir).is_ok() {
                    if let Ok(bytes) = serde_json::to_vec(&Cached { fingerprint, flags: flags.clone() }) {
                        let temp = cache.with_extension(format!("{}.tmp", std::process::id()));
                        if fs::write(&temp, bytes).is_ok() { let _ = fs::rename(&temp, &cache); }
                    }
                }
            }
        }
        return vec![CommandSpec { command: root.into(), flags, arguments: vec![] }];
    }
    Vec::new()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_roff_flags() {
        let flags = extract_flags(".B --amend\n.BI --cleanup=MODE\n.BR --signoff\n");
        assert!(flags.iter().any(|f| f.name == "--amend"));
        assert!(flags.iter().any(|f| f.name == "--cleanup" && f.takes_value));
    }
    #[test]
    fn rejects_invalid_root() {
        assert!(learn("../evil").is_empty());
    }
}
