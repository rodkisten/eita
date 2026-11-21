# ⚡ Persistent Engine & Safe Learning

Eita can run as a small, **optional Rust daemon**. It keeps command candidates in RAM, reuses up to 256 recent query results, and uses a user-local Unix domain socket instead of creating a new Rust process for every suggestion.

## CLI lifecycle

~~~sh
eita daemon start          # start, or report existing daemon
eita daemon status         # PID, uptime, catalogue/cache sizes, watcher state
eita daemon bench          # 200 warm IPC + matching requests, p50/p95/p99
eita daemon sync           # send current PATH and EITA_FPATH/FPATH
eita daemon reload         # force reload from disk
eita daemon socket         # print socket path
eita daemon stop           # shutdown; ZLE can still fall back to local CLI
~~~

Enable from interactive Zsh:

~~~zsh
source "$HOME/.local/share/eita/eita.plugin.zsh"
eita-daemon-on
eita-inline-on
eita-auto-index-on
eita-learn-on
eita-daemon-status
~~~

`eita-daemon-off` disables daemon use in the current shell and removes its auto-index change hook. Other shell sessions may continue using the daemon. `eita daemon stop` stops the shared process.

## Architecture and transport

The daemon binds `$XDG_RUNTIME_DIR/eita/engine.sock` (or a cache directory fallback), with a user-owned `0700` parent and a `0600` socket. Override for isolated development with `EITA_DAEMON_SOCKET`.

Two bounded protocols share that socket:

- **JSON Lines** for status, reload, sync, suggest, inline and stop.
- **A small literal text protocol** for the Zsh inline widget. Zsh's native `zsocket`/`zselect` builtins connect directly without launching the Rust binary on every changed buffer.

The widget's socket wait is limited to 40 ms; on failure, the regular CLI path remains available. The daemon uses read/write timeouts, a maximum request size, memory-bounded caches, and an approximately **15-minute idle timeout**. It starts in a detached Unix session so it need not die when SSH disconnects.

## 📦 Warm caches

1. Rust loads the built-ins, executable filenames, safe learned templates and persisted index.
2. A 256-entry LRU stores recent inline and suggestion results in memory.
3. Index cache files are watched for changes; a changed index invalidates RAM.
4. On Linux, a native inotify worker watches PATH/fpath source directories without rescanning them on every prompt.
5. Zsh sends changed PATH/fpath strings to the daemon only when they differ from the last prompt.
6. The daemon uses one active PATH environment at a time. If a request comes from a different environment, it returns a safe fallback instead of guessing. A future revision will isolate catalogues by shell/session.

No TCP service, cloud database, or remote completion API exists.

## 🧠 What it learns

Learning is explicitly **opt-in**:

~~~zsh
eita-learn-on
# Work normally. Only approved command/subcommand shapes are saved.
eita usage status
eita-learn-off
eita usage clear    # erase the local usage template database
~~~

The Zsh preexec hook temporarily receives the raw command line, sends it via stdin rather than a process argument, and the Rust parser persists only recognized, allowlisted templates. Values such as paths, hosts, tokens, messages, arguments, and full history are **not** stored. New templates become eligible after two uses; frequency bonuses are bounded.

Examples:

| Typed command | Persistable template |
| --- | --- |
| `git commit -m "private message"` | `git commit` |
| `git worktree add /private/path` | `git worktree` |
| `gh pr view 123` | `gh pr` |
| `ssh secret.example.com` | Nothing |
| `echo TOKEN=secret` | Nothing |

## Auto-index source sync

~~~zsh
eita-auto-index-on
# Add a directory to PATH or fpath; the next prompt sends a sync
# and native Linux watchers pick up subsequent filesystem changes.
eita-auto-index-status
eita-auto-index-off
~~~

The auto-index worker never runs in a ZLE keypress callback. It coalesces source events for 350 ms and only indexes relevant file changes. If the watcher is unavailable, explicit commands remain supported.

## 🧪 Performance methodology

~~~zsh
eita daemon bench
eita bench --iterations 500 --compare
eita daemon status
~~~

Daemon benchmarks are **warm Unix socket round trips + matcher + serialization**, not cold process startup, terminal redraw, or iPhone SSH network latency. Report run date, platform, kernel, command corpus size and p50/p95/p99 when comparing measurements. Historical numbers in the README are observations, not a production SLA.

## Security and limitations

- Same-user processes may access the socket. The permissions do not isolate mutually untrusted processes under the same UID.
- Manpage and completion sources are read as data, never executed.
- The daemon's active PATH is currently shared. Very different parallel shells may invalidate each other's warm cache; a per-session index is planned.
- Linux native filesystem event watching is experimental. Missed directory replacement/overflow events may require `eita daemon sync` or `eita-index`.
- All private learning is local and opt-in, with allowlisted templates only.

See [SECURITY.md](../SECURITY.md), [INDEXING.md](INDEXING.md), and [ROADMAP.md](ROADMAP.md).
