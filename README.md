# 🦀 Eita

**A tiny shell assistant with a rather big vocabulary.**

Context-aware Zsh completion and typo recovery, powered by Rust, [Frizbee](https://github.com/saghen/frizbee), and [Ratatui](https://ratatui.rs). Eita learns *command shapes*, not your secrets, and keeps a warm local index ready for your next keystroke.

[![Rust CI](https://github.com/rodkisten/eita/actions/workflows/ci.yml/badge.svg)](https://github.com/rodkisten/eita/actions/workflows/ci.yml)
[![Build & Release](https://github.com/rodkisten/eita/actions/workflows/release.yml/badge.svg)](https://github.com/rodkisten/eita/actions/workflows/release.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust 1.89+](https://img.shields.io/badge/Rust-1.89%2B-orange.svg)](Cargo.toml)
[![Zsh](https://img.shields.io/badge/shell-Zsh-7257b5.svg)](integrations/zsh/eita.plugin.zsh)

> [!IMPORTANT]
> Eita **suggests and edits**, but never executes a correction for you. It is an additive Zsh plugin, not a replacement for `compinit`, native Tab completion, or your favorite shell setup.

## ✨ The quick tour

~~~console
$ eita suggest --line 'git com' --format lines
git commit
git commit --amend

$ eita suggest --line 'git commit --a' --format lines
git commit --all
git commit --amend

$ gti status
zsh: command not found: gti
🦀 Eita: Did you mean git?
# Choosing it queues "git status" for editing, not execution.
~~~

- 🎯 **Hierarchical completion:** `git commit --a` searches *commit flags*, not every option under `git`.
- 🧩 **Cursor-aware replacement:** keeps pipes, prior commands, and the text to the right of the cursor intact where the parser can safely do so.
- 👻 **Ghost text:** exact-prefix suggestions through ZLE `POSTDISPLAY`, with a custom accept key. Native Tab is untouched.
- 🦀 **Compact Ratatui picker:** customizable width, height, colors, alignment, and keyboard shortcuts.
- 🛟 **Typo rescue:** a real `command_not_found_handler` uses installed command/alias/function names and can restore an existing handler.
- 🧠 **Privacy-aware learning:** opt-in frequency ranking for approved command/subcommand templates, never raw command history or arguments.
- ⚡ **Warm Rust engine:** Unix socket, in-memory catalogue, bounded LRU cache (256 entries), and low-overhead Zsh socket integration.
- 👀 **Event-driven discovery (Linux):** native inotify notifications for `PATH`/`fpath`, debounced before incremental indexing, with no per-prompt filesystem walk.
- 📚 **Static knowledge:** opt-in Zsh completion and roff manpage parsing (including gzip). Reads files only, never invokes arbitrary programs.
- 📱 **SSH/mobile friendly:** works from iPhone SSH clients, with a fallback key binding for terminals that intercept Ctrl+Space.

### Table of contents

[Install](#-install) · [Get started](#-get-started) · [Configuration](#-configuration) · [Auto-indexing](#-auto-indexing) · [Hierarchical completion](#-hierarchical-completion) · [Architecture](#-how-it-works) · [Performance](#-performance--benchmarks) · [Security](#-security-and-privacy) · [Roadmap](#-product-roadmap) · [Contributing](#-contributing) · [Documentation](#-documentation)

## 📦 Install

### Latest release from `main`

~~~sh
curl -fsSL https://raw.githubusercontent.com/rodkisten/eita/main/install.sh | sh
~~~

The installer picks a matching **published** archive, verifies SHA-256, and installs `eita` to `~/.local/bin` and the optional plugin to `~/.local/share/eita/eita.plugin.zsh`. It does **not** modify your shell startup files.

### Current pull-request preview

Preview builds are published as moving prereleases, only after validation succeeds. To test PR #2:

~~~sh
curl -fsSL https://raw.githubusercontent.com/rodkisten/eita/main/install.sh | EITA_VERSION=pr-2 sh
~~~

The optional short route `https://shell.migos.club/install/eita/pr/2` depends on the separately deployed Shell Workbench router; the GitHub installer above works independently of that route.

**Current CI release matrix:** portable **Linux x86-64 Musl**. GNU, ARM64, and macOS targets may appear in previous assets or docs but are *not currently enabled* in the release matrix; see [release.yml](.github/workflows/release.yml). Never assume a platform is published until its archive exists.

### Build from source

Requires Rust **1.89+**, Cargo, and Zsh for the optional shell integration:

~~~sh
git clone https://github.com/rodkisten/eita.git
cd eita
cargo build --release
./target/release/eita suggest --line 'git com' --format lines
~~~

## 🚀 Get started

Add to `~/.zshrc`:

~~~zsh
export PATH="$HOME/.local/bin:$PATH"
source "$HOME/.local/share/eita/eita.plugin.zsh"
~~~

Then open a new interactive Zsh session and choose the features you want:

~~~zsh
eita config init       # creates ~/.config/eita/config.toml if absent
eita-index             # one-time local fpath index
eita-inline-on         # opt-in ghost text
eita-daemon-on         # optional persistent Rust engine
eita-learn-on          # optional private template-frequency learning
eita-keys              # inspect your active key bindings
~~~

**Try it:** type `git com` directly at the prompt. To accept ghost text, press `Ctrl+X Ctrl+E`. To open the Ratatui picker, use `Ctrl+Space` or `Ctrl+X Ctrl+P`. None of these keys execute the selected command.

~~~zsh
eita popup --line 'git com'          # direct interactive test, even on mobile
eita suggest --line 'git commit --' --format lines
eita-what 'EDTIOR' variable         # explicitly suggest EDITOR, without reading its value
eita-recover-on                     # enable command-not-found rescue
eita-recover-off                    # restore previous handler
~~~

> [!NOTE]
> The typo popup automatically handles **unknown commands** such as `gti`. Detecting an invalid subcommand *inside* a known tool, such as `git comit` after `git` has already launched, is still planned.

## 🎛️ Configuration

Configuration lives at `$XDG_CONFIG_HOME/eita/config.toml` (default `~/.config/eita/config.toml`). Override with `EITA_CONFIG`. All values are read and validated by Rust, never evaluated as shell source.

~~~toml
[popup]
width = 44
height = 8
max_results = 5
theme = "violet"           # violet | graphite | light | amber
align = "left"            # left | center | right
position = "auto"         # auto | above | below (within inline viewport)

[keys]
popup = "^@"
popup_fallback = "^X^P"
accept = "^X^E"
reindex = "^X^I"

[behavior]
trigger = "on-error"      # on-error | manual | off
inline = true
auto_index = true
~~~

~~~sh
eita config show
eita config zsh
~~~

**Important rendering limit:** Ratatui's current inline viewport reserves the terminal's full width. The *card* is narrow, but `above`/`below` align it within the reserved area. A true cursor-anchored overlay is on the roadmap.

See [configuration](docs/CONFIGURATION.md) and [ZLE integration](docs/INLINE.md).

## 👀 Auto-indexing

`eita-index` still supports a full explicit refresh. With the daemon enabled, the Linux engine can now keep common sources fresh **without checking directories at every prompt**.

~~~zsh
eita-daemon-on
eita-auto-index-on

# The Zsh precmd hook compares PATH and fpath strings in memory.
# Installing a command in a watched PATH directory triggers inotify.
# Adding a completion in a watched fpath directory triggers a
# debounced incremental cache refresh.

eita-auto-index-status
eita daemon sync       # manually publish this shell's PATH/FPATH
eita daemon reload     # force RAM catalogue reload
eita-auto-index-off    # stop this shell's PATH/fpath sync hook
~~~

Implementation notes:

1. **Zsh** compares source path strings on `precmd`. When unchanged, it spawns nothing. Changed paths are communicated asynchronously.
2. **Linux inotify** watches non-recursive source directories and debounces filesystem notifications for 350 ms. No recurring directory traversal is performed.
3. **Incremental index v2** reuses per-file snapshots using file size and modification time; additions/removals cause targeted reparsing.
4. **Daemon** invalidates its warm catalogue when the disk index changes or `PATH` events arrive; the regular ZLE fast path only queries memory.
5. **Fallbacks** remain available: manual refresh, CLI matching without a daemon, and explicit `eita daemon sync` when an event is missed.

Auto-indexing is **experimental**: event watching is native to Linux; on other platforms, shell path changes can still initiate explicit syncs, but there is no continuous native filesystem watch yet. Different simultaneous shells can have different `PATH` values; the current daemon uses one active environment at a time and falls back to local search when the client's `PATH` differs. Per-session catalogues are planned.

Explicitly add manpages to the persisted index when desired:

~~~zsh
eita-index --man-dir /usr/share/man
eita index --status
eita-index --man-dir /usr/share/man   # inspect reused_files / unchanged
~~~

Manpages remain an **opt-in** source. They are never launched or rendered with external programs.

## 🌳 Hierarchical completion

Eita recognizes the deepest known, *fully typed* command prefix:

~~~text
git
 ├─ status
 ├─ checkout
 │   └─ --ours
 └─ commit
     ├─ --amend
     ├─ --all
     └─ --message=    expects <MSG>
~~~

When you type `git com`, Eita searches the `git` namespace. After `git commit --a`, it narrows the pool to flags attached to `git commit` and won't show unrelated flags from `git checkout`. Prefix matches and historical usage influence ranking; fuzzy matching helps with small typos.

Where static Zsh or roff specs explicitly indicate a value, Eita retains the `=` suffix and shows a description such as *expects <path>* or *expects <MSG>*. It **does not** invent argument values, read secrets, or execute completion functions. True dynamic value providers, enum-aware cycling, and full shell AST parsing remain planned.

The parser only accepts a conservative, literal subset of `_arguments`, simple command arrays, and manpage `OPTIONS` / `COMMANDS` sections. Dynamic completions remain the job of native Zsh. See [indexer details](docs/INDEXING.md).

## 🧱 How it works

~~~text
Zsh / ZLE (LBUFFER + RBUFFER)
     │
     ├── Manual picker ──────────────> Ratatui inline card
     │
     ├── Command-not-found ──────────> Safe typo suggestions
     │
     ├── Ghost text ── zsocket ──────> Unix socket ─┐
     │                                              │
     └── PATH/fpath changes ── precmd ──────────────┤
                                                    ▼
                                        Persistent Rust engine
                                          ├─ Frizbee ranking
                                          ├─ hierarchical scope
                                          ├─ 256-entry LRU
                                          ├─ usage templates
                                          └─ cached candidates
                                                    ▲
                                 Linux inotify ─────┤
                                 incremental v2 ────┘
                                   ├─ PATH executables
                                   ├─ Zsh _arguments
                                   ├─ opt-in manpages
                                   └─ user specs.tsv
~~~

**Source map**

| Source | Responsibility |
| --- | --- |
| `src/context.rs` | Quoting-aware, cursor-aware command segment replacement |
| `src/engine.rs` | Frizbee matching, hierarchical scopes, ranking |
| `src/catalog.rs` | Built-ins, executables, user specs, cached suggestions |
| `src/completion.rs`, `src/man.rs` | Read-only static parsers and incremental JSON index |
| `src/watch.rs` | Linux native directory notifications and debounce |
| `src/daemon.rs` | Unix socket transport, idle shutdown, warm LRU and source syncing |
| `src/usage.rs` | Explicit opt-in learning of safe command templates |
| `src/ui.rs`, `src/config.rs` | Ratatui rendering and TOML configuration |
| `integrations/zsh/eita.plugin.zsh` | ZLE widgets, hooks, ghost text, fallback controls |

For protocol, data flow, and limitations, see [architecture](docs/ARCHITECTURE.md).

## ⚡ Performance & benchmarks

Two layers are measured separately:

~~~zsh
eita daemon start
eita daemon status       # PID, catalogue, hits, misses, watcher state
eita daemon bench        # p50/p95/p99 IPC + ranking (warm engine)
eita bench --iterations 500 --compare  # Frizbee vs Nucleo matcher
eita daemon stop
~~~

An earlier GitHub runner measured **27 µs p50 / 33 µs p95 / 54 µs p99** for 200 warm socket queries. These are *historical runner observations*, not a speed guarantee. They exclude binary startup, shell redraw, network latency, Ratatui rendering, and iPhone input events. The next performance milestone is an end-to-end ZLE measurement including mobile SSH.

The engine stays optional, starts on demand, and exits after approximately **15 minutes idle**. The Zsh socket path avoids a Rust process startup per keystroke; normal CLI fallback remains available.

## 🔐 Security and privacy

**Local first means local only.** No network API, cloud learning, telemetry, remote execution, or shell-history import.

- Eita never executes a suggested correction.
- Learning is **opt-in** (`eita-learn-on`) and stores a small allowlist of command/subcommand templates only. Raw arguments, hosts, paths, variables, and messages are not stored.
- Zsh symbols are passed by *name*, not by alias/function bodies or variable values.
- Static completion and manpage sources are read as bounded data, never evaluated or invoked.
- The daemon binds a user-owned Unix socket in a restricted directory (0700); socket permissions are 0600.
- Request sizes, read/write timeouts, source file sizes, cache sizes, and candidate counts have limits.
- Check the selected text before pressing Enter. Fuzzy suggestions are not security decisions.
- Install scripts verify archive checksums. For high-assurance installation, inspect the shell script and pin a release tag and checksum.

Read [SECURITY.md](SECURITY.md) before reporting a vulnerability.

## 🗺️ Product roadmap

We separate **shipped**, **experimental**, and **planned** work explicitly, so this document never presents a mockup as a feature.

| Area | Status | Remaining work |
| --- | --- | --- |
| Cursor-aware command suggestions and typo recovery | ✅ Shipped | Better nested shell syntax and typo recovery for tool-specific subcommands |
| Compact Ratatui picker, themes, keybindings | ✅ Shipped | Real cursor-anchored overlay above/below the prompt |
| Opt-in ZLE ghost text and safe usage templates | ✅ Shipped | Asynchronous redraw and richer opt-in ranking controls |
| Warm Rust daemon and 256-entry LRU | ✅ Shipped | Multi-session environment isolation, persistent socket connections |
| Incremental Zsh + manpage index | ✅ Shipped | Content hashes, richer grammar, coverage reporting |
| Linux inotify + PATH/fpath auto-sync | 🧪 Experimental | Watch recreation, resync after missed events, macOS native watcher |
| Hierarchical subcommand/flag ranking | 🧪 Experimental | Typed argument providers, option groups, mutually exclusive flags |
| Manpage argument hints | 🧪 Experimental | Extract structured enums, files, and value constraints |
| Full ZLE-to-screen benchmark | 🗓️ Planned | p50/p95/p99 on iPhone SSH terminals, CPU and memory budgets |
| Advanced shell adapters | 🗓️ Planned | Other shells, optional Fish/Bash integrations, accessibility audit |

See the [detailed engineering roadmap](docs/ROADMAP.md) for acceptance criteria, milestones, dependencies, and design constraints.

## 🤝 Contributing

Contributions are welcome, especially parser fixtures, performance measurements, Zsh compatibility fixes, and careful UX improvements for small terminals. Please read [CONTRIBUTING.md](CONTRIBUTING.md) for setup, workflow, code style, tests, security, and PR expectations.

~~~sh
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
cargo build --release
zsh -n integrations/zsh/eita.plugin.zsh
~~~

**Please do not** add code that evaluates third-party completion files, logs raw commands, intercepts native Tab by default, or spawns long-running background processes without bounded lifecycle controls.

## 📚 Documentation

- [Architecture](docs/ARCHITECTURE.md): data flow, native watchers, socket protocol, and trust boundaries.
- [Configuration](docs/CONFIGURATION.md): TOML options, themes, shortcuts, and inline viewport limitations.
- [Incremental indexing](docs/INDEXING.md): cache v2, source discovery, static parsers, and refresh behavior.
- [Daemon and learning](docs/DAEMON.md): IPC, privacy model, cache behavior, benchmarks.
- [ZLE interaction](docs/INLINE.md): ghost text, hooks, cursor safety.
- [Builds and releases](docs/RELEASES.md): artifacts, checksums, and PR previews.
- [Engineering roadmap](docs/ROADMAP.md): what comes next.
- [Security policy](SECURITY.md): responsible reporting and security guarantees.
- [Contributing](CONTRIBUTING.md): development and PR process.

## ❤️ Credits & license

Built with [Frizbee](https://github.com/saghen/frizbee), [Nucleo Matcher](https://github.com/helix-editor/nucleo), [Ratatui](https://ratatui.rs/), and the Zsh Line Editor. Eita is available under the [MIT License](LICENSE).

*One less typo. One more happy terminal. 🦀*
