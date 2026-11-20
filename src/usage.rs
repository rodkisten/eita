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
        let path = usage_path();
        if fs::metadata(&path).is_ok_and(|meta| meta.len() > MAX_BYTES) {
            return Self::default();
        }
        let Ok(bytes) = fs::read(path) else {
            return Self::default();
        };
        serde_json::from_slice::<Self>(&bytes).unwrap_or_default()
    }

    pub fn bonus(&self, candidate: &str) -> i64 {
        // Bounded history cannot outrank a completely unrelated query.
        i64::from(self.counts.get(candidate).copied().unwrap_or(0).min(40)) * 12
    }

    pub fn record(&mut self, line: &str, catalog: &[Candidate]) -> Option<String> {
        let normalized = line.trim_start();
        let command = catalog
            .iter()
            .filter(|item| {
                normalized == item.value
                    || (normalized.starts_with(&item.value)
                        && normalized[item.value.len()..].starts_with(char::is_whitespace))
            })
            .max_by_key(|item| item.value.len())?
            .value
            .clone();

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

fn usage_path() -> PathBuf {
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
    fn counts_are_capped() {
        let mut usage = Usage::default();
        usage.counts.insert("git".into(), u32::MAX);
        usage.record("git", &sample());
        assert_eq!(usage.bonus("git"), 480);
    }
}
