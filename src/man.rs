//! Static, opt-in parsing of local manpage source files.
//! No man command, shell, pager, formatter or executable is launched.
use crate::catalog::Candidate;
use flate2::read::GzDecoder;
use std::fs;
use std::io::Read;
use std::path::Path;

const MAX_EXPANDED_BYTES: u64 = 512 * 1024;

pub fn command_name(filename: &str) -> Option<String> {
    let stem = filename.strip_suffix(".gz").unwrap_or(filename);
    let (name, section) = stem.rsplit_once('.')?;
    if !matches!(section, "1" | "1p" | "8") || !valid_name(name) {
        return None;
    }
    if let Some(subcommand) = name.strip_prefix("git-") {
        return Some(format!("git {subcommand}"));
    }
    Some(name.to_owned())
}

fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'))
        && !name.starts_with('-')
}

fn option(value: &str) -> bool {
    let value = value.trim_end_matches([',', ':']);
    let value = if let Some((flag, argument)) = value.split_once('=') {
        if argument.len() > 24
            || !argument
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-')
        {
            return false;
        }
        flag
    } else {
        value.trim_end_matches('=')
    };
    let short = value.starts_with('-') && value.len() == 2;
    let long = value.starts_with("--") && value.len() >= 4;
    (short || long)
        && value
            .trim_start_matches('-')
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '-')
}

fn token(raw: &str) -> Option<String> {
    let normalized = raw
        .replace("\\-", "-")
        .replace("\\fB", "")
        .replace("\\fR", "");
    let item = normalized
        .split_whitespace()
        .next()?
        .trim_matches(['"', '\'', ',', '.'])
        .trim_end_matches(':');
    if item.is_empty() || item.len() > 72 || item.contains(['\\', '{', '}', '$']) {
        return None;
    }
    Some(item.to_owned())
}

fn prose(line: &str) -> Option<String> {
    if line.starts_with('.') || line.trim().is_empty() {
        return None;
    }
    let text = line
        .replace("\\-", "-")
        .replace("\\fB", "")
        .replace("\\fR", "");
    let value = text.trim();
    if value.len() < 3 || value.len() > 160 || value.contains(['\t', '\n']) {
        return None;
    }
    Some(value.to_owned())
}

pub fn parse(filename: &str, source: &str) -> Vec<Candidate> {
    let Some(command) = command_name(filename) else {
        return Vec::new();
    };
    let mut result = Vec::new();
    let mut section = String::new();
    let mut pending = None::<String>;
    let mut heading = false;
    for line in source.lines().take(10_000) {
        let line = line.trim();
        if let Some(title) = line.strip_prefix(".SH ") {
            pending = None;
            heading = false;
            section = title.trim_matches('"').to_ascii_uppercase();
            continue;
        }
        if line == ".TP" || line.starts_with(".TP ") {
            heading = true;
            pending = None;
            continue;
        }
        if heading {
            heading = false;
            let candidate = line
                .strip_prefix(".B ")
                .or_else(|| line.strip_prefix(".BR "))
                .or_else(|| line.strip_prefix(".BI "))
                .and_then(token);
            let allowed = candidate.as_ref().is_some_and(|value| {
                if matches!(section.as_str(), "OPTIONS" | "OPTION" | "FLAGS") {
                    option(value)
                } else if matches!(section.as_str(), "COMMANDS" | "SUBCOMMANDS") {
                    valid_name(value)
                } else {
                    false
                }
            });
            if allowed {
                pending = candidate;
            }
            continue;
        }
        if let (Some(name), Some(description)) = (pending.take(), prose(line)) {
            let (name, description) = if let Some((flag, hint)) = name.split_once('=') {
                let safe =
                    hint.len() <= 24 && hint.chars().all(|c| c.is_ascii_alphanumeric() || c == '-');
                if safe && name.starts_with('-') {
                    (
                        format!("{flag}="),
                        format!(
                            "{description} · expects <{}>",
                            if hint.is_empty() { "value" } else { hint }
                        ),
                    )
                } else {
                    (name, description)
                }
            } else {
                (name, description)
            };
            result.push(Candidate {
                value: format!("{command} {name}"),
                description,
                source: "manpage".into(),
            });
            if result.len() >= 256 {
                break;
            }
        } else if line.starts_with('.') {
            pending = None;
        }
    }
    result
}

pub fn read(path: &Path) -> Option<Vec<Candidate>> {
    let filename = path.file_name()?.to_str()?;
    command_name(filename)?;
    let data = if filename.ends_with(".gz") {
        let file = fs::File::open(path).ok()?;
        let mut decoder = GzDecoder::new(file).take(MAX_EXPANDED_BYTES + 1);
        let mut output = Vec::new();
        decoder.read_to_end(&mut output).ok()?;
        if output.len() as u64 > MAX_EXPANDED_BYTES {
            return None;
        }
        String::from_utf8(output).ok()?
    } else {
        if fs::metadata(path).ok()?.len() > MAX_EXPANDED_BYTES {
            return None;
        }
        fs::read_to_string(path).ok()?
    };
    Some(parse(filename, &data))
}

#[cfg(test)]
mod tests {
    use super::*;
    const MAN: &str = include_str!("../tests/fixtures/orbit.1");

    #[test]
    fn parses_static_flags_and_commands_without_execution() {
        let items = parse("orbit.1", MAN);
        assert!(items.iter().any(|item| item.value == "orbit --verbose"));
        assert!(items.iter().any(|item| item.value == "orbit deploy"));
        assert!(!items.iter().any(|item| item.value.contains("--secret")));
        assert!(!items.iter().any(|item| item.value.contains("evil")));
    }

    #[test]
    fn recognizes_a_git_subcommand_flag_that_takes_a_value() {
        let text = ".SH OPTIONS\n.TP\n.B --message=MSG\nSet the commit message\n";
        let records = parse("git-commit.1", text);
        assert_eq!(records[0].value, "git commit --message=");
        assert!(records[0].description.contains("expects <MSG>"));
    }

    #[test]
    fn handles_git_subcommand_pages_and_rejects_unsafe_names() {
        assert_eq!(
            command_name("git-commit.1.gz").as_deref(),
            Some("git commit")
        );
        assert!(command_name("../../etc/passwd").is_none());
        assert!(command_name("orbit.5").is_none());
        assert!(parse("orbit.1", ".SH OPTIONS\n.TP\n.B --yes\nSafe\n")
            .iter()
            .any(|item| item.value == "orbit --yes"));
    }
}
