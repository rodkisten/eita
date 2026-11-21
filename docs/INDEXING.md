# 📚 Indexing & Command Knowledge

**Static discovery, hot suggestions.** Eita separates the expensive discovery phase from keystroke-time ranking.

## Sources

| Source | How it is discovered | Opt-in? | Executes code? |
| --- | --- | --- | --- |
| Built-in command catalogue | Curated Rust data | No | No |
| `PATH` | Names of executable files | No | No |
| Zsh `fpath` | Literal `_arguments` and `commands=(...)` | `eita-index` or daemon auto-index | No |
| Manpages `.1`, `.1p`, `.8`, `.gz` | Static roff `.SH`, `.TP`, `.B`/`.BR`/`.BI` | Yes, `--man-dir` | No |
| User `specs.tsv` | One candidate and description per tab-delimited line | Yes | No |
| Learning | Frequency of allowlisted command shapes | Yes, `eita-learn-on` | No |

Dynamic completion functions are deliberately **not evaluated**, and `compinit` continues to own normal Tab completion.

## Manual indexing

~~~zsh
source "$HOME/.local/share/eita/eita.plugin.zsh"
eita-index                                   # current Zsh fpath
eita-index --man-dir /usr/share/man          # opt-in local manpages
eita index --status                          # inspect cached file counts
eita daemon reload                           # refresh RAM immediately
~~~

For isolated tests:

~~~sh
eita index --refresh --fpath /opt/completions --man-dir /opt/man \
  --cache /tmp/eita-fixture-index.json

EITA_COMPLETION_CACHE=/tmp/eita-fixture-index.json \
  eita suggest --line 'orbit --ver' --format lines
~~~

The indexer accepts colon-separated paths, reads bounded source files, and writes a JSON cache with an atomic rename and permissions `0600`.

## Incremental index v2

Each source file snapshot contains an absolute or supplied path, size, nanosecond modification time, and parsed candidates. The manifest also persists the selected Zsh/manpage source roots for the background watcher.

When `eita index --refresh` runs again:

1. Discover eligible filenames and check their metadata.
2. **Reuse** records whose file size and mtime match.
3. **Reparse** new or changed files.
4. **Remove** records whose files disappeared.
5. Only write the JSON and reload the daemon if something actually changed.

Example:

~~~json
{
  "version": 2,
  "scanned_files": 120,
  "reused_files": 120,
  "updated_files": 0,
  "removed_files": 0,
  "unchanged": true
}
~~~

Existing version 1 caches remain readable; a rebuild upgrades to the current manifest. The metadata fingerprint is intentionally inexpensive, not cryptographically collision-resistant: manually refresh or clear the cache if a tool replaces content while preserving both size and timestamps.

## 👀 Native auto-indexing (experimental)

On Linux, `src/watch.rs` uses inotify directory watches with a **350 ms debounce**. It subscribes to non-recursive `PATH` directories, Zsh `fpath` directories, and opted-in manpage source folders.

- File creation/removal/move/close-write/attribute events trigger a source-specific update.
- PATH events invalidate the executable catalogue.
- fpath/manpage events call the **incremental** indexer in a background worker.
- On normal prompts, Zsh compares source-path strings in memory. Only an actual PATH/fpath change launches `eita daemon sync` asynchronously.
- On unchanged prompts the auto-index hook performs **no filesystem scan** and **does not spawn a child process**.
- The CLI and manual indexing remain functional if the daemon or watcher is disabled.

~~~zsh
eita-daemon-on
eita-auto-index-on
eita-auto-index-status

# After installing a CLI tool or adding a Zsh completion:
eita daemon status
# If an event was missed:
eita daemon sync
eita-index
~~~

`eita-auto-index-off` removes the current shell's change-notification hook. A daemon that was already started with native monitoring continues watching its existing directories until it stops; use `eita daemon stop` and restart with `EITA_AUTO_INDEX_ENABLED=0` to disable background native watching entirely.

**Limitations:** inotify is Linux-specific; non-Linux platforms retain explicit synchronization but no continuous native watcher. An atomic replacement of an entire watched directory may require an explicit sync/restart. inotify queue overflow recovery and monitoring multiple divergent shell environments are planned.

## 🌳 Command hierarchy

Candidates are represented as *complete shell-editable prefixes* with a source and description:

~~~text
git
git commit
git commit --amend
git commit --message=     expects <MSG>
git checkout --ours
~~~

During ranking, Eita identifies the **deepest recognized complete token prefix**, then restricts candidates to that subtree:

- `git com` → search `git` descendants, fuzzily match `com`.
- `git commit --a` → search `git commit` descendants; `checkout` flags are excluded.
- `git commit --message=` → show the flag and its static argument hint when available.

The current static parser can retain a trailing equals sign on an option and annotate requirements from Zsh `_arguments` or simple roff headings. It does **not** complete actual file paths, read flag values from running programs, or enumerate dynamic choices. Typed argument providers and exclusion groups are planned.

## Security & limits

The indexer is bounded by file count (2,500), source file size (256 KiB each), expanded manpage content (512 KiB), candidate count (20,000), and cache size (8 MiB). Nonliteral and dangerous syntax is skipped. No source code is executed.

User-supplied paths are treated as data, not evaluated as shell expressions. Keep custom source folders restricted to trusted files. For detailed trust boundaries, see [SECURITY.md](../SECURITY.md).

## Benchmarks

~~~zsh
eita daemon start
eita daemon bench
eita daemon status
eita bench --iterations 500 --compare
~~~

This measures hot IPC and the matcher, **not** full ZLE input-to-display latency over SSH. See [architecture](ARCHITECTURE.md) and the [roadmap](ROADMAP.md).
