use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    name = "kode",
    version,
    about = "Codebase intelligence for humans and AI agents",
    long_about = "Local evidence snapshot of any repo, plus a chat REPL where an LLM \
                  answers questions using a cached, parsed view of your project (no training-data guessing)."
)]
/// Top-level CLI arguments parsed by clap.
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,

    /// Project directory (used when no subcommand is given). Defaults to `.`.
    #[arg(default_value = ".", global = false)]
    pub path: PathBuf,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Print project header only.
    Info {
        /// Project directory.
        #[arg(default_value = ".")]
        path: PathBuf,
    },

    /// Print project header + per-file char counts (gitignore-aware walk).
    Scan {
        /// Project directory.
        #[arg(default_value = ".")]
        path: PathBuf,
    },

    /// Open interactive chat REPL backed by the cache + project tools.
    Chat {
        /// Project directory.
        #[arg(default_value = ".")]
        path: PathBuf,

        /// One-shot question. With this flag, kode prints the answer to stdout and exits — no REPL.
        #[arg(short = 'm', long)]
        message: Option<String>,

        /// Override the model for this invocation (otherwise reads $KODE_MODEL or falls back to the default).
        #[arg(long)]
        model: Option<String>,
    },

    /// Build, inspect, and clear the project cache (files, symbols, manifests).
    Cache {
        #[command(subcommand)]
        op: CacheOp,
    },

    /// Expose the kode index to other agents over the Model Context Protocol.
    Mcp {
        #[command(subcommand)]
        op: McpOp,
    },

    /// Manage user configuration at ~/.config/kode/config.toml.
    Config {
        #[command(subcommand)]
        op: ConfigOp,
    },
}

#[derive(Subcommand, Debug)]
pub enum ConfigOp {
    /// Write a default config template (skips if file already exists).
    Init,

    /// Print the config file contents.
    Show,

    /// Set a key in the config (dot-separated for nested keys, e.g. chat.model).
    Set {
        /// Dot-separated key (e.g. chat.model, summarize.max_input_chars).
        key: String,
        /// Value to set.
        value: String,
    },

    /// Print the value of a single config key, nothing else.
    Get {
        /// Dot-separated key (e.g. chat.api_key).
        key: String,
    },
}

/// Editor/tool preset for `mcp init`.
#[derive(clap::ValueEnum, Debug, Clone, Copy, Default)]
pub enum McpPreset {
    /// Claude Code — writes .mcp.json
    #[default]
    Claude,
    /// VS Code — writes .vscode/mcp.json
    Vscode,
    /// Cursor — writes .cursor/mcp.json
    Cursor,
    /// Zed — writes .zed/settings.json
    Zed,
}

#[derive(Subcommand, Debug)]
pub enum McpOp {
    /// Write an MCP config file for the chosen editor preset (skips if file already exists).
    Init {
        /// Project directory to write the config into.
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Editor preset: claude (default), vscode, cursor, zed.
        #[arg(long, default_value = "claude")]
        preset: McpPreset,
    },

    /// Run an MCP server exposing an `ask` tool backed by the kode agent.
    Serve {
        /// Project directory to expose.
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Transport: stdio (default) or http.
        #[arg(long, default_value = "stdio")]
        transport: McpTransport,

        /// Port for the http transport.
        #[arg(long, default_value_t = 8765)]
        port: u16,
    },
}

/// Transport layer for the MCP server.
#[derive(clap::ValueEnum, Debug, Clone, Copy)]
pub enum McpTransport {
    Stdio,
    Http,
}

#[derive(Subcommand, Debug)]
pub enum CacheOp {
    /// Build/refresh the cache for a repo without opening chat.
    Build {
        /// Project directory.
        #[arg(default_value = ".")]
        path: PathBuf,
    },

    /// Show cache stats (db path, file/manifest/symbol counts, last scan time).
    Status {
        /// Project directory.
        #[arg(default_value = ".")]
        path: PathBuf,
    },

    /// Delete the cache for a repo.
    Clear {
        /// Project directory.
        #[arg(default_value = ".")]
        path: PathBuf,
    },

    /// Show a human-friendly breakdown of cache contents (files by language, symbols by kind, manifests, run-configs, summaries).
    Inspect {
        /// Project directory.
        #[arg(default_value = ".")]
        path: PathBuf,
    },
}

/// Parse CLI arguments from `std::env::args_os`.
pub fn parse() -> Cli {
    Cli::parse()
}
