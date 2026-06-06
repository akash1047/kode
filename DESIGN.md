# kode — Design

Codebase evidence service. Browsable docs for humans, MCP server for agent callers. This document defines what kode is, what it is not, and the invariants that keep it honest.

## 1. What kode is

A read-only **evidence service** for codebases.

A caller (human or agent) asks a question. kode reads the project, returns the relevant source slice with citations. The caller decides what to do with it.

```
Question  ──►  kode  ──►  Evidence { answer, spans, files }  ──►  Caller decides
```

The caller carries the brain. kode carries the source.

## 2. What kode is NOT

Naming these keeps the surface small and the contract clear.

- **Not a code generator.** kode answers about code, does not write it.
- **Not an advisor.** No suggestions, no "did you mean", no follow-up prompts, no intent inference. Literal query in, literal evidence out.
- **Not a project manager.** No TODOs, no cross-session memory of past asks.
- **Not a build/test runner.** Read-only on source.
- **Not a chat partner.** Single-turn `ask`. No multi-turn session state. Refinement = caller re-asks.
- **Not a paraphraser.** kode answers what was asked, not what kode thinks the caller meant.

Each "not" is load-bearing. Drift on any of these brings back the over-reach that caused stale-answer bugs in earlier iterations.

## 3. Contract

### 3.1 Inputs

```
ask(question: String) -> Evidence
```

One field. No optional context hints, no intent flags, no session ID. If the caller wants kode to focus, the caller writes a better question.

### 3.2 Output

```
Evidence {
    answer: String,                      // grounded prose reply
    spans:  Vec<Span>,                   // quoted evidence, what kode used
    files:  Vec<PathBuf>,                // touched, not quoted (negative-signal context)
}

Span {
    path:       PathBuf,
    line_start: u32,
    line_end:   u32,
    snippet:    String,
}
```

- `spans` = files kode read AND quoted from. Caller can verify exact bytes.
- `files` = files kode read but did not quote. Tells the caller "I looked here, nothing load-bearing." Useful negative signal — caller skips re-asking about these.
- `answer` = prose grounded entirely in `spans`. Every load-bearing claim must trace to a span.

### 3.3 Statelessness

Each `ask` is independent. No conversation memory on kode's side. The caller refines by issuing another `ask` with a sharper question. kode re-runs scan-cache-verify-answer from scratch.

## 4. Internal flow

```
scan       → populate cache (index)
query      → find candidate evidence in cache
verify     → read source on disk for each candidate
answer     → compose reply citing only verified spans
```

### 4.1 Scan

On project open (or refresh), walk the file tree (gitignore-aware), extract symbols via tree-sitter (Rust, Python, TypeScript, JavaScript today), index into SQLite. Record `(path, line_start, line_end, kind, name, mtime)` per symbol.

Scan is for speed. Scan is **not** the source of truth.

### 4.2 Query

For a given question, kode picks candidate symbols / files via cache lookup. This narrows where to look. Cache hit returns `(path, line)` — pointers only.

### 4.3 Verify (load-bearing invariant)

For every candidate before quoting:

1. Compare cache row `mtime` against disk `mtime` for `path`.
2. If mismatch, invalidate the cache row and re-parse the file.
3. Open the file, read `line_start..line_end` (plus a small context window).
4. Use the **freshly read bytes** as the evidence span.

The cache only ever told kode *where to look*. The answer comes from the file read at query time. This makes the README-rot bug class structurally impossible — a stale cache row produces wrong coordinates, but the verify step reads what is actually at those coordinates now, and the grounding rule ("don't claim what you didn't see") prevents kode from quoting stale content.

### 4.4 Answer

Compose prose using only quoted spans. Cite each load-bearing claim by `path:line_start-line_end`. Append the structured spans + files list as the Evidence return value.

## 5. Cache discipline

Three rules.

1. **Cache indexes. Source answers.** Cache row contents are pointers (path, line, symbol name, mtime). They are never quoted in `answer`. Quoting from cache content = bug.
2. **Verify on every read.** mtime check is mandatory, not an optimization to skip on a hot path.
3. **No semantic pre-digests.** No "parsed README", no "manifest description", no "summary of src/foo.rs". These rot and tempt the agent to quote them. If the caller wants prose about a file, kode reads the file at query time and composes the prose then.

What the cache holds:

- File tree (gitignore-filtered paths + mtime).
- Symbol table (function / type / constant locations from tree-sitter).
- File hashes for change detection.

What the cache does **not** hold:

- Parsed manifest content (names, descriptions, dependencies as semantic strings).
- README excerpts.
- LLM-generated summaries.

## 6. Tool surface (MCP agent)

The MCP `ask` agent gets a minimal toolkit. Each tool returns only what is needed; nothing pre-digested.

| Tool | Returns | Notes |
|---|---|---|
| `list_files(glob?)` | Path list. Gitignore-filtered. Hard char cap. | No content. |
| `find_symbol(name)` | `[{path, line, kind}]` | Cache lookup. Locations only, no bodies. |
| `read_file(path, range?)` | Raw file bytes. Hard char cap per call. | The verify step. Truth source. |

That is the full set. Tools that aggregate, summarize, or pre-bake content are intentionally absent from the MCP surface.

The interactive `kode chat` REPL may keep richer tools (overview, semantic search) for human ergonomics — different audience, different surface, same binary.

## 7. System prompt (MCP agent)

Behavioral rules, not procedure. No tool ordering, no "call X first."

```
You are a codebase assistant for /path/to/project. You have tools to
explore source files.

Workflow:
1. list_files to see structure. find_symbol to locate definitions by name.
2. read_file on each candidate. Read at most once per file per session.
3. Skip files unrelated to the question. Stop when answered.

Rules:
- Never answer from filenames or symbol names alone — read the file first.
- Never claim a command, function, flag, or feature exists without seeing
  it in a file you have read this session. README content is not a source
  of truth — verify in code.
- Quote exact identifiers as they appear in the source.
- Cite path:line for each load-bearing claim.
- Be specific. No filler.
```

## 8. Budgets

Hard caps in the tool layer, surfaced to the agent.

- `list_files`: 40k chars, truncate with `... N more files omitted` footer.
- `read_file`: 80k chars per call, truncate with `... file continues, N bytes total`.
- `find_symbol`: 50 results max, sorted by relevance.
- Session: track total chars read, attach `[session: X / 200k chars used]` to subsequent tool responses as a soft signal.

Budgets force focus. They also make tool output predictable for the agent.

## 9. Smart-caller assumption

kode is mechanically rigorous; the caller is responsible for asking well.

Consequences:

- **No paraphrase.** "where is auth" returns evidence for auth, not for "the auth pattern in this repo."
- **No scope expansion.** Asked about X, return X. Adjacent files are the caller's next question.
- **No second-guessing.** Mismatch between caller's assumption and repo reality surfaces as missing evidence, not as advice. Caller re-asks with corrected premise.
- **Refinement = re-ask.** Statelessness is the protocol.

This is a deliberate bet. Vague query in, weak evidence out. The contract makes the rule visible so callers learn to refine.

## 10. Failure modes

### 10.1 No evidence found

Return:

```
Evidence {
    answer: "No matching code found in the project for: <restated query>.",
    spans:  [],
    files:  [],
}
```

Explicit and machine-checkable. Caller can branch on `spans.is_empty()`.

### 10.2 Cache stale

Verify step catches it. Cache row invalidated, file re-parsed, query retried internally. Never surfaces to the caller as an error.

### 10.3 File too large

`read_file` truncates with explicit footer. Agent sees the truncation marker and can call `read_file` again with a range to get more. Agent never sees a silent cutoff.

### 10.4 Agent hallucinates anyway

Sources footer is the audit trail. If the answer makes a claim and `spans` does not contain it, the answer is bluffing. Callers SHOULD validate load-bearing claims against `spans` before acting destructively.

## 11. Transports

Two MCP transports, same logic.

- **stdio (default).** Newline-delimited JSON-RPC 2.0 over stdin/stdout. For local agent integrations.
- **HTTP.** `POST /mcp` on `127.0.0.1:<port>`. For cross-process / cross-machine use.

Protocol version `2024-11-05`. Server identifies as `kode` at the version from `Cargo.toml`.

## 12. Non-goals (explicit)

To prevent drift, name what kode will not become:

- Multi-repo aggregation. One repo per server instance.
- Cross-session memory or learning. Each `ask` is independent.
- Embedding-based search. Lexical + symbol-table is the index; embeddings add cost + opacity without changing the source-verify invariant.
- Write tools (file edits, refactors, commits). Always read-only.
- Natural-language UI. Slash commands `/kode` and `/kode-ask` are caller-side conveniences, not kode features.

## 13. Principles, distilled

1. **Cache indexes. Source answers.**
2. **Read-verify-cite.** No claim without a source-read this turn.
3. **Stateless.** Refinement = re-ask, not session memory.
4. **Literal queries get literal evidence.** No paraphrase, no expansion, no advice.
5. **Smart caller, mechanical kode.**

These five lines are the design. Everything above is the consequence of taking them seriously.
