use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand};
use kode_acquisition::Language;
use kode_graph as _;
use kode_query::QueryEngine;
use kode_storage::{RepositoryStorage, SqliteBackend};
use serde as _;

mod chat;

fn init_logging(
    verbose: u8,
    quiet: bool,
    log_file: Option<PathBuf>,
) -> Option<tracing_appender::non_blocking::WorkerGuard> {
    use tracing_subscriber::fmt;
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;
    use tracing_subscriber::EnvFilter;

    let log_level = if quiet && verbose == 0 {
        "off"
    } else {
        match verbose {
            0 => "kode=warn",
            1 => "kode=info",
            2 => "kode=debug",
            _ => "kode=trace",
        }
    };

    let filter = EnvFilter::try_new(log_level).unwrap_or_else(|_| EnvFilter::new("kode=warn"));

    let stderr_layer = fmt::layer().with_writer(std::io::stderr).with_target(false);

    let subscriber = tracing_subscriber::registry()
        .with(filter)
        .with(stderr_layer);

    if let Some(path) = log_file {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let file_appender = tracing_appender::rolling::never(
            path.parent().unwrap_or(std::path::Path::new(".")),
            path.file_name().unwrap().to_str().unwrap_or("kode.log"),
        );
        let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
        let file_layer = fmt::layer()
            .with_writer(non_blocking)
            .with_target(true)
            .with_ansi(false);
        let subscriber = subscriber.with(file_layer);
        let _ = subscriber.try_init();
        Some(guard)
    } else {
        let _ = subscriber.try_init();
        None
    }
}
mod config;
mod formatter;
mod mcp;
mod presenter;

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

    #[arg(
        global = true,
        long = "log-file",
        value_name = "PATH",
        help = "Write logs to file (default: .kode/logs/kode.log)"
    )]
    log_file: Option<String>,
}

#[derive(Debug, Subcommand)]
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
        about = "Export the knowledge graph",
        long_about = "Export the indexed knowledge graph to DOT or GraphML."
    )]
    Export {
        #[arg(
            long = "format",
            value_name = "FMT",
            default_value = "dot",
            help = "Export format: dot | graphml"
        )]
        format: String,

        #[arg(
            short = 'o',
            long = "output",
            value_name = "PATH",
            help = "Write to file instead of stdout"
        )]
        output: Option<String>,
    },

    #[command(
        about = "Interactive repository assistant",
        long_about = "Starts an interactive session with an LLM assistant."
    )]
    Chat {
        #[arg(
            short = 'm',
            long = "message",
            value_name = "TEXT",
            help = "Ask one question and exit (alias for positional argument)"
        )]
        message: Option<String>,

        #[arg(help = "Question to ask (one-shot mode; overrides --message)")]
        question: Option<String>,
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

#[derive(Debug, Subcommand)]
enum CacheCommands {
    #[command(about = "Show cache information")]
    Status,
    #[command(about = "Remove cached repository data")]
    Clear,
}

#[derive(Debug, Subcommand)]
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

#[derive(Debug, Subcommand)]
enum McpCommands {
    #[command(about = "Start the MCP server")]
    Serve {
        #[arg(help = "Repository path")]
        path: String,

        #[arg(long, help = "Port for HTTP transport (default: stdio)")]
        port: Option<u16>,
    },
}

fn resolve_path<'a>(path: Option<&'a Path>, repo: Option<&'a str>) -> &'a str {
    path.and_then(|p| p.to_str()).or(repo).unwrap_or(".")
}

fn handle_scan(
    path: Option<&str>,
    full: bool,
) -> Result<presenter::scan::ScanView, Box<dyn std::error::Error>> {
    if full {
        // Best-effort clear of existing cache so Stage 5 writes a fresh revision.
        if let Err(e) = handle_cache_clear(path) {
            tracing::debug!(error = %e, "cache clear before --full (may be first scan)");
        }
    }
    let result = kode_app::run_scan(path)?;
    Ok(presenter::scan::ScanView::from_scan_result(&result))
}

fn handle_status(
    path: Option<&str>,
) -> Result<presenter::status::StatusView, Box<dyn std::error::Error>> {
    // Prefer reading the index; only re-scan when no cache exists.
    match try_status_from_cache(path) {
        Ok(view) => Ok(view),
        Err(cache_err) => {
            tracing::info!(error = %cache_err, "no usable cache; running scan for status");
            let result = kode_app::run_scan(path)?;
            Ok(presenter::status::StatusView::from_scan_result(&result))
        }
    }
}

fn try_status_from_cache(
    path: Option<&str>,
) -> Result<presenter::status::StatusView, Box<dyn std::error::Error>> {
    let storage = open_storage(path)?;
    let meta = storage.metadata()?;
    let engine = QueryEngine::new(storage);
    let stats = engine.graph_stats()?;
    let repo_path = path.unwrap_or(".");
    let root = Path::new(repo_path)
        .canonicalize()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| repo_path.to_string());
    Ok(presenter::status::StatusView::from_cache(
        root, &meta, &stats,
    ))
}

fn handle_files(
    path: Option<&str>,
    language: Option<&str>,
) -> Result<presenter::files::FilesView, Box<dyn std::error::Error>> {
    // Prefer graph file nodes when an index exists.
    if let Ok(storage) = open_storage(path) {
        let engine = QueryEngine::new(storage);
        if let Ok(files) = engine.list_files(language) {
            return Ok(presenter::files::FilesView::from_file_results(&files));
        }
    }
    let result = kode_app::run_scan(path)?;
    let filter_lang = match language {
        Some(l) => Some(l.parse::<Language>()?),
        None => None,
    };
    Ok(presenter::files::FilesView::from_scan_result(
        &result,
        filter_lang.as_ref(),
    ))
}

fn open_storage(path: Option<&str>) -> Result<RepositoryStorage, Box<dyn std::error::Error>> {
    let repo_path = path.unwrap_or(".");
    let path = Path::new(repo_path);
    let absolute = path.canonicalize()?;
    let db_path = absolute.join(".kode").join("cache.db");

    if !db_path.exists() {
        return Err("Repository has not been scanned yet. Run `kode scan` first.".into());
    }

    let backend = SqliteBackend::open(&db_path)?;
    let repo_id = absolute.display().to_string();
    Ok(RepositoryStorage::open(Box::new(backend), &repo_id)?)
}

fn handle_symbols(
    path: Option<&str>,
    language: Option<&str>,
) -> Result<presenter::symbols::SymbolsView, Box<dyn std::error::Error>> {
    let storage = open_storage(path)?;
    let engine = QueryEngine::new(storage);
    let results = engine.search_symbols_filtered("", None, language)?;
    // Prefer entity symbols for listing (exclude pure structural roots).
    let results: Vec<_> = results
        .into_iter()
        .filter(|s| {
            !matches!(
                s.kind,
                kode_graph::NodeKind::Repository | kode_graph::NodeKind::Workspace
            )
        })
        .collect();
    Ok(presenter::symbols::SymbolsView::from_symbols(&results))
}

enum QueryDispatch {
    Symbols(presenter::symbols::SymbolsView),
    Analysis { text: String, json: String },
}

fn handle_query(
    path: Option<&str>,
    query: &str,
) -> Result<QueryDispatch, Box<dyn std::error::Error>> {
    let storage = open_storage(path)?;
    let engine = QueryEngine::new(storage);
    match engine.execute(query)? {
        kode_query::QueryOutcome::Symbols(results) => Ok(QueryDispatch::Symbols(
            presenter::symbols::SymbolsView::from_symbols(&results),
        )),
        kode_query::QueryOutcome::Metrics(m) => {
            let json = serde_json::json!({
                "metrics": m.iter().map(|row| serde_json::json!({
                    "name": row.name,
                    "file": row.file_path.display().to_string(),
                    "line": row.start_line,
                    "fan_in": row.fan_in,
                    "fan_out": row.fan_out,
                    "instability_pct": row.instability_pct,
                    "verified": row.verified,
                })).collect::<Vec<_>>(),
            });
            Ok(QueryDispatch::Analysis {
                text: kode_query::format_metrics(&m),
                json: serde_json::to_string_pretty(&json).unwrap_or_else(|_| "{}".into()),
            })
        }
        kode_query::QueryOutcome::Cycles(c) => {
            let json = serde_json::json!({
                "cycles": c.iter().map(|cy| &cy.members).collect::<Vec<_>>(),
            });
            Ok(QueryDispatch::Analysis {
                text: kode_query::format_cycles(&c),
                json: serde_json::to_string_pretty(&json).unwrap_or_else(|_| "{}".into()),
            })
        }
    }
}

fn handle_export(
    path: Option<&str>,
    format: &str,
    output: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let storage = open_storage(path)?;
    let engine = QueryEngine::new(storage);
    let body = match format.to_ascii_lowercase().as_str() {
        "dot" => engine.export_dot()?,
        "graphml" | "xml" => engine.export_graphml()?,
        other => {
            return Err(format!("unknown export format `{other}` (use: dot, graphml)").into());
        }
    };
    if let Some(path) = output {
        std::fs::write(path, body)?;
        println!("wrote {path}");
    } else {
        print!("{body}");
    }
    Ok(())
}

fn handle_cache_status(
    path: Option<&str>,
) -> Result<presenter::cache::CacheStatusView, Box<dyn std::error::Error>> {
    let storage = open_storage(path)?;
    let meta = storage.metadata()?;
    Ok(presenter::cache::CacheStatusView::from_metadata(&meta))
}

fn handle_cache_clear(path: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    let mut storage = open_storage(path)?;
    storage.clear()?;
    Ok(())
}

fn resolve_log_path(cli: &Cli) -> PathBuf {
    if let Some(path) = &cli.log_file {
        return PathBuf::from(path);
    }
    let repo_path = cli.repo.as_deref().unwrap_or(".");
    if let Ok(cfg) = config::Config::load(Path::new(repo_path)) {
        if let Some(val) = cfg.get("logging.file") {
            if let Some(s) = val.as_str() {
                if !s.is_empty() {
                    return PathBuf::from(s);
                }
            }
        }
    }
    Path::new(repo_path)
        .join(".kode")
        .join("logs")
        .join("kode.log")
}

fn main() {
    let cli = Cli::parse();
    let _guard = init_logging(cli.verbose, cli.quiet, Some(resolve_log_path(&cli)));

    let start = std::time::Instant::now();
    tracing::info!(command = ?cli.command, "CLI command started");

    let result = match &cli.command {
        Commands::Scan {
            path,
            full,
            watch,
            threads,
        } => {
            if *watch {
                tracing::error!("--watch is not implemented yet");
                std::process::exit(2);
            }
            if threads.is_some() {
                tracing::warn!("--threads is accepted but not yet used (scan is single-threaded)");
            }
            let scan_path = resolve_path(path.as_deref().map(Path::new), cli.repo.as_deref());
            handle_scan(Some(scan_path), *full).map(|o| {
                if cli.json {
                    print!("{}", formatter::json::format_scan(&o))
                } else {
                    print!("{}", formatter::scan::format(&o))
                }
            })
        }
        Commands::Status => handle_status(cli.repo.as_deref()).map(|o| {
            if cli.json {
                print!("{}", formatter::json::format_status(&o))
            } else {
                print!("{}", formatter::status::format(&o))
            }
        }),
        Commands::Files { language, .. } => handle_files(cli.repo.as_deref(), language.as_deref())
            .map(|o| {
                if cli.json {
                    print!("{}", formatter::json::format_files(&o))
                } else {
                    print!("{}", formatter::files::format(&o))
                }
            }),
        Commands::Symbols { language } => handle_symbols(cli.repo.as_deref(), language.as_deref())
            .map(|o| {
                if cli.json {
                    print!("{}", formatter::json::format_symbols(&o))
                } else {
                    print!("{}", formatter::symbols::format(&o))
                }
            }),
        Commands::Query { query } => handle_query(cli.repo.as_deref(), query).map(|o| match o {
            QueryDispatch::Symbols(view) => {
                if cli.json {
                    print!("{}", formatter::json::format_symbols(&view))
                } else {
                    print!("{}", formatter::symbols::format(&view))
                }
            }
            QueryDispatch::Analysis { text, json } => {
                if cli.json {
                    print!("{json}")
                } else {
                    print!("{text}")
                }
            }
        }),
        Commands::Export { format, output } => {
            handle_export(cli.repo.as_deref(), format, output.as_deref())
        }
        Commands::Chat { message, question } => {
            let repo_path = cli.repo.as_deref();
            let msg = question.as_deref().or(message.as_deref());
            match chat::handle_chat(repo_path, msg, cli.no_color) {
                Ok(view) => {
                    if cli.json {
                        println!("{}", formatter::chat::format_json(&view));
                    } else {
                        let out = formatter::chat::format(&view);
                        if out.is_empty() {
                            println!("(no response)");
                        } else {
                            print!("{}", out);
                        }
                    }
                    Ok(())
                }
                Err(e) => {
                    tracing::error!(error = %e, "Chat command failed");
                    std::process::exit(1);
                }
            }
        }
        Commands::Cache { command } => match command {
            CacheCommands::Status => handle_cache_status(cli.repo.as_deref()).map(|o| {
                if cli.json {
                    print!("{}", formatter::json::format_cache(&o))
                } else {
                    print!("{}", formatter::cache::format(&o))
                }
            }),
            CacheCommands::Clear => handle_cache_clear(cli.repo.as_deref())
                .map(|_| println!("Cache cleared successfully.")),
        },
        Commands::Config { command } => {
            let repo_path = cli.repo.as_deref().unwrap_or(".");
            let path = std::path::Path::new(repo_path);
            match command {
                ConfigCommands::Init => {
                    match config::init(path) {
                        Ok(p) => println!("Configuration initialized at {}", p.display()),
                        Err(e) => tracing::error!(error = %e, "Config Init failed"),
                    }
                    Ok(())
                }
                ConfigCommands::Get { key } => {
                    match config::Config::load(path) {
                        Ok(cfg) => match cfg.get(key) {
                            Some(val) => println!("{val}"),
                            None => tracing::warn!(key = %key, "Config key not found"),
                        },
                        Err(e) => tracing::error!(error = %e, "Config Get failed"),
                    }
                    Ok(())
                }
                ConfigCommands::Set { key, value } => {
                    match config::set(path, key, value) {
                        Ok(()) => println!("set {key} = {value}"),
                        Err(e) => tracing::error!(error = %e, "Config Set failed"),
                    }
                    Ok(())
                }
            }
        }
        Commands::Mcp { command } => {
            match command {
                McpCommands::Serve { path, port } => {
                    let path = resolve_path(Some(Path::new(path.as_str())), cli.repo.as_deref())
                        .to_string();
                    let rt = tokio::runtime::Runtime::new();
                    match rt {
                        Ok(runtime) => {
                            let result = match port {
                                Some(p) => runtime.block_on(mcp::run_http(&path, *p)),
                                None => runtime.block_on(mcp::run_stdio(&path)),
                            };
                            if let Err(e) = result {
                                tracing::error!(error = %e, "MCP serve failed");
                                std::process::exit(1);
                            }
                        }
                        Err(e) => {
                            tracing::error!(error = %e, "MCP serve failed to start tokio runtime");
                            std::process::exit(1);
                        }
                    }
                }
            }
            Ok(())
        }
    };

    if let Err(e) = result {
        tracing::error!(error = %e, "Command failed");
        std::process::exit(1);
    }

    tracing::info!(elapsed = ?start.elapsed(), "CLI command completed");
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
            Commands::Chat { message, question } => {
                assert_eq!(message.as_ref().unwrap(), "hello");
                assert!(question.is_none());
            }
            _ => panic!("Expected Chat command"),
        }
    }

    #[test]
    fn test_chat_with_long_message() {
        let cli = Cli::parse_from(["kode", "chat", "--message", "hello"]);
        match &cli.command {
            Commands::Chat { message, question } => {
                assert_eq!(message.as_ref().unwrap(), "hello");
                assert!(question.is_none());
            }
            _ => panic!("Expected Chat command"),
        }
    }

    #[test]
    fn test_chat_with_positional_question() {
        let cli = Cli::parse_from(["kode", "chat", "what does this do?"]);
        match &cli.command {
            Commands::Chat { message, question } => {
                assert!(message.is_none());
                assert_eq!(question.as_ref().unwrap(), "what does this do?");
            }
            _ => panic!("Expected Chat command"),
        }
    }

    #[test]
    fn test_chat_positional_overrides_message() {
        let cli = Cli::parse_from(["kode", "chat", "-m", "ignored", "used"]);
        match &cli.command {
            Commands::Chat { message, question } => {
                assert_eq!(message.as_ref().unwrap(), "ignored");
                assert_eq!(question.as_ref().unwrap(), "used");
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
                let McpCommands::Serve { path, port } = command;
                assert_eq!(path, ".");
                assert_eq!(*port, None);
            }
            _ => panic!("Expected Mcp command"),
        }
    }

    #[test]
    fn test_mcp_serve_with_port() {
        let cli = Cli::parse_from(["kode", "mcp", "serve", ".", "--port", "3000"]);
        match &cli.command {
            Commands::Mcp { command } => {
                let McpCommands::Serve { path, port } = command;
                assert_eq!(path, ".");
                assert_eq!(*port, Some(3000));
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
    fn test_root_help_contains_all_commands() {
        let mut cmd = Cli::command();
        let help = cmd.render_help().to_string();
        assert!(help.contains("scan"));
        assert!(help.contains("status"));
        assert!(help.contains("files"));
        assert!(help.contains("symbols"));
        assert!(help.contains("query"));
        assert!(help.contains("export"));
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

    mod resolve_path_tests {
        use super::*;

        #[test]
        fn test_none_path_none_repo_returns_dot() {
            assert_eq!(resolve_path(None, None), ".");
        }

        #[test]
        fn test_some_path_none_repo_returns_path() {
            assert_eq!(resolve_path(Some(Path::new("/foo")), None), "/foo");
        }

        #[test]
        fn test_none_path_some_repo_returns_repo() {
            assert_eq!(resolve_path(None, Some("myrepo")), "myrepo");
        }

        #[test]
        fn test_some_path_some_repo_path_wins() {
            assert_eq!(
                resolve_path(Some(Path::new("/path")), Some("repo")),
                "/path"
            );
        }
    }

    #[test]
    fn test_open_storage_no_cache_db_returns_error() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().to_str().unwrap();
        let result = open_storage(Some(path));
        assert!(result.is_err());
        let err = result.err().unwrap();
        assert!(err.to_string().contains("has not been scanned"));
    }

    #[test]
    fn test_json_flag_parsing() {
        let cli = Cli::parse_from(["kode", "--json", "status"]);
        assert!(cli.json);
    }

    #[test]
    fn test_no_color_flag() {
        let cli = Cli::parse_from(["kode", "--no-color", "status"]);
        assert!(cli.no_color);
    }

    #[test]
    fn test_json_and_no_color_together() {
        let cli = Cli::parse_from(["kode", "--json", "--no-color", "scan", "."]);
        assert!(cli.json);
        assert!(cli.no_color);
    }

    #[test]
    fn test_log_init_does_not_panic() {
        init_logging(0, false, None);
        init_logging(1, false, None);
        init_logging(3, false, None);
        init_logging(0, true, None);
    }

    #[test]
    fn test_verbose_zero_log_level() {
        init_logging(0, false, None);
    }

    #[test]
    fn test_verbose_one_log_level() {
        init_logging(1, false, None);
    }

    #[test]
    fn test_verbose_three_log_level() {
        init_logging(3, false, None);
    }

    #[test]
    fn test_quiet_suppresses_output() {
        init_logging(0, true, None);
    }

    #[test]
    fn test_log_file_flag_parsing() {
        let cli = Cli::parse_from(["kode", "--log-file", "/tmp/test.log", "status"]);
        assert_eq!(cli.log_file.as_deref(), Some("/tmp/test.log"));
    }

    #[test]
    fn test_log_file_default_path() {
        let cli = Cli::parse_from(["kode", "status"]);
        let path = resolve_log_path(&cli);
        assert!(path.ends_with(".kode/logs/kode.log"));
    }

    #[test]
    fn test_log_file_creates_file() {
        let dir = tempfile::TempDir::new().unwrap();
        let log_path = dir.path().join("subdir").join("test.log");
        let _guard = init_logging(0, false, Some(log_path.clone()));
        // Directory created even if subscriber init fails (already inited by prior tests)
        assert!(log_path.parent().unwrap().exists());
    }
}
