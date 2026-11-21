#!/usr/bin/env bash
# End-to-end Linux inotify + daemon + hierarchical context checks.
set -euo pipefail

binary="${EITA_BIN:-./target/release/eita}"
workspace="$(mktemp -d)"
cleanup() {
  "$binary" daemon stop >/dev/null 2>&1 || true
  rm -rf -- "$workspace"
}
trap cleanup EXIT

mkdir -p "$workspace/bin" "$workspace/fpath" "$workspace/daemon"
export EITA_DAEMON_SOCKET="$workspace/daemon/engine.sock"
export EITA_COMPLETION_CACHE="$workspace/index.json"
export EITA_FPATH="$workspace/fpath"
export EITA_AUTO_INDEX_ENABLED=1
export EITA_DAEMON_ENABLED=1
export PATH="$workspace/bin:$PATH"

"$binary" daemon start
sleep 0.5  # let the native watcher subscribe before creating files

has_exact() {
  "$binary" suggest --line "$1" --format lines | grep -Fxq -- "$2"
}
await_present() {
  local attempt
  for attempt in $(seq 1 30); do
    if has_exact "$1" "$2"; then return 0; fi
    sleep 0.2
  done
  echo "Missing suggestion: $2 for $1" >&2
  "$binary" daemon status >&2 || true
  exit 1
}
await_absent() {
  local attempt
  for attempt in $(seq 1 30); do
    if ! has_exact "$1" "$2"; then return 0; fi
    sleep 0.2
  done
  echo "Stale suggestion: $2 for $1" >&2
  exit 1
}

printf '#!/bin/sh\nexit 0\n' > "$workspace/bin/crab-runner"
chmod +x "$workspace/bin/crab-runner"
await_present 'crab-runner' 'crab-runner'

printf "#compdef crabmark\n_arguments '--quiet[Quiet output]'\n" > "$workspace/fpath/_crabmark"
await_present 'crabmark --q' 'crabmark --quiet'

has_exact 'git commit --a' 'git commit --amend'
if "$binary" suggest --line 'git commit --a' --format lines | grep -q '^git checkout '; then
  echo "Hierarchy allowed options from an unrelated subcommand" >&2
  exit 1
fi
"$binary" config zsh | grep -F $'EITA_AUTO_INDEX_ENABLED\t1'
"$binary" daemon status | grep -F '"auto_index":true'

rm -f -- "$workspace/bin/crab-runner" "$workspace/fpath/_crabmark"
await_absent 'crab-runner' 'crab-runner'
await_absent 'crabmark --q' 'crabmark --quiet'

EITA_FPATH="$workspace/fpath" "$binary" index --refresh > "$workspace/first-report.json"
EITA_FPATH="$workspace/fpath" "$binary" index --refresh > "$workspace/second-report.json"
grep -F '"unchanged": true' "$workspace/second-report.json"

echo "🦀 Auto-index, hierarchical ranking and cache tests passed"
