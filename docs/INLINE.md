# 👻 ZLE Ghost Text, Ratatui & Typo Recovery

The Zsh adapter is intentionally **additive**: native Tab, syntax highlighting, and existing ZLE widgets keep working.

## Try ghost text

~~~zsh
export PATH="$HOME/.local/bin:$PATH"
source "$HOME/.local/share/eita/eita.plugin.zsh"

eita-inline-on
# Type: git com
# Eita can show "mit" as faded POSTDISPLAY text.
# Accept: Ctrl+X, Ctrl+E
eita-inline-off
~~~

`POSTDISPLAY` contains a *visual suffix only*. The original `BUFFER` is not changed until an explicit accept. Eita does not show inline typos that would require rewriting already typed characters; use the picker for that.

The widget uses named `line-pre-redraw` and `line-finish` hooks via `add-zle-hook-widget`. It does not replace Tab, Right Arrow or the default completion system. When another plugin owns `POSTDISPLAY`, Eita stays out of its way. Ghost text is disabled when the cursor is in the middle of the buffer or the context is not safely editable.

### Optional Rust engine

~~~zsh
eita-daemon-on
eita-inline-on
eita-auto-index-on
eita daemon status
eita daemon bench
~~~

With the daemon active, the Zsh adapter queries the Unix socket through built-in `zsocket` and `zselect`, **without starting Rust on every changed buffer**. The socket wait is bounded to 40 ms; an unavailable service falls back to the ordinary CLI. Identical buffer redraws reuse the most recent suffix. Standard limits are a minimum of three characters, 400 typed characters, and 160 suffix characters.

Relevant environment controls:

| Variable | Default | Purpose |
| --- | --- | --- |
| `EITA_INLINE_ENABLED` | 0 | Ghost text on/off |
| `EITA_INLINE_MIN_CHARS` | 3 | Minimum input length |
| `EITA_INLINE_COLOR` | 8 | Terminal highlight color |
| `EITA_DAEMON_ENABLED` | 0 | Use warm socket engine |
| `EITA_AUTO_INDEX_ENABLED` | 1 | Enable native watches + PATH/fpath synchronization |
| `EITA_USAGE_ENABLED` | 0 | Private template ranking |
| `EITA_COMPLETION_CACHE` | XDG cache | Static candidate snapshots |

## 🦀 Open the picker

~~~zsh
eita popup --line 'git com'
# Or press Ctrl+Space / Ctrl+X Ctrl+P inside a live ZLE prompt.
~~~

Ratatui displays a narrow configurable card. This is **not** a true GUI popup tied to cursor coordinates; its inline viewport reserves terminal rows and width. For small phone terminals, edit the TOML popup height/width and use the fallback shortcut.

## 🛟 Unknown-command recovery

~~~zsh
eita-recover-on
gti status
# The correction UI can offer git.
# Confirming stages "git status" in the next prompt; nothing runs.
eita-recover-off
eita-what 'EDTIOR' variable
~~~

The handler collects only **names** of available commands, aliases, functions, built-ins, reserved words and variables. It never reads alias bodies, function bodies or variable values for recovery. Previous command-not-found handlers are retained/restored when possible. Unknown commands in noninteractive shells and sessions without a TTY do not open a TUI.

A typo such as `git comit` executes an existing program (`git`) and does not trigger the unknown-command hook. Automated subcommand-failure recovery is planned, while manual fuzzy suggestions already work.

## 🧠 Optional local learning

~~~zsh
eita-learn-on
eita usage status
eita-learn-off
eita usage clear
~~~

Approved templates are saved to a private JSON file. Full commands, arbitrary arguments, passwords and history are not stored. See [DAEMON.md](DAEMON.md) for the precise privacy boundary.

## Matcher and transport benchmarks

~~~zsh
eita inline --left 'git com'       # prints "mit" when available
eita suggest --line 'git com' --format json
eita bench --iterations 500 --compare
eita daemon bench
~~~

The algorithm benchmark compares Frizbee and Nucleo on the same in-memory inputs. The daemon benchmark measures warm IPC and matching. Neither measures SSH network lag or the time between an iPhone keypress and the rendered suggestion.
