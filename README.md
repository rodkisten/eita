# 🦀 Eita

**Your terminal knows the commands. Eita helps you find the right ones.** ✨

[![Rust](https://img.shields.io/badge/Rust-1.89%2B-orange?logo=rust)](https://www.rust-lang.org/) [![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE) [![Zsh](https://img.shields.io/badge/Shell-Zsh-89e051)](https://www.zsh.org/)

Eita is a Rust-powered, typo-tolerant command suggestion engine for Zsh, combining **Frizbee** fuzzy matching, **Ratatui** terminal UI, hierarchical command awareness and a persistent executable index. It complements native Zsh completion rather than replacing it.

> 🚧 **Early development.** The checklist below distinguishes shipped code from future plans. No suggestion is executed automatically.

## ⚡ Quick start

Install the latest published release (Linux x86_64/aarch64, macOS Intel/Apple Silicon):

```sh
curl -fsSL https://raw.githubusercontent.com/rodkisten/eita/main/install.sh | sh
```

The installer detects your platform, downloads the release archive, verifies SHA-256 and installs the binary and optional Zsh plugin. It does **not** silently modify your shell configuration. Review remote scripts before piping them to a shell.

Add to `~/.zshrc`:

```zsh
export PATH="$HOME/.local/bin:$PATH"
source "$HOME/.local/share/eita/eita.plugin.zsh"
```

Restart Zsh and press **Ctrl+Space**. If your terminal does not transmit that key combination, bind another:

```zsh
bindkey '^X^P' eita-pick
```

**Try without installing the plugin:**

```sh
eita suggest --line 'git comit --am'
eita suggest --line 'git commit --am' --format lines
eita pick --line 'git com'
eita bench --iterations 1000
```

`pick` inserts text; Enter/Tab select, Esc cancels. Selection never runs the command.

## 🧠 How it works

```text
Zsh / CLI input
    │
    ▼
Contextual command path ─── git → commit → --amend
    │
    ▼
Catalog ──────────────── built-ins + PATH index + custom TSV
    │
    ▼
Frizbee fuzzy ranking ─── typo tolerance + prefix/exact boosts
    │
    ▼
JSON / lines / Ratatui selector ─── insertion, never execution
```

### 🪄 Hierarchical matching

Eita scopes candidate matching to the active command path instead of matching the entire line as one flat string.

| Input | Intended completion | Behavior |
| --- | --- | --- |
| `got` | `git` | Fuzzy root correction |
| `git com` | `git commit` | Subcommand context |
| `git commit --am` | `git commit --amend` | Flag within a subcommand |
| `git checkout ` | Child candidates only | Preserve parent context |

The current implementation matches **known catalog entries**, not arbitrary dynamically discovered flag schemas. Typed argument values, `--flag=value`, quote-aware cursor replacement and live Zsh completion function introspection remain planned.

### ⚡ Smart PATH indexing

Executable discovery now uses a persistent JSON cache instead of scanning every directory on each invocation.

1. Collect ordered `PATH` directories and the `FPATH` environment value.
2. Build a fingerprint from these values and directory modification timestamps.
3. Reuse the cached executable names when the fingerprint matches.
4. Otherwise scan executable files, deduplicate, sort and atomically replace the index.

Default location: `$XDG_CACHE_HOME/eita/path-index.json` or `~/.cache/eita/path-index.json`. Override the directory with `EITA_CACHE_DIR`.

**Important limitations:** this is *metadata-based invalidation*, not an always-running file watcher. A changed executable permission or file contents without a parent-directory timestamp change may require manual cache deletion. `FPATH` changes invalidate the executable index, but Zsh function definitions are **not yet indexed**. Future work will add directory watching, robust generation tracking and shell-event hooks.

### 📦 Knowledge sources

| Source | Status | Notes |
| --- | --- | --- |
| Built-in commands and common subcommands | ✅ | Git, GitHub CLI, Docker, Cargo, package managers and more |
| Executables in `PATH` | ✅ | No executable invocation |
| Persistent PATH index | ✅ | Fingerprint-based cache |
| Custom TSV specs | ✅ | `command<TAB>description` |
| Zsh aliases, functions and `fpath` definitions | 🗓️ | Shell adapter and incremental index |
| Native `_arguments` / `compdef` specs | 🗓️ | Structured completion knowledge |
| Man pages and `--help` ingestion | 🗓️ | Safe opt-in parsing |
| Accepted positional arguments and dynamic values | 🗓️ | Typed, context-sensitive providers |

Custom specs live at `~/.config/eita/specs.tsv` (or `$XDG_CONFIG_HOME/eita/specs.tsv`). Override with `EITA_SPECS`.

```tsv
git commit --signoff	Add Signed-off-by trailer
mytool deploy --dry-run	Preview deployment
```

Use a **literal tab** between the command and description.

## 🎛️ Experience roadmap

- [x] Opt-in ZLE widget with configurable key binding
- [x] Inline Ratatui selection and safe cancellation
- [x] CLI JSON/lines output
- [x] Typo-tolerant root and nested command matching
- [x] Cached executable discovery with metadata invalidation
- [ ] Configurable popup width, height, position (above/below), theme and keyboard controls
- [ ] Automatic vs manual popup trigger, debounce and minimum input length
- [ ] Ghost-text autosuggestions with accept-word/accept-line actions
- [ ] Cursor-aware edits preserving surrounding text, quoting and pipes
- [ ] Unknown-command correction with ranked suggestions
- [ ] Persistent ranking from local command history (opt-in)
- [ ] Zsh aliases, functions, completions and `fpath` index
- [ ] Structured subcommand/flag/argument specs and value providers
- [ ] Background service / daemon with optional file-system notifications
- [ ] Incremental indexing, bounded cache, versioning and self-healing
- [ ] Configurable exclusions and privacy controls
- [ ] Benchmark suite: Frizbee vs Nucleo vs FZF; cold/warm latency, memory and ranking quality
- [ ] Accessibility and terminal compatibility matrix
- [ ] Automated installation and upgrade diagnostics

See [architecture](docs/ARCHITECTURE.md) and [release engineering](docs/RELEASES.md) for deeper implementation details.

## 🛠️ Build & test

Requires Rust **1.89+** and Zsh for the integration:

```sh
cargo build --release
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
cargo run -- suggest --line 'git commit --am'
cargo run -- bench --iterations 1000
```

The benchmark measures matching **after** catalog loading. It does not measure process startup, disk I/O, index invalidation or TUI rendering.

### Release and PR previews

```sh
curl -fsSL https://shell.migos.club/install/eita | sh
curl -fsSL https://shell.migos.club/install/eita/pr/2 | sh
```

These short URLs require the corresponding route in [Shell Workbench](https://shell.migos.club) to be deployed. GitHub prerelease previews use `pr-<number>` and do not replace `latest`. See [release documentation](docs/RELEASES.md).

## 🤝 Contributing

Contributions, bug reports, performance traces and terminal compatibility findings are welcome!

1. Open an issue describing the behavior and reproduction steps.
2. Branch from the latest `main`, keeping changes focused on a coherent feature.
3. Add isolated Rust unit tests for matching, parsing and index invalidation.
4. Run `cargo fmt --all -- --check && cargo clippy --all-targets -- -D warnings && cargo test --all-targets`.
5. Submit a PR explaining user-visible changes, trade-offs and benchmark impact.

**Engineering principles:** no surprise command execution; fast warm-path latency; explicit privacy boundaries; cross-platform behavior; graceful fallback without optional integrations; clear separation of shipped features and experiments. Prefer deterministic parsers and incremental indexes over running arbitrary shell commands to discover metadata.

## 🔒 Security & privacy

Eita reads command names and user-provided specs locally. PATH discovery does **not** execute the discovered programs. The suggestion engine only returns replacement text. Future history-based learning and daemon features must be opt-in, local-first and documented with retention controls.

## 📜 License

MIT. Built with Rust, Frizbee, Ratatui and a healthy respect for the Enter key. 🦀
