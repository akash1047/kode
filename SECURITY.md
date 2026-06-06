# Security Policy

## Supported versions

| Version | Supported |
|---------|-----------|
| 0.1.x   | Yes       |

## Reporting a vulnerability

**Do not open a public GitHub issue for security vulnerabilities.**

Report vulnerabilities via one of:

1. **GitHub private vulnerability reporting** — use the "Report a vulnerability" button on the Security tab of this repository.
2. **Email** — send details to akashlohar1047@gmail.com with subject `[kode] Security vulnerability`.

Include:
- Description of the vulnerability
- Steps to reproduce
- Affected version(s)
- Potential impact

You will receive an acknowledgement within 48 hours and a resolution timeline within 7 days.

## Scope

kode is a read-only local tool. It reads source files from disk and queries an LLM API over HTTPS. It does not:
- Write to source files
- Execute code from the project
- Store credentials beyond `~/.config/kode/config.toml` (mode 0600)

The primary attack surfaces are:
- The SQLite cache at `~/.cache/kode/` (local only)
- API keys stored in config or env vars
- Malicious tree-sitter query files embedded in a project
