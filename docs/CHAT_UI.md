# Chat TUI

## Purpose

Interactive chatbot interface for asking questions about the indexed codebase. Runs inside `kode chat` subcommand.

## Position

Layer: CLI presentation (top of stack). Depends on `ChatSession`/`ChatEngine` for networking, `InputBuffer` for text editing, `render_markdown` for message display. Zero dependency on acquisition/analysis/graph/storage crates.

## Architecture

Two mutually exclusive full-screen states occupying a single `[Min(1), Length(input)]` vertical split with 1-cell margin around the entire layout:

```
┌──────────────────────────────────────┐
│                                      │
│   [HERO] or [CHAT SESSION]          │  ← Constraint::Min(1)
│                                      │
├──────────────────────────────────────┤
│ ┌──────────────────────────────────┐ │
│ │  > input (1–10 lines)           │ │  ← Constraint::Length(line_count + 3)
│ │  Ctrl-D exit · Shift-Enter nl   │ │
│ └──────────────────────────────────┘ │
└──────────────────────────────────────┘
```

### State machine

| State | Trigger | Renders |
|-------|---------|---------|
| **Hero** | `session.messages.is_empty()` on launch | Bordered info panel with repo name, model, tips |
| **Chat** | First non-empty message submitted | Gutter-prefixed conversation history |

Transition is one-way. No return to hero.

## Implementation

### Layout (`tui.rs:render`)

```rust
let line_count = self.input_buffer.content.lines().count().clamp(1, 10) as u16;
let input_height = line_count + 3; // border(1) + input + info(1) + border(1)
let [screen_area, input_area] =
    Layout::vertical([Constraint::Min(1), Constraint::Length(input_height)])
        .margin(1)
        .areas(frame.area());
```

Input height computed fresh each frame: content lines + 3 for borders + info bar. Min 4, max 13. Screen area gets all remaining space.

### Hero (`render_hero`)

```
╔══ kode chat — kode ═══════════════╗
║                                     ║
║  repo:   kode                      ║
║  model:  gpt-4 (8k ctx)            ║
║                                     ║
║  Tips:                              ║
║  • Type message, press Enter        ║
║  • Shift-Enter for multi-line       ║
║  • /help for commands               ║
║  • Ctrl-C cancel · Ctrl-D exit      ║
║                                     ║
╚═════════════════════════════════════╝
```

- `Block::bordered()` with title, centered via nested `Layout::vertical([Fill(1), Length(content), Fill(1)])`
- Content centered horizontally via `Layout::horizontal([Fill(1), Min(40), Fill(1)])`
- Each info line styled with `theme.status_fg`, tips with `theme.tip_fg`, title with `theme.hero_title`

### Chat session (`render_chat`)

```
▎what does resolve_symbol do?              ← green gutter (user)
▎
▎It traverses the knowledge graph…         ← cyan gutter (assistant)
▎```rust
▎pub fn resolve_symbol(...) -> Option<...>
▎```
▎                                         ← blank line separator
```

- Every line of every message prefixed with `▎` (`U+258E`) colored by role
- Role colors from `theme.gutter_user`, `theme.gutter_assistant`, `theme.gutter_system`
- No role labels (no `You`/`AI`/`System` text)
- Messages separated by blank line
- Top padding: blank line at start of chat
- Gutter spacing: 2 spaces after `▎` for breathing room
- During streaming: first `▎` replaced with braille spinner animation
- Thinking loader: when streaming but no tokens yet, shows `⠋ thinking...` with cycling spinner
- Markdown rendered by `render_markdown()` (pulldown_cmark + syntect highlighting)

### Scrolling

- Auto-scroll to bottom on each new token during streaming
- Scroll lock (`Ctrl-L` toggle) — when ON, streaming tokens don't auto-scroll
- Set via PgUp/PgDn, mouse scroll, Ctrl-Home/Ctrl-End
- `Scrollbar` widget on right edge when content exceeds visible height
- Scroll offset clamped to `content_height - 1`

### Input

- Bordered block with `theme.input_border` color, 2-col left padding inside border
- Prefix `>` styled with `theme.prompt_fg`
- Dynamic height: line_count + 3 (border + rows + info + border), min 4, max 13
- Disabled state shows `⏳ Waiting for response...` inline during streaming
- Cursor positioned at `area.x + 2 + cursor_pos` (after `> ` prompt) on first row
- Info bar on last row: "Ctrl-D exit · Shift-Enter newline · Ctrl-C cancel"
- History navigation via Up/Down arrows

### Keyboard

| Key | Action |
|-----|--------|
| Enter | Submit message |
| Shift-Enter | Newline |
| Esc / Ctrl-C | Cancel streaming / exit |
| Ctrl-D | Exit |
| Ctrl-L | Toggle scroll lock |
| Ctrl-Home | Scroll to top |
| Ctrl-End | Scroll to bottom (unlock) |
| PgUp/PgDn | Scroll page |
| Up/Down | Input history |
| Left/Right | Move cursor |
| Backspace/Delete | Delete char |
| Ctrl-U | Clear input |
| Ctrl-W | Delete word backward |
| Mouse scroll | Scroll ±3 lines |

### Theme fields (`theme.rs`)

| Field | Role |
|-------|------|
| `gutter_user` | Gutter color for user messages |
| `gutter_assistant` | Gutter color for assistant messages |
| `gutter_system` | Gutter color for system messages |
| `hero_border` | Box-drawing border color |
| `hero_title` | Title text color |
| `tip_fg` | Help tip text color |
| `status_fg` | Info label text color |
| `prompt_fg` | `>` prompt prefix color |
| `input_border` | Input widget border color |
| `assistant_fg` | Input text color |
| `code_bg` | Code block background fill |

Three presets: `dark()`, `light()`, `no_color()`. Extensible to custom palettes.

## Files

- `tools/cli/src/chat/tui.rs` — Layout, rendering, event loop, key handling
- `tools/cli/src/chat/theme.rs` — Color configuration
- `tools/cli/src/chat/input.rs` — Text buffer, cursor, history
- `tools/cli/src/chat/markdown.rs` — Markdown → ratatui Lines rendering

## Layout padding

Entire UI has 1-cell margin (`Layout::margin(1)`) so content doesn't touch terminal edges. Chat output adds a blank line at top and 2 spaces after gutter for visual breathing room.

Input widget has its own border + 2-col left padding inside for visual separation.

## Animations & transitions

### Non-blocking streaming

`ChatTui` stores `stream_rx: Option<mpsc::Receiver<StreamEvent>>`. Each frame, `poll_stream()` calls `try_recv()` on the receiver — non-blocking. Event loop continues running during streaming, enabling responsive UI, spinner animation, and key handling (Ctrl-C to cancel).

### Hero → Chat transition

When first message submitted, `transition_frame = 8`. For the next 8 frames (~400ms), a centered overlay shows `⠋ starting...` with animated spinner. After transition, chat session appears with user message and thinking indicator.

### Smooth scroll

`scroll_target` tracks desired scroll position. Each frame, `scroll_offset` interpolates toward `scroll_target` with deceleration: step = `max(1, diff / 4)` when diff > 8, else 1. Produces smooth glide instead of instant jumps.

PvE: PgUp/PgDn, mouse scroll, Ctrl-Home/End all set `scroll_target` for smooth animation.

### Streaming cursor blink

During active streaming, a block cursor `▊` blinks at the end of the partial content. Uses `frame_count % 6 < 3` for ~8Hz blink (300ms on/off). Visible only when streaming has produced at least one token.

### Thinking loader

When streaming but `partial_assistant_content` is still empty, shows `⠋ thinking...` with cycling spinner. Once first token arrives, transitions to inline streaming display.

## Future

- Slash commands (`/help`, `/clear`, `/model`, `/export`)
- Code block collapsing
- Custom theme via `.kode/config.toml`
- Token usage display in hero
- Index status in hero (symbol count, last scan time)
- Multi-line cursor positioning
- Search within conversation
