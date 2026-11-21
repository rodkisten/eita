//! Passive man-page learning. Never executes the documented program.
use std::{env,fs,path::PathBuf,};
use crate::specs::{CommandSpec,FlagSpec};
pub fn learn(root: &str) -> Vec<CommandSpec> {
    if root.is_empty() || !root.chars().all(|c| c.is_ascii_alphanumeric() || "-_+.".contains(c)) { return Vec::new(); }
    // Read man-page files directly. No shell, man renderer or target executable.
    let roots = env::var_os("MANPATH").map(|v| env::split_paths(&v).collect::<Vec<_>>())
        .unwrap_or_else(|| vec![PathBuf::from("/usr/share/man"),PathBuf::from("/usr/local/share/man")]);
    let mut flags = Vec::new();
    for base in roots {
        for section in ["man1","man8"] {
            let path = base.join(section).join(format!("{root}.1"));
            let alternate = base.join(section).join(format!("{root}.8"));
            let source = fs::read_to_string(&path).or_else(|_| fs::read_to_string(&alternate));
            let Ok(source) = source else { continue };
            for line in source.lines() {
                if !line.starts_with(".B ") && !line.starts_with(".BI ") { continue; }
                let token = line.trim_start_matches(".BI ").trim_start_matches(".B ").trim().trim_matches('"');
                if !token.starts_with("--") { continue; }
                let flag = token.split(|c:char| c.is_whitespace() || c == '=' || c == '\\').next().unwrap_or("");
                if flag.len() < 3 || !flag[2..].chars().all(|c| c.is_ascii_alphanumeric() || c == '-') { continue; }
                if !flags.iter().any(|f: &FlagSpec| f.name == flag) {
                    flags.push(FlagSpec { name:flag.to_owned(), description:"Discovered in local man page".into(),values:vec![],takes_value:token.contains('=') });
                }
            }
            if !flags.is_empty() { return vec![CommandSpec {command:root.into(),flags,arguments:vec![]}]; }
        }
    }
    Vec::new()
}
