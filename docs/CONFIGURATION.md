# 🎛️ Configuration & Terminal UX

Eita uses a typed, validated TOML file at `$XDG_CONFIG_HOME/eita/config.toml` (default: `~/.config/eita/config.toml`). Set `EITA_CONFIG` to override its path.

~~~sh
eita config init                 # create; never overwrite without --force
eita config show                 # effective settings
eita config zsh                  # trusted key/value TSV for the Zsh plugin
~~~

## Reference configuration

~~~toml
[popup]
width = 48
height = 8
max_results = 5
theme = "violet"             # violet | graphite | light | amber
align = "left"              # left | center | right
position = "auto"           # auto | above | below (within inline viewport)

[keys]
popup = "^@"
popup_fallback = "^X^P"
accept = "^X^E"
reindex = "^X^I"

[behavior]
trigger = "on-error"        # on-error | manual | off
inline = false
auto_index = true
~~~

| Setting | Valid values | Meaning |
| --- | --- | --- |
| `popup.width` | 24–160 | Visual card width, clipped to terminal width |
| `popup.height` | 6–14 | Reserved card height |
| `popup.max_results` | 1–10 | Maximum displayed candidates |
| `popup.theme` | violet, graphite, light, amber | Ratatui color palette |
| `popup.align` | left, center, right | Placement across the reserved viewport |
| `popup.position` | auto, above, below | Alignment **inside the inline viewport** |
| `behavior.trigger` | on-error, manual, off | Automatic *unknown-command* recovery behavior |
| `behavior.inline` | boolean | Enable ghost-text rendering on startup |
| `behavior.auto_index` | boolean | Allow native daemon watches and shell source sync |
| `keys.*` | Zsh `bindkey` sequences | Configurable shortcuts |

### Activation details

- `on-error` installs the command-not-found recovery when the plugin loads, preserving status 127. It does not detect invalid subcommands inside existing programs.
- `manual` preserves the explicit Ratatui picker but disables automatic command-not-found handling.
- `off` disables recovery; set `inline = false` too if you want no ghost text.
- `auto_index = true` enables source synchronization in a Zsh session when the daemon is enabled. It does not start the daemon by itself.
- Existing explicitly set `EITA_*` environment overrides take priority for the shell adapter. Restart Zsh or source the plugin again after editing keys.

## 🎨 The viewport is compact, not cursor-anchored

Ratatui `Viewport::Inline` still reserves the terminal's full width. Eita draws a narrow, configurable card **inside** that space. `position = "above"` / `"below"` currently align within the reserved rows and cannot float relative to the text cursor. A true cursor-anchored overlay is a planned ZLE rendering project; see [ROADMAP.md](ROADMAP.md).

## 📱 SSH / iPhone keyboard behavior

Some SSH clients intercept Ctrl+Space. Use **Ctrl+X, then Ctrl+P** to open the picker, or call:

~~~zsh
eita popup --line 'git com'
eita-keys
~~~

The picker never executes text. Arrow keys move selection, Enter applies to an editable buffer, and Esc cancels. The option to accept ghost text is separate: **Ctrl+X, Ctrl+E**.

## Live diagnostics

~~~zsh
eita config show
eita daemon status
eita-auto-index-status
eita-inline-on
eita-inline-off
eita-recover-on
eita-recover-off
~~~

The explicit `eita auto-index` subcommand is *not* currently implemented; source-watch operations belong to `eita daemon` and Zsh helper functions. Full configuration editing UI remains planned.
