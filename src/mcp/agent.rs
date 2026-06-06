use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};

use adk_model::openai::{OpenAIClient, OpenAIConfig};
use adk_rust::prelude::*;
use adk_rust::session::{CreateRequest, SessionService};
use anyhow::Result;

use crate::cache::Cache;
use crate::chat::{APP_NAME, BASE_URL, USER_ID};

use super::agent_tools;
use super::evidence::EvidenceCollector;

const SYSTEM_PROMPT: &str = "You are a codebase assistant for the project at the configured root. You have tools to explore the source.\n\n\
    WORKFLOW:\n\
    1. Use `list_files` to discover paths (gitignore-aware). Use `find_symbol` to locate definitions by name.\n\
    2. Call `read_file` on each candidate to read the actual source. Read each file at most once per session.\n\
    3. Stop as soon as the question is answered.\n\n\
    RULES:\n\
    - Never answer from filenames or symbol names alone — read the file first.\n\
    - Never claim a command, function, flag, or feature exists without seeing it in a file you have read this session. README content is not a source of truth — verify in code.\n\
    - Quote exact identifiers as they appear in the source.\n\
    - Cite path:line for each load-bearing claim.\n\
    - Stick to the literal question. Do not paraphrase, expand scope, or volunteer advice.\n\
    - Be specific. No filler.";

pub async fn build_mcp_runner(
    api_key: &str,
    model_name: &str,
    root: &Path,
    counter: usize,
    cache: Arc<Mutex<Cache>>,
    collector: Arc<Mutex<EvidenceCollector>>,
    session_chars: Arc<Mutex<usize>>,
) -> Result<(Runner, String)> {
    let config = OpenAIConfig::compatible(api_key.to_string(), BASE_URL, model_name);
    let model = Arc::new(OpenAIClient::new(config)?);

    let tools_vec = agent_tools::build_all(root.to_path_buf(), cache, collector, session_chars);

    let mut builder = LlmAgentBuilder::new("kode-mcp-agent")
        .instruction(SYSTEM_PROMPT)
        .model(model);
    for tool in tools_vec {
        builder = builder.tool(tool);
    }
    let agent: Arc<dyn Agent> = Arc::new(builder.build()?);

    let sessions: Arc<dyn SessionService> = Arc::new(InMemorySessionService::new());
    let session_id = format!("kode-mcp-session-{counter}");
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
