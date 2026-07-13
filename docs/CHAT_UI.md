# Chat TUI

## Purpose

Interactive repository assistant for asking questions about the indexed codebase.
Runs as `kode chat` (fullscreen) or `kode chat -m "..."` (one-shot).

## Architecture

Patterned on the proven `rusts/kode` agent TUI (async `select!` loop, tool
events, `tui-textarea`), not the unfinished multi-tab design draft.

```
┌────────────────────────────────────────────────────────────┐
│  kode · model · provider                          [status] │
├────────────────────────────────────────────────────────────┤
│  chat                                                      │
│    you / kode / status bubbles                             │
│    ⚙ tool start / ✓ tool result lines                      │
├────────────────────────────────────────────────────────────┤
│  input · Enter send · Ctrl+J newline                       │
├────────────────────────────────────────────────────────────┤
│  status …                          tools · /help · Ctrl+C  │
└────────────────────────────────────────────────────────────┘
```

### Layers

| Layer | Location | Role |
|-------|----------|------|
| CLI wire-up | `tools/cli/src/chat/mod.rs` | Config → `Agent` + optional `SymbolIndex` |
| TUI loop | `tools/cli/src/chat/tui.rs` | `tokio::select!` on keys / stream / tick |
| Draw | `tools/cli/src/chat/ui.rs` | Header, bubbles, input, footer |
| State | `tools/cli/src/chat/state.rs` | Bubbles, scroll, slash commands |
| Agent | `crates/agent` | Tool loop + OpenAI-compatible providers |
| Tools | `crates/agent/src/tools` | `list_dir`, `grep`, `read_file`, `search_symbols`, `find_symbol` |

The TUI never calls HTTP directly. It receives `AgentEvent`s
(`Delta`, `ToolStart`, `ToolResult`, `Done`, `Error`).

### Knowledge graph

If `.kode/cache.db` exists, symbols are preloaded into a `Send + Sync`
`SymbolIndex` so the agent can run on worker tasks without holding
non-`Sync` storage backends.

Without a scan, chat still works with filesystem tools only and suggests
`kode scan`.

## Keyboard

| Key | Action |
|-----|--------|
| Enter | Send message |
| Ctrl+J | Newline |
| Ctrl+L | Clear conversation |
| Ctrl+C | Quit |
| PgUp / PgDn | Scroll chat |
| Ctrl+Up / Ctrl+Down | Scroll line |
| `/help` | Slash help |
| `/tools [on\|off]` | Toggle tools |
| `/clear` | Clear history |
| `/quit` | Exit |

## Config

`.kode/config.toml` or environment:

| Key / env | Meaning |
|-----------|---------|
| `chat.provider` | `openai`, `ollama`, `groq`, … |
| `chat.model` | Model id |
| `chat.api_key` / `OPENAI_API_KEY` / `OLLAMA_API_KEY` | Auth |
| `chat.base_url` / `OLLAMA_HOST` | API base |

## Files

- `tools/cli/src/chat/` — TUI client
- `crates/agent/` — agent runtime + tools
- Reference: out-of-tree `~/rusts/kode` for interaction model
