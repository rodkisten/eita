# 📦 Releases, PR Previews & Installation

## CI gates

GitHub Actions runs format checks, Clippy with `-D warnings`, unit tests, Zsh syntax/integration checks, installer syntax, and source/build smoke tests before releasing an archive.

The release pipeline is in [`.github/workflows/release.yml`](../.github/workflows/release.yml). It runs for every commit on `main`, on manual dispatch, and for open PR builds. Stable releases publish only from `main`, while repository-internal PRs can publish moving prereleases.

## Current distribution target

**Enabled in the current release matrix:** `x86_64-unknown-linux-musl` (portable x86-64 Linux, including modern Debian systems without relying on a recent host glibc).

The workflow contains commented examples for ARM64 and macOS, but these are **not enabled targets** and should not be advertised as already built. The matrix controls actual availability.

Each published archive contains:

~~~text
eita
integrations/zsh/eita.plugin.zsh
README.md
LICENSE
~~~

Every `.tar.gz` is accompanied by a `.tar.gz.sha256` file. The pipeline validates checksum pairs before publication. Artifacts are also retained in Actions for 30 days.

## Stable releases

A `main` SHA produces a tag `main-<first 12 sha characters>`. When the workflow runs again for the same SHA, it can update the corresponding release. Only the current `main` HEAD may be promoted to `latest`.

~~~sh
curl -fsSL https://raw.githubusercontent.com/rodkisten/eita/main/install.sh | sh
"$HOME/.local/bin/eita" --version
~~~

The installer validates the downloaded checksum and installs without Rust, GitHub CLI, jq, or sudo. It does not modify `~/.zshrc` or PATH for you. For isolated installs, use `EITA_INSTALL_DIR` and `EITA_PLUGIN_DIR`.

## Pull-request previews

Successful PR builds publish or update a moving `pr-N` tag and a matching GitHub prerelease, without promoting it to `latest`. A bot comment on the PR contains installation/download links.

~~~sh
curl -fsSL https://raw.githubusercontent.com/rodkisten/eita/main/install.sh | EITA_VERSION=pr-2 sh
~~~

Optional Shell Workbench shortcuts exist **only when that external router is deployed**:

~~~sh
curl -fsSL https://shell.migos.club/install/eita/pr/2 | sh
~~~

The raw GitHub installer and direct prerelease assets remain the independent fallback. A prerelease is removed when its PR closes; do not use moving PR tags as long-term reproducibility pins.

## Integrity, platform notes & source

- Linux builds use the statically linked Musl target and verify the executable ELF does not require a dynamic interpreter.
- The installer checks archives using SHA-256, but a checksum published beside an archive is **not** a signed supply-chain attestation.
- If an archive for your OS/architecture is absent, the installer fails instead of silently selecting a different target.
- Inspect the installer and pin release SHA/assets for sensitive environments.
- Format, tests, and runtime behavior on other platforms must be validated before enabling the target matrix.

See the [README](../README.md) and [CONTRIBUTING](../CONTRIBUTING.md) guides for development and release workflow conventions.
