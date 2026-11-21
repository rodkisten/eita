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

The current implementation matches **known catalog entries**, not arbitrary dynamically discovered flag schemas. Structured JSON specs now support declared flags, positional values, and both `--flag=value` and `--flag value` completions. Quote-aware cursor replacement and live Zsh completion function introspection remain planned.

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
| Passive Zsh completion file discovery | 🧪 | Reads `_command` files from `FPATH`; simple quoted option specs only |\n| Zsh aliases, functions and live `compdef` resolution | 🗓️ | Shell adapter and incremental index |
| Full `_arguments`, `_describe` and `compdef` interpretation | 🗓️ | Requires a richer static parser |
| Passive man-page extraction | 🧪 | Experimental: plain/gzip section 1/8 pages, simple roff flags, metadata cache |\n| Nested man-page commands and `--help` | 🗓️ | Rich parsing and trusted opt-in probing |
| Declared positional values and flag values | ✅ | JSON schemas, static values |
| Dynamic argument value providers | 🗓️ | Context-sensitive runtime sources |

Custom specs live at `~/.config/eita/specs.tsv` (or `$XDG_CONFIG_HOME/eita/specs.tsv`). Override with `EITA_SPECS`.

```tsv
git commit --signoff	Add Signed-off-by trailer
mytool deploy --dry-run	Preview deployment
```

Use a **literal tab** between the command and description.

### 🐚 Learning from Zsh completion definitions

The experimental passive reader checks `FPATH` for `_command` completion files, including one level of category subdirectories. It recognizes simple quoted option specifications such as `'--amend[Amend previous commit]'` and `'--cleanup=-[Cleanup mode]'`. It **never sources shell functions**. This is a narrow static extraction, not execution of the Zsh completion engine: dynamically generated options, `compdef` aliases, nested subcommands, `_describe` and conditional specifications are not supported yet. The file reader currently runs on each suggestion and does not yet have a dedicated persistent cache.

### 🧠 Learning from installed documentation

Eita now includes an **experimental passive man-page reader**. When suggesting a command, it checks local uncompressed and gzip-compressed man pages in sections 1 and 8 for simple roff flag declarations. This does not execute the command or run a man renderer. Coverage is currently limited: advanced roff macros, nested subcommands, option descriptions and dynamic argument types need further work.

**Long-term source precedence:** installed Zsh completions and documented CLI schemas → parsed man pages → explicitly trusted `--help` probes → user JSON fallback. Actual source reconciliation, provenance and persistent knowledge caching are planned; today the JSON and passive man results are combined. JSON is primarily for fixtures and undocumented tools.

### 🧩 Structured command specs

For test fixtures or undocumented commands, copy [examples/commands.json](examples/commands.json) to `~/.config/eita/commands.json`, or set `EITA_COMMAND_SPECS` to a custom JSON file.

```sh
mkdir -p ~/.config/eita
cp examples/commands.json ~/.config/eita/commands.json
eita suggest --line 'git commit --cleanup=st'
eita suggest --line 'gh pr list --state mer'
eita pick --line 'docker compose up --pu'
```

Each entry declares a full `command` path, optional `flags` with `name`, `description`, `takes_value`, `values`, and optional `arguments` with `name` and static `values`. The engine resolves the command path first, then offers valid flags and declared values. JSON files are read locally; Eita does not execute the described commands. Values are currently prefix-matched.

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
- [x] Experimental passive Zsh completion file scanning for simple quoted flags\n- [ ] Zsh aliases, functions, live `compdef` mapping and comprehensive `fpath` index\n- [x] Experimental gzip man-page parsing and fingerprint-based knowledge cache
- [x] Structured JSON specs for command paths, flags, positional values and static flag values
- [ ] Dynamic value providers, shell-aware quoting and fuzzy ranking for spec values
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
