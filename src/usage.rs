//! Local, explicit opt-in ranking by previously selected command *templates*.
//! Never persist raw shell lines, arguments, credentials, or shell history.
use crate::catalog::Candidate;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::fs;
use std::path::PathBuf;

const MAX_ENTRIES: usize = 4096;
const MAX_BYTES: u64 = 512 * 1024;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Usage {
    counts: HashMap<String, u32>,
}

impl Usage {
    pub fn enabled() -> bool {
        env::var("EITA_USAGE_ENABLED").is_ok_and(|value| value == "1")
    }

    pub fn load() -> Self {
        if !Self::enabled() {
            return Self::default();
        }
        Self::load_saved()
    }

    /// Existing opt-in data can be reloaded by a daemon that was started
    /// before this particular shell enabled tracking.
    pub fn load_saved() -> Self {
        let path = usage_path();
        if fs::metadata(&path).is_ok_and(|meta| meta.len() > MAX_BYTES) {
            return Self::default();
        }
        let Ok(bytes) = fs::read(path) else {
            return Self::default();
        };
        serde_json::from_slice::<Self>(&bytes).unwrap_or_default()
    }

    /// Learned candidates are strictly whitelisted command/subcommand templates.
    /// Never import history, paths, arbitrary shell arguments, or opaque flags.
    pub fn learned(&self) -> Vec<Candidate> {
        let mut output: Vec<_> = self
            .counts
            .iter()
            .filter_map(|(value, count)| {
                if *count < 2 || Self::safe_template(value).as_deref() != Some(value.as_str()) {
                    return None;
                }
                Some(Candidate {
                    value: value.clone(),
                    description: "Aprendido localmente (apenas template)".into(),
                    source: "learned".into(),
                })
            })
            .collect();
        output.sort_by(|a, b| a.value.cmp(&b.value));
        output
    }

    fn safe_template(line: &str) -> Option<String> {
        let mut parts = line.split_whitespace();
        let root = parts.next()?;
        let verb = parts.next()?;
        let verbs: &[&str] = match root {
            "git" => &[
                "add",
                "bisect",
                "branch",
                "checkout",
                "cherry-pick",
                "clean",
                "clone",
                "commit",
                "diff",
                "fetch",
                "grep",
                "init",
                "log",
                "merge",
                "pull",
                "push",
                "rebase",
                "reflog",
                "remote",
                "reset",
                "restore",
                "revert",
                "show",
                "stash",
                "status",
                "submodule",
                "switch",
                "tag",
                "worktree",
            ],
            "gh" => &[
                "api", "auth", "gist", "issue", "pr", "release", "repo", "run", "workflow",
            ],
            "cargo" => &[
                "add", "bench", "build", "check", "clean", "clippy", "doc", "fmt", "install",
                "new", "publish", "run", "search", "test", "tree", "update",
            ],
            "docker" => &[
                "build",
                "compose",
                "container",
                "exec",
                "images",
                "inspect",
                "logs",
                "network",
                "ps",
                "pull",
                "push",
                "run",
                "stop",
                "volume",
            ],
            "kubectl" => &[
                "apply",
                "config",
                "create",
                "delete",
                "describe",
                "edit",
                "exec",
                "get",
                "logs",
                "port-forward",
                "rollout",
                "scale",
                "top",
            ],
            "npm" => &[
                "audit",
                "ci",
                "exec",
                "install",
                "outdated",
                "publish",
                "run",
                "test",
                "uninstall",
                "update",
                "view",
            ],
            "pnpm" => &[
                "add", "build", "dev", "dlx", "exec", "install", "lint", "outdated", "remove",
                "run", "test", "update",
            ],
            "yarn" => &["add", "build", "install", "remove", "run", "test", "up"],
            "bun" => &["add", "build", "install", "run", "test", "update", "x"],
            "go" => &[
                "build", "clean", "doc", "env", "fmt", "get", "install", "mod", "run", "test",
                "vet",
            ],
            "aws" => &[
                "cloudformation",
                "cloudfront",
                "ec2",
                "ecs",
                "ecr",
                "iam",
                "lambda",
                "s3",
                "sts",
            ],
            "gcloud" => &[
                "auth",
                "builds",
                "compute",
                "config",
                "functions",
                "iam",
                "projects",
                "run",
                "storage",
            ],
            "wrangler" => &[
                "deploy", "dev", "d1", "pages", "r2", "secret", "tail", "types",
            ],
            _ => return None,
        };
        if !verbs.contains(&verb) {
            return None;
        }
        // Optional third token only from a small universal flag vocabulary;
        // everything following is dropped, never stored.
        let flag = parts.next();
        let flag = match flag {
            Some(
                "-h" | "--help" | "-v" | "--verbose" | "-q" | "--quiet" | "--all" | "--short"
                | "--oneline" | "--dry-run" | "--force" | "--json",
            ) => flag,
            _ => None,
        };
        Some(match flag {
            Some(flag) => format!("{root} {verb} {flag}"),
            None => format!("{root} {verb}"),
        })
    }

    pub fn bonus(&self, candidate: &str) -> i64 {
        // Bounded history cannot outrank a completely unrelated query.
        i64::from(self.counts.get(candidate).copied().unwrap_or(0).min(40)) * 12
    }

    pub fn record(&mut self, line: &str, catalog: &[Candidate]) -> Option<String> {
        let normalized = line.trim_start();
        let from_catalog = catalog
            .iter()
            .filter(|item| {
                normalized == item.value
                    || (normalized.starts_with(&item.value)
                        && normalized[item.value.len()..].starts_with(char::is_whitespace))
            })
            .max_by_key(|item| item.value.len())
            .map(|item| item.value.clone());
        let safe = Self::safe_template(normalized);
        let command = match (from_catalog, safe) {
            (Some(existing), Some(new)) if new.len() > existing.len() => new,
            (Some(existing), _) => existing,
            (None, Some(new)) => new,
            (None, None) => return None,
        };

        if !self.counts.contains_key(&command) && self.counts.len() >= MAX_ENTRIES {
            return None;
        }
        let entry = self.counts.entry(command.clone()).or_insert(0);
        *entry = entry.saturating_add(1);
        Some(command)
    }

    pub fn save(&self) -> Result<(), Box<dyn Error>> {
        let path = usage_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let temporary = path.with_extension(format!("{}.tmp", std::process::id()));
        let encoded = serde_json::to_vec(self)?;
        if encoded.len() as u64 > MAX_BYTES {
            return Err("Usage storage limit exceeded".into());
        }
        fs::write(&temporary, encoded)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&temporary, fs::Permissions::from_mode(0o600))?;
        }
        fs::rename(temporary, path)?;
        Ok(())
    }

    pub fn clear() -> Result<(), Box<dyn Error>> {
        match fs::remove_file(usage_path()) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error.into()),
        }
    }

    pub fn summary(&self) -> Vec<(String, u32)> {
        let mut result: Vec<_> = self
            .counts
            .iter()
            .map(|(value, count)| (value.clone(), *count))
            .collect();
        result.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        result.truncate(50);
        result
    }
}

pub fn usage_path() -> PathBuf {
    if let Some(path) = env::var_os("EITA_USAGE_FILE") {
        return path.into();
    }
    let base = env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share")))
        .unwrap_or_else(env::temp_dir);
    base.join("eita/usage.json")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Vec<Candidate> {
        ["git", "git commit", "git commit --amend"]
            .iter()
            .map(|value| Candidate {
                value: (*value).into(),
                description: "test".into(),
                source: "builtin".into(),
            })
            .collect()
    }

    #[test]
    fn remembers_only_public_templates_not_raw_args() {
        let mut usage = Usage::default();
        assert_eq!(
            usage.record("git commit -m very-private-message", &sample()),
            Some("git commit".into())
        );
        assert_eq!(usage.bonus("git commit"), 12);
        assert!(!serde_json::to_string(&usage)
            .unwrap()
            .contains("very-private-message"));
    }

    #[test]
    fn matching_more_specific_templates_wins() {
        let mut usage = Usage::default();
        assert_eq!(
            usage.record("git commit --amend", &sample()),
            Some("git commit --amend".into())
        );
        assert_eq!(usage.bonus("git commit --amend"), 12);
    }

    #[test]
    fn learns_only_whitelisted_templates_and_never_arguments() {
        let mut usage = Usage::default();
        assert_eq!(
            usage.record("git worktree prune --expire=private", &sample()),
            Some("git worktree".into())
        );
        assert!(usage.learned().is_empty());
        usage.record("git worktree add /sensitive/location", &sample());
        let learned = usage.learned();
        assert_eq!(learned[0].value, "git worktree");
        assert!(!serde_json::to_string(&usage).unwrap().contains("sensitive"));
        assert_eq!(usage.record("ssh example.secret.host", &sample()), None);
        assert_eq!(usage.record("echo API_KEY=password", &sample()), None);
        assert_eq!(
            usage.record("git commit -m super-secret", &sample()),
            Some("git commit".into())
        );
    }

    #[test]
    fn counts_are_capped() {
        let mut usage = Usage::default();
        usage.counts.insert("git".into(), u32::MAX);
        usage.record("git", &sample());
        assert_eq!(usage.bonus("git"), 480);
    }
}
