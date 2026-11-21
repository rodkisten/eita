# 🔐 Eita Security Policy

Eita is a local shell-assistance tool, not a command sandbox. It offers suggestions and safely stages edits, but **users remain responsible for reviewing commands before executing them**.

## Supported releases

Security fixes target the latest `main` release and currently maintained PR previews. Old preview tags are not long-term supported versions.

## Reporting vulnerabilities

Please **do not open a public issue containing exploit details or sensitive data**. Use GitHub's private vulnerability reporting for this repository when available, or contact the maintainers privately via the repository's published contact channels. Include a minimal reproduction and affected commit or release. Do not disclose private terminal content.

## Trust boundaries

| Surface | Protection | Important limitation |
| --- | --- | --- |
| Zsh plugin | Handles editable text, never invokes a suggestion | Other plugins can own keymaps or terminal drawing |
| Completion/manpage parser | Bounded static reads, no code evaluation | Complex grammars may be intentionally ignored |
| Local usage learning | Opt-in allowlisted templates; no raw history persistence | `preexec` sees the original line **transiently** before sanitization |
| Rust daemon | Unix socket in 0700 directory, 0600 socket, bounded messages | Other processes running as the same local user may reach the socket |
| Cache | Versioned JSON, 0600 writes, capped size, atomic rename | `mtime`+size cannot detect every possible content-preserving edit |
| Installer | HTTPS release archives and SHA-256 checksum validation | SHA-256 alongside an archive is not a cryptographic publisher signature |

Eita never intentionally sends command usage or personal data over the network and has no cloud-learning service. It may read PATH, fpath and local manpage directories, subject to file and directory limits. Keep those paths restricted to trusted local content.

## Security-focused contributions

Prefer conservative parsing over clever execution. Keep parsers read-only, preserve prior Zsh handlers and rollback paths, sanitize filenames and arguments, avoid outputting secrets in CI, and test malformed input at every boundary. See [CONTRIBUTING.md](CONTRIBUTING.md).
