# 🧱 Eita Architecture

## System overview

Eita has two independently usable paths: a regular CLI and an optional Zsh + local-daemon hot path.

~~~text
ZLE keymap / command_not_found_handler / precmd
      │
      ├─ picker: BUFFER, LBUFFER, RBUFFER ──────> Rust CLI ─────> Ratatui
      ├─ ghost: direct zsocket ───────────────────────────┐
      └─ source signature change: daemon sync ───────┐   │
                                                     ▼   ▼
                                           Unix domain socket (0600)
                                                     │
                                         Rust daemon (user scoped)
                                         ├─ source environment
                                         ├─ catalogue + 256 LRU
                                         ├─ Frizbee hierarchy ranking
                                         ├─ learned templates
                                         └─ incremental index cache
                                                     ▲
                                        native inotify worker (Linux)
                                        PATH and fpath directory events
~~~

## Source ownership

- `src/context.rs`: conservative parser for the active unquoted command segment, with cursor-safe replacement.
- `src/engine.rs`: hierarchical scoping and Frizbee fuzzy ranking; usage bonuses are applied after filtering.
- `src/catalog.rs`: built-in candidates, executable filenames in PATH, custom TSV specs, cached Zsh/manpage candidates and safe learning.
- `src/completion.rs`: incremental source index, versioned per-file snapshot manifest (mtime/size), atomic JSON output.
- `src/man.rs`: bounded static roff parser; gzip decoded locally using Rust; no external command or formatter.
- `src/watch.rs`: worker thread, native inotify on Linux, 350 ms debounce, source-path updates and index refresh away from the input thread.
- `src/daemon.rs`: single local Unix listener, request validation, warm LRU and lifecycle (15-minute idle timeout).
- `src/usage.rs`: opt-in template frequency; no user commands, arguments or history persisted.
- `src/ui.rs`: Ratatui selector; intentionally conservative inline viewport.
- `integrations/zsh/eita.plugin.zsh`: scoped ZLE widgets, named hooks and direct built-in socket operations.

## The hot path

When enabled, `line-pre-redraw` calls a Zsh function that checks whether `LBUFFER` actually changed. If so, it sends the bounded inline request over a local Unix socket using built-in `zsocket` and `zselect`. A 40 ms timeout prevents unbounded blocking; CLI matching remains a fallback.

The daemon services the request from its already loaded catalogue, applies the hierarchy ranking and bounded LRU, and returns a literal suffix. ZLE writes that suffix only to `POSTDISPLAY`. The original buffer is not modified until the user explicitly accepts the suggestion.

The `eita suggest` CLI also has an opt-in daemon JSON path. A changed `PATH` triggers safe local fallback when the daemon's active environment differs.

## Event-driven indexing and resynchronization

1. On daemon start, index source roots are read from the Zsh environment or the persisted index manifest. No recurring directory scan is scheduled.
2. On Linux, inotify watches explicit non-recursive directories from PATH/fpath and, when enabled, manpage source roots. Relative paths are excluded.
3. Create/remove/move/write/attribute events mark a source type dirty. The worker coalesces events over 350 ms.
4. PATH events trigger an in-memory executable catalogue reload. fpath/manpage events call the existing **incremental** indexer in the background, retaining unchanged files and removing deleted entries.
5. The daemon observes indexed file timestamp changes and refreshes its catalogue. An unchanged index avoids disk writes.
6. Zsh `precmd` compares the current PATH/fpath strings with their previously seen values. It launches a detached `eita daemon sync` **only when they differ**.

Important limitations: watcher events may be missed during directory replacement, overflow or shutdown. Linux inotify is experimental, not a substitute for `eita-index` / `eita daemon reload`. Other OSes have explicit source-sync fallback but no native continuous watcher. The daemon currently supports one active PATH environment at a time; multi-shell isolation remains planned.

## Hierarchy and argument hints

A partially typed last token is not considered a complete command ancestor. For example:

- `git com` is matched within the `git` namespace.
- `git commit --a` is matched within the `git commit` namespace, excluding `git checkout` options.
- `git commit --message=` can display an argument requirement extracted from an opt-in static source, but never invents a value.

The parser accepts a static subset of `_arguments`, simple subcommand arrays and manpage OPTIONS/COMMANDS sections. Complex Zsh functions, dynamically computed values and full shell AST parsing remain outside the current implementation.

## Data formats and boundaries

| Data | Location | Characteristics |
| --- | --- | --- |
| User configuration | `$XDG_CONFIG_HOME/eita/config.toml` | Validated TOML; no eval |
| Cached source index | `$XDG_CACHE_HOME/eita/zsh-completions.json` | Versioned JSON, size limited, atomic rename, 0600 |
| Learned frequency templates | `$XDG_DATA_HOME/eita/usage.json` | Opt-in, allowlisted templates, private permissions |
| Socket | `$XDG_RUNTIME_DIR/eita/engine.sock` or cache fallback | User-owned directory 0700, socket 0600 |
| User candidates | `$XDG_CONFIG_HOME/eita/specs.tsv` | Data only; never executed |

The daemon request protocol includes bounded JSON Lines operations (`status`, `sync`, `reload`, `suggest`, `inline`, `stop`) and a simple Zsh inline protocol. Request size limits and timeouts prevent unbounded requests. Local same-user processes can access the socket: this is user-level isolation, **not** a security boundary against the current account.

## CI, releases and measurements

The Rust CI checks formatting, Clippy, tests, release build and Zsh integration. The release workflow builds the enabled portable Linux Musl target, packages the binary/plugin with SHA-256 checksums and publishes a moving prerelease for open PRs. Stable `main` tags are immutable per commit.

`eita daemon bench` measures warm local IPC plus matching. `eita bench --compare` measures algorithmic matching in memory. Neither includes SSH network round trips or iPhone redraw. User-visible latency testing is a planned milestone.

See [INDEXING.md](INDEXING.md), [DAEMON.md](DAEMON.md), [ROADMAP.md](ROADMAP.md), and [CONTRIBUTING.md](../CONTRIBUTING.md).
