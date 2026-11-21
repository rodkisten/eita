# 🗺️ Eita Engineering Roadmap

*A fast shell helper is useful only when it keeps the shell yours.* 🦀

This living plan documents **what exists**, **what is experimental**, and **what is not yet implemented**. It is not a promise of dates. Every milestone should produce a complete, reviewable feature block and an accurate README update.

## ✅ Foundation: available today

| Capability | Implementation | Evidence |
| --- | --- | --- |
| Context-aware completion and fuzzy ranking | `src/context.rs`, `src/engine.rs` | Rust unit tests and CLI smoke |
| Compact Ratatui picker | `src/ui.rs` and TOML themes | CI, manual terminal inspection still important |
| ZLE ghost text with explicit accept | `integrations/zsh/eita.plugin.zsh` | Native Zsh socket integration test |
| Unknown-command typo recovery | `src/recover.rs` and handler fallback | Error-path Zsh integration test |
| Local safe-template learning | `src/usage.rs` | Persistence/privacy tests |
| Persistent Unix socket engine, 256-entry LRU | `src/daemon.rs` | IPC benchmark and CI |
| Incremental static Zsh/manpage indexing v2 | `src/completion.rs`, `src/man.rs` | Changed/unchanged/removed tests |
| Hierarchical command/subcommand scope | `src/engine.rs` | Nested negative/positive tests |
| Linux event-driven auto-indexing | `src/watch.rs` | Experimental: inotify + debounce |
| Portable Linux x86-64 Musl preview | `.github/workflows/release.yml` | Build + ELF smoke |

## 🧪 M1: Indexer hardening and discovery (in progress)

**Goal:** newly installed tools and new Zsh completions appear without manual reindex and without ongoing directory traversal.

- Native Linux inotify subscriptions to non-recursive PATH/fpath directories.
- Small in-memory comparison of PATH/fpath in the Zsh precmd hook; notify daemon only on changes.
- 350 ms event debounce; indexed file snapshots reuse unchanged content.
- Correct cache invalidation and daemon refresh when an event results in a real change.
- Explicit recovery: `eita daemon sync`, `eita daemon reload`, `eita-index`.

**Acceptance:** create, rename, update and delete a test executable and a test completion; confirm results update and disappear. With no changes, 100 prompts cause **zero index scans** and **zero Eita child processes from the auto-sync hook**. CI must not depend on arbitrary external tools.

**Remaining:** robust watch reattachment after replacing a watched directory, missed/overflow event recovery, hash-based content verification, macOS FSEvents/kqueue backend, per-session source isolation, opt-out lifecycle controls for all platforms, and watchdog memory/resource profiling.

## 🌳 M2: Deep hierarchical completion (in progress)

**Goal:** `git commit --a` and `docker compose up --` use the correct command scope, not root-level options from unrelated subcommands.

- Index a path of command tokens, with the deepest *fully recognized* prefix as context.
- Search only descendants of that path; keep typo tolerance on the unfinished token.
- Parse static Zsh `_arguments` value requirements and conservative roff option tokens.
- Present required-value hints such as `--message=` → `expects <MSG>`.
- Never fill untrusted argument values or execute dynamically defined completions.

**Acceptance:** tests cover scoped positive and negative results, cursor replacement with pipes/quotes, no contamination between sibling commands, user specs for nested commands, and option descriptions that survive caching/reload.

**Remaining:** typed argument schema (file, dir, enum, string, number), mutually exclusive flag groups, repeated flags, option/value separator handling, nested dynamic Zsh grammar, alias expansion metadata (without execution), structured usage-aware ranking, and verified option descriptions for each version of a tool.

## 🎨 M3: Truly anchored popup

**Goal:** small, tasteful overlays above or below the **cursor**, not merely a narrow box within a full-width inline viewport.

- Build a cooperative ZLE rendering adapter that owns a bounded cursor region.
- Restore scrollback, selection and cursor after Esc, submit, resize and interruption.
- Responsive layout for iPhone SSH clients and compact terminals.
- Configurable theme, dimensions, max rows, shortcuts, trigger, and accessible focus treatment.
- Keep Ratatui inline mode as the safe fallback when cursor geometry is unavailable.

**Acceptance:** no visible terminal corruption in Zsh, iOS SSH, tmux and normal terminals after 100 open/close cycles, resize, SIGINT or EOF.

**Remaining:** implementation and comprehensive manual terminal testing. Current `above` and `below` settings align only inside the reserved inline region.

## ⚡ M4: Ultra-low-latency engine

**Goal:** real-world input-to-suggestion latency with a predictable resource ceiling.

- Distinguish matcher-only, warm socket IPC, cold startup, ZLE hook, and render timing.
- Use per-session path indexes to avoid swapping a shared active PATH between shells.
- Eliminate blocking fallback on every keypress; explore cancelable async requests.
- Cache parsed tokens/scopes and repeated prefixes; bound memory and eviction policy.
- Instrument cache hit/miss, watcher events, rebuild duration and rendering cost.

**Acceptance:** report p50/p95/p99 and peak RSS for warm/cold workloads on Linux and at least one iPhone SSH client. Define a budget **after** establishing a reproducible baseline. Never treat benchmark runner microseconds as user-visible latency.

## 🧰 M5: Ecosystem and publishing

- Pluggable, versioned read-only command schemas and package-specific opt-ins.
- Clear integration with Zsh completions, aliases, manpages and command wrappers.
- Optional help/descriptions panel similar in purpose to Fig, without replacing core ZLE workflows.
- User-controlled sync of curated command schemas. No untrusted script evaluation.
- Linux ARM64 and macOS release matrix with reproducible package verification.
- Documented installs, upgrades, preview cleanup, release notes and compatibility matrix.

## 📏 Engineering principles

1. **Fast first.** No I/O-heavy discovery on the typing path.
2. **Shell-safe by default.** Never execute suggestions or parse arbitrary source by evaluation.
3. **No secret collection.** Opt-in template learning only; raw command history is not a data source.
4. **Respect the terminal.** Compose with native ZLE instead of overriding everything.
5. **Bounded resource use.** Memory, file count, bytes, processes, sockets and timeouts have limits.
6. **Honest status.** Experimental features stay labeled experimental until proven in production.
7. **Broad compatibility.** Keep CLI and manual indexing as functional fallbacks.

Each accepted milestone must update the README, architecture notes and relevant tests in the same PR. See [CONTRIBUTING.md](../CONTRIBUTING.md).
