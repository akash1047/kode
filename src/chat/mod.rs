mod commands;
mod prompt;
mod render;
mod spinner;
mod tools;
mod ui;

use std::collections::HashMap;
use std::env;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use adk_core::{Content, Part, SessionId, UserId};
use adk_model::openai::{OpenAIClient, OpenAIConfig};
use adk_rust::futures::StreamExt;
use adk_rust::prelude::*;
use adk_rust::session::{CreateRequest, SessionService};

use crossterm::event::{KeyCode, KeyModifiers};
use reedline::{
    EditCommand, Emacs, FileBackedHistory, Reedline, ReedlineEvent, Signal,
    default_emacs_keybindings,
};

use commands::SlashCommand;
use prompt::{ContinuationValidator, KodePrompt};
use spinner::Spinner;

use crate::cache::Cache;
use crate::config::KodeConfig;

/// Default base URL for the OpenAI-compatible API endpoint.
pub const BASE_URL: &str = "https://ollama.com/v1";
/// Default model name used when no override is provided.
pub const DEFAULT_MODEL: &str = "nemotron-3-nano:30b";
/// Environment variable that overrides the model name.
pub const MODEL_ENV: &str = "KODE_MODEL";
/// Environment variable that supplies the API key.
pub const API_KEY_ENV: &str = "KODE_MODEL_API_KEY";
/// ADK application name used for session scoping.
pub const APP_NAME: &str = "kode-chat";
/// ADK user identifier used for session scoping.
pub const USER_ID: &str = "user";
const HISTORY_CAPACITY: usize = 1000;

const SYSTEM_PROMPT: &str = "You are a codebase assistant for the project at the configured root. You have tools to explore the source.\n\n\
    AVAILABLE TOOLS (these are the ONLY tools — do not call anything else):\n\
    - `list_files` — enumerate project files (gitignore-aware). USE FOR FILENAME / PATH DISCOVERY.\n\
    - `find_symbol` — locate a definition by name in the symbol index.\n\
    - `search_project` — regex over file CONTENTS (not filenames).\n\
    - `read_project_files` — read source spans (batched).\n\n\
    WORKFLOW:\n\
    1. To find which files exist or match a name pattern, call `list_files`. To find a definition by name, call `find_symbol`. To find a literal string or regex inside source, call `search_project`.\n\
    2. Call `read_project_files` on each candidate to read the actual source. Batch multiple reads into a single call. Read each file at most once per session.\n\
    3. Stop as soon as the question is answered.\n\n\
    RULES:\n\
    - Never answer from filenames, symbol names, or search snippets alone — read the file first.\n\
    - Never claim a command, function, flag, or feature exists without seeing it in a file you have read this session. README content is not a source of truth — verify in code.\n\
    - Quote exact identifiers as they appear in the source.\n\
    - Cite path:line for each load-bearing claim.\n\
    - If a search returns 0 hits, change tool or change strategy — do not vary regex spelling and retry. Filename lookups belong in `list_files`, not `search_project`.\n\
    - Stick to the literal question. Do not paraphrase, expand scope, or volunteer advice.\n\
    - Be specific. No filler.";

struct ReplState {
    api_key: String,
    model_name: String,
    base_url: String,
    root: PathBuf,
    cache: Arc<Mutex<Cache>>,
    runner: Runner,
    session_id: String,
    session_counter: usize,
}

impl ReplState {
    async fn new(
        root: PathBuf,
        api_key: String,
        model_name: String,
        base_url: String,
        cache: Arc<Mutex<Cache>>,
    ) -> anyhow::Result<Self> {
        let (runner, session_id) =
            build_runner(&api_key, &model_name, &base_url, &root, 1, cache.clone()).await?;
        Ok(Self {
            api_key,
            model_name,
            base_url,
            root,
            cache,
            runner,
            session_id,
            session_counter: 1,
        })
    }

    async fn rebuild_runner(&mut self) -> anyhow::Result<()> {
        self.session_counter += 1;
        let (runner, session_id) = build_runner(
            &self.api_key,
            &self.model_name,
            &self.base_url,
            &self.root,
            self.session_counter,
            self.cache.clone(),
        )
        .await?;
        self.runner = runner;
        self.session_id = session_id;
        Ok(())
    }
}

/// Build an ADK `Runner` with all project tools registered and a fresh session.
///
/// `counter` is appended to the session ID to make each REPL turn uniquely addressable.
pub async fn build_runner(
    api_key: &str,
    model_name: &str,
    base_url: &str,
    root: &Path,
    counter: usize,
    cache: Arc<Mutex<Cache>>,
) -> anyhow::Result<(Runner, String)> {
    let config = OpenAIConfig::compatible(api_key.to_string(), base_url, model_name);
    let model = Arc::new(OpenAIClient::new(config)?);

    let tools_vec = tools::build_all(root.to_path_buf(), cache);

    let mut builder = LlmAgentBuilder::new("kode-agent")
        .instruction(SYSTEM_PROMPT)
        .model(model);
    for tool in tools_vec {
        builder = builder.tool(tool);
    }
    let agent: Arc<dyn Agent> = Arc::new(builder.build()?);

    let sessions: Arc<dyn SessionService> = Arc::new(InMemorySessionService::new());
    let session_id = format!("kode-session-{counter}");
    sessions
        .create(CreateRequest {
            app_name: APP_NAME.into(),
            user_id: USER_ID.into(),
            session_id: Some(session_id.clone()),
            state: HashMap::new(),
        })
        .await?;

    let runner = Runner::builder()
        .app_name(APP_NAME)
        .agent(agent)
        .session_service(sessions)
        .build()?;
    Ok((runner, session_id))
}

fn resolve_model(model_override: Option<String>, config_model: Option<String>, default_model: Option<String>) -> String {
    model_override
        .or_else(|| env::var(MODEL_ENV).ok())
        .or(config_model)
        .or(default_model)
        .unwrap_or_else(|| DEFAULT_MODEL.to_string())
}

fn resolve_api_key(config_key: Option<String>, default_key: Option<String>) -> anyhow::Result<String> {
    env::var(API_KEY_ENV)
        .ok()
        .or(config_key)
        .or(default_key)
        .ok_or_else(|| anyhow::anyhow!("{API_KEY_ENV} not set and not in config [chat] or [default]"))
}

fn resolve_base_url(config_url: Option<String>, default_url: Option<String>) -> String {
    config_url.or(default_url).unwrap_or_else(|| BASE_URL.to_string())
}

/// Run a single question-answer cycle, print the answer to stdout, and return.
pub async fn run_one_shot(
    root: &Path,
    message: &str,
    model_override: Option<String>,
) -> anyhow::Result<()> {
    let cfg = KodeConfig::load();
    let chat_cfg = cfg.chat.as_ref();
    let def = cfg.default.as_ref();
    let api_key = resolve_api_key(chat_cfg.and_then(|c| c.api_key.clone()), def.and_then(|d| d.api_key.clone()))?;
    let model_name = resolve_model(model_override, chat_cfg.and_then(|c| c.model.clone()), def.and_then(|d| d.model.clone()));
    let base_url = resolve_base_url(chat_cfg.and_then(|c| c.base_url.clone()), def.and_then(|d| d.base_url.clone()));

    let mut cache = Cache::open(root)?;
    let _ = crate::cache::revalidate::refresh(&mut cache)?;
    let cache = Arc::new(Mutex::new(cache));

    let (runner, session_id) =
        build_runner(&api_key, &model_name, &base_url, root, 1, cache).await?;
    let reply = one_shot(&runner, &session_id, message).await?;
    println!("{}", reply.trim_end());
    Ok(())
}

/// Start the interactive chat REPL loop until the user exits.
pub async fn run_repl(root: &Path, model_override: Option<String>) -> anyhow::Result<()> {
    let cfg = KodeConfig::load();
    let chat_cfg = cfg.chat.as_ref();
    let def = cfg.default.as_ref();
    let api_key = resolve_api_key(chat_cfg.and_then(|c| c.api_key.clone()), def.and_then(|d| d.api_key.clone()))?;
    let model_name = resolve_model(model_override, chat_cfg.and_then(|c| c.model.clone()), def.and_then(|d| d.model.clone()));
    let base_url = resolve_base_url(chat_cfg.and_then(|c| c.base_url.clone()), def.and_then(|d| d.base_url.clone()));

    let mut cache = Cache::open(root)?;
    let report = crate::cache::revalidate::refresh(&mut cache)?;
    let cache = Arc::new(Mutex::new(cache));

    let mut state = ReplState::new(root.to_path_buf(), api_key, model_name, base_url, cache).await?;

    ui::banner(&state.model_name);
    let summary_counts = {
        let guard = state.cache.lock().expect("cache mutex poisoned");
        guard
            .stats()
            .map(|s| (s.files, s.symbols, s.symbol_files))
            .unwrap_or((0, 0, 0))
    };
    let _ = summary_counts;
    let summary_count = {
        let guard = state.cache.lock().expect("cache mutex poisoned");
        guard
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM file_summaries",
                [],
                |r| r.get::<_, i64>(0),
            )
            .unwrap_or(0) as usize
    };
    ui::cache_summary(
        report.scanned,
        report.added,
        report.changed,
        report.unchanged,
        report.deleted,
        report.manifests,
        report.run_configs,
        report.readmes,
        report.symbols_files,
        report.symbols_total,
        report.dirs_hashed,
        summary_count,
    );

    let mut line_editor = build_line_editor()?;

    loop {
        let prompt = KodePrompt {
            model: state.model_name.clone(),
        };

        let sig = tokio::task::block_in_place(|| line_editor.read_line(&prompt))?;

        match sig {
            Signal::Success(input) => {
                let trimmed = input.trim();
                if trimmed.is_empty() {
                    continue;
                }

                if let Some(cmd) = commands::parse(trimmed) {
                    if handle_command(cmd, &mut state).await? {
                        break;
                    }
                    continue;
                }

                let cleaned = strip_continuations(&input);
                if let Err(e) = run_turn(&state.runner, &state.session_id, &cleaned).await {
                    ui::error_line(&format!("{e:#}"));
                }
            }
            Signal::CtrlC => {
                ui::info_line("(empty prompt — ctrl-d or /exit to quit)");
            }
            Signal::CtrlD => break,
            _ => {}
        }
    }

    Ok(())
}

fn build_line_editor() -> anyhow::Result<Reedline> {
    let history_path = history_file_path();
    let history = FileBackedHistory::with_file(HISTORY_CAPACITY, history_path)?;

    let mut keybindings = default_emacs_keybindings();
    keybindings.add_binding(
        KeyModifiers::ALT,
        KeyCode::Enter,
        ReedlineEvent::Edit(vec![EditCommand::InsertNewline]),
    );
    keybindings.add_binding(
        KeyModifiers::SHIFT,
        KeyCode::Enter,
        ReedlineEvent::Edit(vec![EditCommand::InsertNewline]),
    );

    let edit_mode = Box::new(Emacs::new(keybindings));

    Ok(Reedline::create()
        .with_history(Box::new(history))
        .with_validator(Box::new(ContinuationValidator))
        .with_edit_mode(edit_mode))
}

fn history_file_path() -> PathBuf {
    if let Some(home) = dirs_home() {
        return home.join(".kode_history");
    }
    PathBuf::from(".kode_history")
}

fn dirs_home() -> Option<PathBuf> {
    env::var_os("HOME").map(PathBuf::from)
}

fn strip_continuations(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for line in input.lines() {
        if let Some(stripped) = line.strip_suffix('\\') {
            out.push_str(stripped);
            out.push(' ');
        } else {
            out.push_str(line);
            out.push('\n');
        }
    }
    out.trim_end().to_string()
}

async fn handle_command(cmd: SlashCommand, state: &mut ReplState) -> anyhow::Result<bool> {
    match cmd {
        SlashCommand::Help => commands::print_help(),
        SlashCommand::Clear => commands::clear_terminal(),
        SlashCommand::Reset => {
            state.rebuild_runner().await?;
            ui::info_line("new conversation started");
        }
        SlashCommand::Model(None) => {
            ui::info_line(&format!("current model: {}", state.model_name));
        }
        SlashCommand::Model(Some(name)) => {
            let previous = std::mem::replace(&mut state.model_name, name.clone());
            match state.rebuild_runner().await {
                Ok(()) => ui::info_line(&format!("model switched to: {name}")),
                Err(e) => {
                    state.model_name = previous;
                    let _ = state.rebuild_runner().await;
                    ui::error_line(&format!("model switch failed: {e:#}"));
                }
            }
        }
        SlashCommand::Exit => return Ok(true),
        SlashCommand::Unknown(name) => {
            ui::error_line(&format!("unknown command: /{name} (try /help)"));
        }
    }
    Ok(false)
}

async fn run_turn(runner: &Runner, session_id: &str, user_text: &str) -> anyhow::Result<()> {
    tokio::select! {
        result = drive_turn(runner, session_id, user_text) => result,
        _ = tokio::signal::ctrl_c() => {
            ui::cancel_line();
            Ok(())
        }
    }
}

/// Send `user_text` to the runner and collect the full text reply.
///
/// Tool calls are printed to stderr; the returned `String` contains only the
/// final text response.
pub async fn one_shot(
    runner: &Runner,
    session_id: &str,
    user_text: &str,
) -> anyhow::Result<String> {
    let mut stream = runner
        .run(
            UserId::new(USER_ID)?,
            SessionId::new(session_id)?,
            Content::new("user").with_text(user_text),
        )
        .await?;

    let mut reply = String::new();
    while let Some(event) = stream.next().await {
        let event = event?;
        let Some(content) = &event.llm_response.content else {
            continue;
        };
        for part in &content.parts {
            match part {
                Part::Text { text } => reply.push_str(text),
                Part::FunctionCall { name, args, .. } => {
                    eprintln!("tool> {name} {}", compact_args(args));
                }
                _ => {}
            }
        }
    }
    Ok(reply)
}

fn compact_args(args: &serde_json::Value) -> String {
    let s = args.to_string();
    if s.chars().count() <= 160 {
        return s;
    }
    let head: String = s.chars().take(160).collect();
    format!("{head}…")
}

async fn drive_turn(
    runner: &Runner,
    session_id: &str,
    user_text: &str,
) -> anyhow::Result<()> {
    let mut stream = runner
        .run(
            UserId::new(USER_ID)?,
            SessionId::new(session_id)?,
            Content::new("user").with_text(user_text),
        )
        .await?;

    let mut active_spinner: Option<Spinner> = Some(Spinner::start("thinking"));
    let mut in_thinking = false;
    let mut reply_buffer = String::new();
    let renderer = render::Renderer::new();
    let mut in_flight: Vec<(String, serde_json::Value, std::time::Instant)> = Vec::new();

    while let Some(event) = stream.next().await {
        let event = event?;
        let Some(content) = &event.llm_response.content else {
            continue;
        };

        for part in &content.parts {
            match part {
                Part::Thinking { thinking, .. } => {
                    if let Some(s) = &active_spinner {
                        s.set_label("reasoning");
                    }
                    if let Some(s) = active_spinner.take() {
                        s.stop().await;
                    }
                    if !in_thinking {
                        print!("\n{}", ui::thinking_prefix());
                        in_thinking = true;
                    }
                    print!("{}{}{}", ui::DIM, thinking, ui::RESET);
                    std::io::stdout().flush().ok();
                }
                Part::FunctionCall { name, args, .. } => {
                    if in_thinking {
                        println!();
                        in_thinking = false;
                    }
                    if let Some(s) = &active_spinner {
                        s.set_label(format!("running {name}"));
                    } else {
                        active_spinner = Some(Spinner::start(format!("running {name}")));
                    }
                    in_flight.push((name.clone(), args.clone(), std::time::Instant::now()));
                }
                Part::FunctionResponse { function_response, .. } => {
                    let (n, args, start) = in_flight
                        .iter()
                        .position(|(n, _, _)| n == &function_response.name)
                        .map(|idx| in_flight.remove(idx))
                        .unwrap_or_else(|| {
                            (function_response.name.clone(), serde_json::Value::Null, std::time::Instant::now())
                        });
                    let elapsed = start.elapsed();
                    if let Some(s) = active_spinner.take() {
                        s.stop().await;
                    }
                    ui::tool_completed(&n, &args, &function_response.response, elapsed);
                    active_spinner = Some(Spinner::start("thinking"));
                }
                Part::Text { text } if !text.is_empty() => {
                    if let Some(s) = &active_spinner {
                        s.set_label("writing reply");
                    }
                    reply_buffer.push_str(text);
                }
                _ => {}
            }
        }
    }

    if let Some(s) = active_spinner.take() {
        s.stop().await;
    }
    if in_thinking {
        println!();
    }

    if !reply_buffer.trim().is_empty() {
        println!("{}", ui::assistant_prefix());
        renderer.print(&reply_buffer);
        println!();
    } else {
        println!();
    }
    Ok(())
}
