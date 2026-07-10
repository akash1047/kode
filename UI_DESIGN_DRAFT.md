# AI Research Agent TUI

## Vision

A fullscreen terminal application built with Ratatui that feels like a cross between a Unix shell, DeepWiki, and a knowledge graph explorer.

The application is not a coding assistant. Its purpose is to help users understand software systems by exploring repositories, documentation, APIs, architecture, and relationships between concepts.

Design principles:

- Minimal
- Monochrome
- Information dense
- Keyboard-first
- Fast
- Calm (very little animation)

---

# Primary Use Cases

- Explore GitHub repositories
- Read and summarize documentation
- Understand project architecture
- Navigate relationships between files and concepts
- Search documentation semantically
- Build persistent knowledge sessions

---

# Layout

```
┌────────────────────────────────────────────────────────────────────────────┐
│ > atlas.sh █     Explore   Sources   Graph   Sessions   Settings           │
├────────────────────────────────────────────────────────────────────────────┤
│                                                                            │
│  Understand_                     Live Knowledge Graph                      │
│  software systems                (ASCII visualization)                     │
│                                                                            │
│  Description                     Capabilities                             │
│                                                                            │
│  Primary Action                  Agent Status                              │
│                                                                            │
├────────────────────────────────────────────────────────────────────────────┤
│ Sources │ Concepts │ Sessions │ Activity                                   │
├────────────────────────────────────────────────────────────────────────────┤
│ Status line                                                         Ready  │
└────────────────────────────────────────────────────────────────────────────┘
```

---

# Navigation

## Top Navigation

- Explore
- Sources
- Graph
- Sessions
- Settings

Keyboard shortcuts:

- 1-5
- Left / Right
- Tab

---

# Home

Purpose:

Introduce the agent and provide a launch point for research.

## Hero

```
Understand_
software systems
```

Subtitle:

```
Explore repositories, documentation, APIs and architecture.
```

---

## Capability Panel

```
Repository Analysis

Documentation Search

Architecture Mapping

Knowledge Graph

Persistent Memory
```

---

## Primary Action

```
▶ Start Research
```

Opens command mode.

---

## Live Graph

Instead of artwork, display a dynamic ASCII graph.

Example:

```
            Tokio
           /     \
      Runtime   Net
        |         |
      Mio ----- Hyper
           \
          Tower
```

Future versions can animate node discovery.

---

# Explore

Acts as the primary workspace.

Layout:

```
Search

────────────────────────────────

Results

Summary

Related Concepts

Sources
```

Workflow:

1. Ask question
2. Agent researches
3. Streams findings
4. Shows citations
5. Builds graph

---

# Sources

Repository browser.

```
repo/

├── README.md
├── docs/
├── src/
│   ├── parser
│   ├── api
│   └── model
└── tests/
```

Selecting a file displays:

- Summary
- Purpose
- Key entities
- Related files
- Outgoing references

---

# Knowledge Graph

Interactive graph explorer.

Shows:

- Modules
- Classes
- APIs
- Documents
- Concepts

Example:

```
Repository

├── Parser

│   ├── Lexer

│   └── AST

└── Runtime

    ├── Scheduler

    └── Tasks
```

Navigation:

- Arrow keys
- Enter
- Zoom
- Center

---

# Sessions

Displays previous explorations.

```
Today

Rust Async Runtime

Axum Middleware

Yesterday

Raft

Kubernetes Scheduler
```

Selecting a session restores:

- Graph
- Search history
- Memory
- Sources

---

# Settings

Configuration for:

- AI provider
- Model
- Theme
- Indexing
- Cache
- Memory
- Keyboard shortcuts

---

# Footer

Displays runtime information.

Example:

```
indexed 12 repositories

324 documents

graph 218 nodes

GPT-5.5

ready
```

---

# Visual Style

## Colors

Background

Black

Primary

White

Muted

Gray

Accent

Cyan

Selection

Reverse video

---

## Typography

Everything is monospace.

Large headings use FIGlet or tui-big-text.

Borders are rounded.

Whitespace is generous.

---

# Components

## Header

Contains:

- Logo
- Navigation
- Current model
- Agent state

---

## Hero

Contains:

- Large heading
- Description
- Action button

---

## Knowledge Graph

ASCII visualization of discovered concepts.

Future:

- Expandable
- Collapsible
- Interactive

---

## Capability List

Shows current abilities.

```
Explore

Connect

Explain

Search

Remember
```

---

## Status Line

Examples:

```
Idle
```

```
Searching repository...
```

```
Reading documentation...
```

```
Building knowledge graph...
```

```
Finished
```

---

# Interaction Model

Everything is keyboard driven.

| Key | Action |
|------|--------|
| / | Search |
| Tab | Next panel |
| Enter | Open |
| Esc | Back |
| g | Graph |
| s | Sources |
| e | Explore |
| m | Sessions |
| ? | Help |
| q | Quit |

---

# Future Ideas

## Streaming

Agent responses stream into the interface.

---

## Live Graph

Nodes appear as knowledge is discovered.

---

## Multi-Repository Research

Compare multiple repositories simultaneously.

---

## Document Collections

Research across:

- GitHub
- Markdown
- PDFs
- Websites

---

## Semantic Memory

Store previous discoveries and reuse them across sessions.

---

## Export

Export research as:

- Markdown
- JSON
- HTML
- PDF

---

# Overall Feeling

The interface should resemble a Unix command-line tool built for exploration rather than conversation.

Users should feel like they are navigating a living map of technical knowledge instead of chatting with an AI. Every screen should emphasize structure, relationships, and discoverability while maintaining a clean, monochrome, terminal-native aesthetic.
