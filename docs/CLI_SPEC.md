# CLI Specification

## Root

```text
kode — Evidence-first code intelligence for humans and AI agents

Builds a deterministic understanding of your repository and answers
questions using live source code with path:line citations.

USAGE:
    kode [OPTIONS] <COMMAND>

COMMANDS:
  scan     Discover and index a repository
  status   Show repository indexing status
  files    Explore indexed repository files
  symbols  Explore extracted symbols
  query    Query repository knowledge
  chat     Interactive repository assistant
  cache    Manage the local repository cache
  config   Configure kode
  mcp      Run or manage the MCP server
  help     Print this message or the help of the given subcommand(s)

Options:
  -C, --repo <PATH>      Repository to operate on
  -v, --verbose...       Increase logging verbosity
  -q, --quiet            Suppress non-essential output
      --json             Machine-readable output
      --no-color         Disable colored output
      --log-file <PATH>  Write logs to file (default: .kode/logs/kode.log)
  -h, --help             Print help
  -V, --version          Print version
```

---

## scan

```text
Scans the repository, parses supported source files, and updates the local
knowledge graph. Only changed files are reprocessed when possible.

USAGE:
    kode scan [OPTIONS] [PATH]

ARGUMENTS:
    [PATH]                Repository to scan (default: current directory)

OPTIONS:
        --full            Ignore incremental state and rebuild from scratch
        --watch           Monitor repository for filesystem changes
        --threads <N>     Worker threads
    -C, --repo <PATH>     Repository to operate on
    -v, --verbose...      Increase logging verbosity
    -q, --quiet           Suppress non-essential output
        --json            Machine-readable output
        --no-color        Disable colored output
    -h, --help            Print help
    -V, --version         Print version
```

---

## status

```text
Displays repository metadata and the current state of the local index.

USAGE:
    kode status [OPTIONS]

Displays:
    • Repository root
    • Last scan
    • Indexed files
    • Languages
    • Graph statistics
    • Cache location
```

---

## files

```text
Lists files known to the knowledge graph.

USAGE:
    kode files [OPTIONS]

OPTIONS:
        --language <LANG>  Filter by language
        --modified         Show only modified files
        --ignored          Show ignored files
```

---

## symbols

```text
Lists functions, types, traits, modules, classes, and other language
symbols extracted from indexed source files.

USAGE:
    kode symbols [OPTIONS]

OPTIONS:
        --language <LANG>  Filter by language
```

---

## query

```text
Execute deterministic queries against the knowledge graph.

USAGE:
    kode query [OPTIONS] <QUERY>

EXAMPLES:
    kode query "functions named parse"
    kode query "who calls scan"
```

---

## chat

```text
Starts an interactive session that answers repository questions using
live source code and evidence-backed citations.

USAGE:
    kode chat [OPTIONS]

OPTIONS:
    -m, --message <TEXT>    Ask one question and exit
```

---

## cache

```text
Manage the local repository cache

USAGE:
    kode cache [OPTIONS] <COMMAND>

COMMANDS:
    status      Show cache information
    clear       Remove cached repository data
    help        Print this message or the help of the given subcommand(s)
```

---

## config

```text
Configure kode

USAGE:
    kode config [OPTIONS] <COMMAND>

COMMANDS:
    init        Create configuration
    get         Read a configuration value
    set         Update a configuration value
    help        Print this message or the help of the given subcommand(s)
```

---

## mcp

```text
Run or manage the MCP server

USAGE:
    kode mcp [OPTIONS] <COMMAND>

COMMANDS:
    serve       Start the MCP server
    help        Print this message or the help of the given subcommand(s)
```
