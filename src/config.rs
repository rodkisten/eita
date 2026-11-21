//! Local user configuration. Reading this file does not execute shell code.
use serde::{Deserialize, Serialize};
use std::{env, error::Error, fs, path::PathBuf};

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Theme {
    Violet,
    Graphite,
    Light,
    Amber,
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Align {
    Left,
    Center,
    Right,
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Position {
    Auto,
    Above,
    Below,
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Trigger {
    Manual,
    OnError,
    Off,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct Popup {
    /// Visible card width, not the actual terminal viewport width.
    pub width: u16,
    pub height: u16,
    pub max_results: usize,
    pub theme: Theme,
    pub align: Align,
    pub position: Position,
}
impl Default for Popup {
    fn default() -> Self {
        Self {
            width: 48,
            height: 8,
            max_results: 5,
            theme: Theme::Violet,
            align: Align::Left,
            position: Position::Auto,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct Keys {
    /// Zsh bindkey notation; never shell commands.
    pub popup: String,
    pub popup_fallback: String,
    pub accept: String,
    pub reindex: String,
}
impl Default for Keys {
    fn default() -> Self {
        Self {
            popup: "^@".into(),
            popup_fallback: "^X^P".into(),
            accept: "^X^E".into(),
            reindex: "^X^I".into(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct Behavior {
    /// "on-error" = popup for missing commands; "manual" = only on shortcut.
    pub trigger: Trigger,
    /// Visual ghost text, never executes or opens a Ratatui window.
    pub inline: bool,
    /// Enable native filesystem event monitoring while the daemon runs.
    pub auto_index: bool,
}
impl Default for Behavior {
    fn default() -> Self {
        Self {
            trigger: Trigger::OnError,
            inline: false,
            auto_index: true,
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct Config {
    pub popup: Popup,
    pub keys: Keys,
    pub behavior: Behavior,
}

pub fn path() -> PathBuf {
    if let Some(value) = env::var_os("EITA_CONFIG") {
        return value.into();
    }
    let base = env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
        .unwrap_or_else(env::temp_dir);
    base.join("eita/config.toml")
}

impl Config {
    pub fn load() -> Result<Self, Box<dyn Error>> {
        let path = path();
        let content = match fs::read_to_string(&path) {
            Ok(content) => content,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(err) => return Err(err.into()),
        };
        if content.len() > 64 * 1024 {
            return Err("eita config exceeds 64 KiB".into());
        }
        let value: Self = toml::from_str(&content)?;
        value.validate()?;
        Ok(value)
    }
    pub fn validate(&self) -> Result<(), Box<dyn Error>> {
        if !(24..=160).contains(&self.popup.width) {
            return Err("popup.width must be 24..160".into());
        }
        if !(6..=14).contains(&self.popup.height) {
            return Err("popup.height must be 6..14".into());
        }
        if !(1..=10).contains(&self.popup.max_results) {
            return Err("popup.max_results must be 1..10".into());
        }
        for key in [
            &self.keys.popup,
            &self.keys.popup_fallback,
            &self.keys.accept,
            &self.keys.reindex,
        ] {
            if key.is_empty()
                || key.len() > 20
                || !key.is_ascii()
                || key.chars().any(|ch| ch.is_ascii_control() || ch == '\t')
            {
                return Err("invalid Zsh bindkey sequence".into());
            }
        }
        Ok(())
    }
    pub fn init(force: bool) -> Result<PathBuf, Box<dyn Error>> {
        let destination = path();
        if destination.exists() && !force {
            return Err("config exists; use --force to overwrite".into());
        }
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }
        let file = toml::to_string_pretty(&Self::default())?;
        let temporary = destination.with_extension("toml.tmp");
        fs::write(&temporary, file)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&temporary, fs::Permissions::from_mode(0o600))?;
        }
        fs::rename(temporary, &destination)?;
        Ok(destination)
    }
    pub fn zsh_pairs(&self) -> String {
        let trigger = match self.behavior.trigger {
            Trigger::OnError => "1",
            Trigger::Manual | Trigger::Off => "0",
        };
        format!(
            "EITA_POPUP_WIDTH\t{}\nEITA_POPUP_HEIGHT\t{}\nEITA_POPUP_THEME\t{}\nEITA_POPUP_ALIGN\t{}\nEITA_POPUP_POSITION\t{}\nEITA_RECOVER_ENABLED\t{}\nEITA_INLINE_ENABLED\t{}\nEITA_AUTO_INDEX_ENABLED\t{}\nEITA_KEY_POPUP\t{}\nEITA_KEY_POPUP_FALLBACK\t{}\nEITA_KEY_ACCEPT\t{}\nEITA_KEY_REINDEX\t{}\n",
            self.popup.width, self.popup.height,
            toml_key(self.popup.theme), toml_key(self.popup.align),
            toml_key(self.popup.position), trigger,
            if self.behavior.inline {"1"} else {"0"},
            if self.behavior.auto_index {"1"} else {"0"},
            self.keys.popup, self.keys.popup_fallback,
            self.keys.accept, self.keys.reindex
        )
    }
}
fn toml_key<T: Serialize>(value: T) -> String {
    toml::Value::try_from(value)
        .map(|x| x.to_string().trim_matches('"').to_owned())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn default_is_compact() {
        let config = Config::default();
        assert_eq!(config.popup.width, 48);
        assert_eq!(config.popup.theme, Theme::Violet);
        config.validate().unwrap();
        assert!(config.zsh_pairs().contains("EITA_KEY_POPUP_FALLBACK\t^X^P"));
    }
    #[test]
    fn partial_toml_keeps_defaults() {
        let config: Config =
            toml::from_str("[popup]\nwidth = 36\ntheme = 'light'\n[behavior]\ntrigger = 'manual'")
                .unwrap();
        assert_eq!(config.popup.height, 8);
        assert_eq!(config.popup.width, 36);
        assert_eq!(config.behavior.trigger, Trigger::Manual);
        assert_eq!(config.popup.theme, Theme::Light);
    }
    #[test]
    fn invalid_keys_are_rejected() {
        let mut config = Config::default();
        config.keys.popup = "hello\nworld".into();
        assert!(config.validate().is_err());
    }
}
