//! CLI binary: thin composition root for the `kode` command-line tool.
//!
//! # Philosophy
//!
//! - The binary is thin: all business logic lives in crates.
//! - The binary owns CLI parsing, dependency wiring, and command dispatch.
//! - The binary does not implement business logic.
//!
//! # Extension Points
//!
//! New subcommands are added by:
//! 1. Adding a variant to [`Commands`].
//! 2. Implementing the dispatch logic in `main`.
//!
//! The [`Cli`] struct defines global options shared across all subcommands.

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "kode",
    version,
    propagate_version = true,
    about = "Evidence-first code intelligence for humans and AI agents\n\nBuilds a deterministic understanding of your repository and answers\nquestions using live source code with path:line citations.",
    help_template = "\
{name} \u{2014} {about}

USAGE:
    {usage}

{all-args}
",
    subcommand_help_heading = "COMMANDS",
    subcommand_value_name = "COMMAND"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    #[arg(
        global = true,
        short = 'C',
        long = "repo",
        value_name = "PATH",
        help = "Repository to operate on"
    )]
    repo: Option<String>,

    #[arg(
        global = true,
        short = 'v',
        long = "verbose",
        action = clap::ArgAction::Count,
        help = "Increase logging verbosity"
    )]
    verbose: u8,

    #[arg(
        global = true,
        short = 'q',
        long = "quiet",
        help = "Suppress non-essential output"
    )]
    quiet: bool,

    #[arg(global = true, long = "json", help = "Machine-readable output")]
    json: bool,

    #[arg(global = true, long = "no-color", help = "Disable colored output")]
    no_color: bool,
}

/// All supported CLI commands.
///
/// Each variant corresponds to a top-level subcommand. New commands
/// should be added here and dispatched in [`main`].
#[derive(Subcommand)]
enum Commands {
    #[command(
        about = "Discover and index a repository",
        long_about = "Scans the repository, parses supported source files, and updates the local knowledge graph. Only changed files are reprocessed when possible."
    )]
    Scan {
        #[arg(help = "Repository to scan (default: current directory)")]
        path: Option<String>,

        #[arg(
            long = "full",
            help = "Ignore incremental state and rebuild from scratch"
        )]
        full: bool,

        #[arg(long = "watch", help = "Monitor repository for filesystem changes")]
        watch: bool,

        #[arg(long = "threads", value_name = "N", help = "Worker threads")]
        threads: Option<u32>,
    },

    #[command(
        about = "Show repository indexing status",
        long_about = "Displays repository metadata and the current state of the local index."
    )]
    Status,

    #[command(
        about = "Explore indexed repository files",
        long_about = "Lists files known to the knowledge graph."
    )]
    Files {
        #[arg(long = "language", value_name = "LANG", help = "Filter by language")]
        language: Option<String>,

        #[arg(long = "modified", help = "Show only modified files")]
        modified: bool,

        #[arg(long = "ignored", help = "Show ignored files")]
        ignored: bool,
    },

    #[command(
        about = "Explore extracted symbols",
        long_about = "Lists functions, types, traits, modules, classes, and other language symbols extracted from indexed source files."
    )]
    Symbols {
        #[arg(long = "language", value_name = "LANG", help = "Filter by language")]
        language: Option<String>,
    },

    #[command(
        about = "Query repository knowledge",
        long_about = "Execute deterministic queries against the knowledge graph."
    )]
    Query {
        #[arg(help = "Query string")]
        query: String,
    },

    #[command(
        about = "Interactive repository assistant",
        long_about = "Starts an interactive session that answers repository questions using live source code and evidence-backed citations."
    )]
    Chat {
        #[arg(
            short = 'm',
            long = "message",
            value_name = "TEXT",
            help = "Ask one question and exit"
        )]
        message: Option<String>,
    },

    #[command(about = "Manage the local repository cache")]
    Cache {
        #[command(subcommand)]
        command: CacheCommands,
    },

    #[command(about = "Configure kode")]
    Config {
        #[command(subcommand)]
        command: ConfigCommands,
    },

    #[command(about = "Run or manage the MCP server")]
    Mcp {
        #[command(subcommand)]
        command: McpCommands,
    },
}

/// Cache management subcommands.
#[derive(Subcommand)]
enum CacheCommands {
    #[command(about = "Show cache information")]
    Status,
    #[command(about = "Remove cached repository data")]
    Clear,
}

/// Configuration management subcommands.
#[derive(Subcommand)]
enum ConfigCommands {
    #[command(about = "Create configuration")]
    Init,
    #[command(about = "Read a configuration value")]
    Get {
        #[arg(help = "Configuration key")]
        key: String,
    },
    #[command(about = "Update a configuration value")]
    Set {
        #[arg(help = "Configuration key")]
        key: String,
        #[arg(help = "Configuration value")]
        value: String,
    },
}

/// MCP (Model Context Protocol) server subcommands.
#[derive(Subcommand)]
enum McpCommands {
    #[command(about = "Start the MCP server")]
    Serve {
        #[arg(help = "Repository path")]
        path: String,
    },
}

/// Generates a placeholder message for unimplemented commands.
fn placeholder_message(command: &str) -> String {
    format!("{} has not been implemented yet.", command)
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Scan { .. } => {
            println!("{}", placeholder_message("Repository scanning"));
        }
        Commands::Status => {
            println!("{}", placeholder_message("Repository status"));
        }
        Commands::Files { .. } => {
            println!("{}", placeholder_message("File exploration"));
        }
        Commands::Symbols { .. } => {
            println!("{}", placeholder_message("Symbol exploration"));
        }
        Commands::Query { .. } => {
            println!("{}", placeholder_message("Querying"));
        }
        Commands::Chat { .. } => {
            println!("{}", placeholder_message("Chat"));
        }
        Commands::Cache { command } => match command {
            CacheCommands::Status => {
                println!("{}", placeholder_message("Cache status"));
            }
            CacheCommands::Clear => {
                println!("{}", placeholder_message("Cache clearing"));
            }
        },
        Commands::Config { command } => match command {
            ConfigCommands::Init => {
                println!("{}", placeholder_message("Configuration initialization"));
            }
            ConfigCommands::Get { .. } => {
                println!("{}", placeholder_message("Configuration get"));
            }
            ConfigCommands::Set { .. } => {
                println!("{}", placeholder_message("Configuration set"));
            }
        },
        Commands::Mcp { command } => match command {
            McpCommands::Serve { .. } => {
                println!("{}", placeholder_message("MCP server"));
            }
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn verify_cli() {
        Cli::command().debug_assert();
    }

    #[test]
    fn test_scan_command() {
        let cli = Cli::parse_from(["kode", "scan"]);
        assert!(matches!(cli.command, Commands::Scan { .. }));
    }

    #[test]
    fn test_scan_with_path() {
        let cli = Cli::parse_from(["kode", "scan", "/some/path"]);
        match &cli.command {
            Commands::Scan { path, .. } => {
                assert_eq!(path.as_ref().unwrap(), "/some/path");
            }
            _ => panic!("Expected Scan command"),
        }
    }

    #[test]
    fn test_scan_with_options() {
        let cli = Cli::parse_from(["kode", "scan", "--full", "--watch", "--threads", "4"]);
        match &cli.command {
            Commands::Scan {
                full,
                watch,
                threads,
                ..
            } => {
                assert!(*full);
                assert!(*watch);
                assert_eq!(*threads, Some(4));
            }
            _ => panic!("Expected Scan command"),
        }
    }

    #[test]
    fn test_status_command() {
        let cli = Cli::parse_from(["kode", "status"]);
        assert!(matches!(cli.command, Commands::Status));
    }

    #[test]
    fn test_files_command() {
        let cli = Cli::parse_from(["kode", "files"]);
        assert!(matches!(cli.command, Commands::Files { .. }));
    }

    #[test]
    fn test_files_with_options() {
        let cli = Cli::parse_from([
            "kode",
            "files",
            "--language",
            "rust",
            "--modified",
            "--ignored",
        ]);
        match &cli.command {
            Commands::Files {
                language,
                modified,
                ignored,
            } => {
                assert_eq!(language.as_ref().unwrap(), "rust");
                assert!(*modified);
                assert!(*ignored);
            }
            _ => panic!("Expected Files command"),
        }
    }

    #[test]
    fn test_symbols_command() {
        let cli = Cli::parse_from(["kode", "symbols"]);
        assert!(matches!(cli.command, Commands::Symbols { .. }));
    }

    #[test]
    fn test_symbols_with_language() {
        let cli = Cli::parse_from(["kode", "symbols", "--language", "python"]);
        match &cli.command {
            Commands::Symbols { language } => {
                assert_eq!(language.as_ref().unwrap(), "python");
            }
            _ => panic!("Expected Symbols command"),
        }
    }

    #[test]
    fn test_query_command() {
        let cli = Cli::parse_from(["kode", "query", "test query"]);
        match &cli.command {
            Commands::Query { query } => {
                assert_eq!(query, "test query");
            }
            _ => panic!("Expected Query command"),
        }
    }

    #[test]
    fn test_chat_command() {
        let cli = Cli::parse_from(["kode", "chat"]);
        assert!(matches!(cli.command, Commands::Chat { .. }));
    }

    #[test]
    fn test_chat_with_message() {
        let cli = Cli::parse_from(["kode", "chat", "-m", "hello"]);
        match &cli.command {
            Commands::Chat { message } => {
                assert_eq!(message.as_ref().unwrap(), "hello");
            }
            _ => panic!("Expected Chat command"),
        }
    }

    #[test]
    fn test_chat_with_long_message() {
        let cli = Cli::parse_from(["kode", "chat", "--message", "hello"]);
        match &cli.command {
            Commands::Chat { message } => {
                assert_eq!(message.as_ref().unwrap(), "hello");
            }
            _ => panic!("Expected Chat command"),
        }
    }

    #[test]
    fn test_cache_status() {
        let cli = Cli::parse_from(["kode", "cache", "status"]);
        match &cli.command {
            Commands::Cache { command } => {
                assert!(matches!(command, CacheCommands::Status));
            }
            _ => panic!("Expected Cache command"),
        }
    }

    #[test]
    fn test_cache_clear() {
        let cli = Cli::parse_from(["kode", "cache", "clear"]);
        match &cli.command {
            Commands::Cache { command } => {
                assert!(matches!(command, CacheCommands::Clear));
            }
            _ => panic!("Expected Cache command"),
        }
    }

    #[test]
    fn test_config_init() {
        let cli = Cli::parse_from(["kode", "config", "init"]);
        match &cli.command {
            Commands::Config { command } => {
                assert!(matches!(command, ConfigCommands::Init));
            }
            _ => panic!("Expected Config command"),
        }
    }

    #[test]
    fn test_config_get() {
        let cli = Cli::parse_from(["kode", "config", "get", "some.key"]);
        match &cli.command {
            Commands::Config { command } => match command {
                ConfigCommands::Get { key } => {
                    assert_eq!(key, "some.key");
                }
                _ => panic!("Expected Config Get command"),
            },
            _ => panic!("Expected Config command"),
        }
    }

    #[test]
    fn test_config_set() {
        let cli = Cli::parse_from(["kode", "config", "set", "key", "value"]);
        match &cli.command {
            Commands::Config { command } => match command {
                ConfigCommands::Set { key, value } => {
                    assert_eq!(key, "key");
                    assert_eq!(value, "value");
                }
                _ => panic!("Expected Config Set command"),
            },
            _ => panic!("Expected Config command"),
        }
    }

    #[test]
    fn test_mcp_serve() {
        let cli = Cli::parse_from(["kode", "mcp", "serve", "."]);
        match &cli.command {
            Commands::Mcp { command } => {
                let McpCommands::Serve { path } = command;
                assert_eq!(path, ".");
            }
            _ => panic!("Expected Mcp command"),
        }
    }

    #[test]
    fn test_global_repo() {
        let cli = Cli::parse_from(["kode", "--repo", "/path", "status"]);
        assert_eq!(cli.repo.as_ref().unwrap(), "/path");
    }

    #[test]
    fn test_global_repo_short() {
        let cli = Cli::parse_from(["kode", "-C", "/path", "status"]);
        assert_eq!(cli.repo.as_ref().unwrap(), "/path");
    }

    #[test]
    fn test_global_verbose() {
        let cli = Cli::parse_from(["kode", "-v", "status"]);
        assert_eq!(cli.verbose, 1);
    }

    #[test]
    fn test_global_verbose_long() {
        let cli = Cli::parse_from(["kode", "--verbose", "status"]);
        assert_eq!(cli.verbose, 1);
    }

    #[test]
    fn test_global_quiet() {
        let cli = Cli::parse_from(["kode", "-q", "status"]);
        assert!(cli.quiet);
    }

    #[test]
    fn test_global_quiet_long() {
        let cli = Cli::parse_from(["kode", "--quiet", "status"]);
        assert!(cli.quiet);
    }

    #[test]
    fn test_global_json() {
        let cli = Cli::parse_from(["kode", "--json", "status"]);
        assert!(cli.json);
    }

    #[test]
    fn test_global_no_color() {
        let cli = Cli::parse_from(["kode", "--no-color", "status"]);
        assert!(cli.no_color);
    }

    #[test]
    fn test_global_options_after_command() {
        let cli = Cli::parse_from(["kode", "status", "--verbose"]);
        assert_eq!(cli.verbose, 1);
    }

    #[test]
    fn test_global_options_before_command() {
        let cli = Cli::parse_from(["kode", "--json", "--no-color", "status"]);
        assert!(cli.json);
        assert!(cli.no_color);
    }

    #[test]
    fn test_multiple_verbose_flags() {
        let cli = Cli::parse_from(["kode", "-vvv", "status"]);
        assert_eq!(cli.verbose, 3);
    }

    #[test]
    fn test_invalid_command() {
        let result = Cli::try_parse_from(["kode", "unknown"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_cache_subcommand() {
        let result = Cli::try_parse_from(["kode", "cache", "unknown"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_config_subcommand() {
        let result = Cli::try_parse_from(["kode", "config", "unknown"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_mcp_subcommand() {
        let result = Cli::try_parse_from(["kode", "mcp", "unknown"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_missing_query_argument() {
        let result = Cli::try_parse_from(["kode", "query"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_missing_config_get_key() {
        let result = Cli::try_parse_from(["kode", "config", "get"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_missing_config_set_value() {
        let result = Cli::try_parse_from(["kode", "config", "set", "key"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_missing_mcp_serve_path() {
        let result = Cli::try_parse_from(["kode", "mcp", "serve"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_placeholder_message_format() {
        let msg = placeholder_message("Repository scanning");
        assert_eq!(msg, "Repository scanning has not been implemented yet.");
    }

    #[test]
    fn test_root_help_contains_all_commands() {
        let mut cmd = Cli::command();
        let help = cmd.render_help().to_string();
        assert!(help.contains("scan"));
        assert!(help.contains("status"));
        assert!(help.contains("files"));
        assert!(help.contains("symbols"));
        assert!(help.contains("query"));
        assert!(help.contains("chat"));
        assert!(help.contains("cache"));
        assert!(help.contains("config"));
        assert!(help.contains("mcp"));
    }

    #[test]
    fn test_root_help_contains_global_options() {
        let mut cmd = Cli::command();
        let help = cmd.render_help().to_string();
        assert!(help.contains("--repo"));
        assert!(help.contains("-C"));
        assert!(help.contains("--verbose"));
        assert!(help.contains("--quiet"));
        assert!(help.contains("--json"));
        assert!(help.contains("--no-color"));
    }

    #[test]
    fn test_root_help_contains_description() {
        let mut cmd = Cli::command();
        let help = cmd.render_help().to_string();
        assert!(help.contains("Evidence-first code intelligence"));
        assert!(help.contains("deterministic understanding"));
        assert!(help.contains("path:line citations"));
    }

    #[test]
    fn test_scan_help_contains_options() {
        let mut cmd = Cli::command();
        let help = cmd.render_help().to_string();
        assert!(help.contains("--full") || help.contains("scan"));
        let sub = cmd.find_subcommand_mut("scan").unwrap();
        let help = sub.render_help().to_string();
        assert!(help.contains("--full"));
        assert!(help.contains("--watch"));
        assert!(help.contains("--threads"));
    }

    #[test]
    fn test_files_help_contains_options() {
        let mut cmd = Cli::command();
        let sub = cmd.find_subcommand_mut("files").unwrap();
        let help = sub.render_help().to_string();
        assert!(help.contains("--language"));
        assert!(help.contains("--modified"));
        assert!(help.contains("--ignored"));
    }

    #[test]
    fn test_chat_help_contains_options() {
        let mut cmd = Cli::command();
        let sub = cmd.find_subcommand_mut("chat").unwrap();
        let help = sub.render_help().to_string();
        assert!(help.contains("--message"));
    }

    #[test]
    fn test_help_contains_help_and_version() {
        let mut cmd = Cli::command();
        let help = cmd.render_help().to_string();
        assert!(help.contains("--help"));
        assert!(help.contains("--version"));
    }
}
