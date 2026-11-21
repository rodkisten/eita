# 🤝 Contributing to Eita

Thank you for helping a little crab build a better terminal. 🦀

Eita prioritizes **predictability, privacy, low latency, and interoperability**. A clever feature is only useful when it does not interfere with an existing Zsh configuration.

## 🌱 First steps

1. Read the [README](README.md), [architecture](docs/ARCHITECTURE.md), and [roadmap](docs/ROADMAP.md).
2. Look for an existing issue or discussion before implementing a major feature.
3. Use a branch based on the current `main`. Avoid work on merged or closed PR branches.
4. Keep one meaningful feature or fix in each PR. Group tightly related changes into complete, reviewable blocks.
5. Describe the motivation, constraints, trade-offs, risks, and tests in **English**.

### Prerequisites

- Rust toolchain **1.89+** with `rustfmt` and `clippy`
- Cargo
- Zsh for the integration and interactive tests
- Bash, tar, and a POSIX-like environment
- Optional: a Linux system for native inotify and Musl release checks

~~~sh
git clone https://github.com/rodkisten/eita.git
cd eita
cargo build
cargo test --all-targets
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
zsh -n integrations/zsh/eita.plugin.zsh
~~~

For live development:

~~~zsh
export PATH="$PWD/target/debug:$PATH"
source integrations/zsh/eita.plugin.zsh
eita suggest --line 'git com' --format json
eita popup --line 'git com'
~~~

The plugin is loaded only in an **interactive** Zsh shell and does not modify startup files on its own.

## 🧩 Repository layout

| Path | What to change here |
| --- | --- |
| `src/engine.rs` | Context-aware fuzzy ranking and hierarchical scopes |
| `src/context.rs` | Safe replacements across pipes, quotes and cursor positions |
| `src/completion.rs` | Static Zsh parsing and incremental file snapshots |
| `src/man.rs` | Conservative, read-only roff manpage parsing |
| `src/watch.rs` | Native filesystem event handling and debounce |
| `src/daemon.rs` | Unix socket, cache, lifecycle, per-shell synchronization |
| `src/usage.rs` | Privacy-preserving command template frequency |
| `src/ui.rs` | Ratatui interface and mobile terminal constraints |
| `src/config.rs` | Validated, typed TOML configuration |
| `integrations/zsh/` | ZLE widgets, hooks and IPC integration |
| `tests/fixtures/` | Small, self-contained parser fixtures |
| `docs/` | Architecture, specifications, roadmap, usage and releases |
| `.github/workflows/` | CI, portable builds, PR preview lifecycle |

## 🛡️ Non-negotiable safety rules

- **Never execute completion source files.** No shell `eval`, untrusted `--help`, external manpage formatters, or arbitrary executable invocation to discover flags.
- **Never persist raw command lines** from hooks, arbitrary arguments, credentials, hosts or variable values. The learning database stores approved templates only.
- **Never execute a selected suggestion.** Selection edits the next ZLE buffer; it must not submit commands automatically.
- **Do not seize native Tab, keymaps, or terminal output** without an explicit setting and a conflict-safe fallback.
- **Do not block the prompt on filesystem scans.** Source discovery belongs to an event-driven worker or an explicit index command.
- **Do not make the engine mandatory.** CLI fallback must function when the daemon is disabled, dead, slow, or the socket is unavailable.
- **Keep resources bounded.** Set limits for input bytes, number of files, recursion, cache entries, memory, timeouts, and thread/process lifetime.
- **Do not invent support or performance numbers.** A green Linux CI is not an iOS end-to-end benchmark.

See [SECURITY.md](SECURITY.md) for responsible reporting.

## ✍️ Coding style

Rust 2021 and straightforward, explicit data flow. Prefer the standard library and existing crates unless a new dependency offers clear value. Validate all user inputs at trust boundaries; use lossless types, deterministic ordering, and readable names.

- Run `cargo fmt` and Clippy with `-D warnings`.
- Use `Result` for failures users need to understand. Silent fallback is appropriate only for optional, best-effort features such as ghost text.
- Distinguish **raw user text**, **parsed candidate data**, **selected suggestions**, and **rendered output**.
- Keep static indexing separate from interactive ranking and UI.
- Make operations idempotent and atomic when they write to disk.
- Keep code, documentation, PR titles, release notes, and commit messages in **English**.
- Write test fixtures for every new parser grammar feature; do not assume a parser understands dynamic Zsh or full roff.

### Zsh conventions

Use named ZLE widgets and `add-zle-hook-widget` / `add-zsh-hook` for composition. Preserve existing handlers, use `emulate -L zsh` in functions that change options, and avoid global key interception. No `eval` of responses. Communicate local shell changes to Rust by a bounded, typed transport.

### Code review checklist

- [ ] Current PR and `main` state verified; conflicts resolved before updating the PR
- [ ] Feature is represented accurately in README and roadmap
- [ ] Unit tests added or updated for parser/engine behavior
- [ ] Static analysis and Zsh syntax pass
- [ ] Resource bounds and security implications reviewed
- [ ] No shell execution, raw history storage, or terminal corruption introduced
- [ ] Linux daemon gracefully handles shutdown and absent inotify support
- [ ] Benchmark methodology is reproducible and documented
- [ ] Preview installation command included, if release artifacts exist

## 🧪 Tests and benchmarks

~~~sh
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
cargo build --release
sh -n install.sh
zsh -n integrations/zsh/eita.plugin.zsh

# Algorithm-only comparison
./target/release/eita bench --iterations 500 --compare

# Actual local socket round trips
./target/release/eita daemon start
./target/release/eita daemon bench
./target/release/eita daemon stop
~~~

When changing the file watcher, manually test creating, renaming, removing, and replacing an executable in a dedicated temporary `PATH` directory and adding/removing `_completion` files in a dedicated temporary `fpath`. Confirm stale entries disappear and unchanged prompts do not spawn new processes.

For hierarchy work, cover both positive and negative cases: `git commit --a` should find commit options, but **not** checkout-only flags. Test required-argument hints and unsafe source literals.

For UI work, verify Zsh on a small terminal, 80-column desktop terminals, terminal resize, Esc, keyboard conflicts, and scrollback restoration.

## 🔀 Pull requests and releases

All PRs target `main`. CI validates format, lint, tests, installer syntax and Zsh syntax. The [release workflow](.github/workflows/release.yml) builds the enabled platform matrix, verifies checksum pairs, and creates a moving `pr-N` prerelease for repository-internal PRs. Main releases use commit-specific tags.

Preferred commit messages:

~~~text
feat: index fpath changes without rescanning on every prompt [deploy]
fix: preserve argument context in nested git completion [deploy]
docs: document architecture and contribution workflow [deploy]
~~~

If making multiple intermediate commits in a longer branch, use `[skip ci]` on intermediate snapshots and `[deploy]` for the final integrated change, where permitted by branch policy. Group complete work instead of fragmenting code into tiny commits. Never rewrite someone else's branch history without coordination.

PR descriptions should contain an overview, design decisions, test evidence, remaining limitations, a link to relevant issue(s), and a ChatGPT conversation URL **when AI-assisted development was used and sharing that URL is appropriate**. Do not add secrets to PR logs or screenshots.

## 🗣️ Reporting issues

Include the Eita version, OS/architecture, `zsh --version`, reproduction steps, expected/actual behavior, and whether the daemon and auto-index are enabled. Redact command history, passwords, private hosts, tokens, user paths, and environment values. Provide a minimal public fixture whenever possible.

Thanks for contributing! Keep the terminal fast and the claws friendly. 🦀
