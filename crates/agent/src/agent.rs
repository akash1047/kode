//! Multi-turn agent with sandboxed tools and optional knowledge-graph queries.

use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::mpsc;

use crate::config::AgentConfig;
use crate::error::AgentError;
use crate::index::{SharedIndex, SymbolIndex};
use crate::provider::{CompletionOutput, CompletionRequest, Message, Provider, Role, ToolCall};
use crate::tools;

/// Events emitted during an agent turn for TUI / one-shot consumers.
#[derive(Debug, Clone)]
pub enum AgentEvent {
    /// Incremental assistant text (may be a full block when non-streaming).
    Delta(String),
    /// Tool about to run.
    ToolStart { name: String, args: String },
    /// Tool finished; `preview` is a short status line.
    ToolResult { name: String, preview: String },
    /// Final assistant answer for this user turn.
    Done(String),
    /// Fatal error for this turn.
    Error(String),
}

/// Repository-aware chat agent (Send so turns can run on worker tasks).
pub struct Agent {
    config: AgentConfig,
    provider: Box<dyn Provider>,
    symbol_index: Option<SharedIndex>,
    project_root: PathBuf,
    tools_enabled: bool,
    history: Vec<Message>,
}

impl Agent {
    /// Create an agent. `project_root` sandboxes filesystem tools.
    pub fn new(
        config: AgentConfig,
        provider: Box<dyn Provider>,
        symbol_index: Option<SharedIndex>,
        project_root: PathBuf,
    ) -> Self {
        Self {
            config,
            provider,
            symbol_index,
            project_root,
            tools_enabled: true,
            history: Vec::new(),
        }
    }

    /// Whether filesystem / query tools are offered to the model.
    pub fn set_tools_enabled(&mut self, enabled: bool) {
        self.tools_enabled = enabled;
    }

    pub fn tools_enabled(&self) -> bool {
        self.tools_enabled
    }

    pub fn project_root(&self) -> &std::path::Path {
        &self.project_root
    }

    pub fn has_symbol_index(&self) -> bool {
        self.symbol_index.is_some()
    }

    pub fn history(&self) -> &[Message] {
        &self.history
    }

    pub fn clear_history(&mut self) {
        self.history.clear();
    }

    fn system_prompt(&self) -> String {
        let mut s = String::from(
            "You are kode, an evidence-first code intelligence assistant. \
             Answer questions about this repository using tools and live source. \
             Always cite specific file paths and line numbers. \
             Prefer search_symbols/find_symbol when a knowledge graph is available; \
             use find_callers/find_callees/impact_analysis for call-graph questions; \
             use list_dir, grep, and read_file to inspect source. \
             Do not invent symbols or file paths.",
        );
        if self.symbol_index.is_none() {
            s.push_str(
                "\n\nNote: no knowledge graph index is loaded. \
                 Prefer list_dir/grep/read_file, and suggest the user run `kode scan`.",
            );
        }
        s
    }

    fn tool_defs(&self) -> Vec<crate::provider::ToolDefinition> {
        if self.tools_enabled {
            tools::tool_definitions(self.symbol_index.is_some())
        } else {
            Vec::new()
        }
    }

    /// Run one user turn (blocking tool loop). Returns final assistant text.
    pub async fn run(&mut self, user_message: &str) -> Result<String, AgentError> {
        self.run_with_tx(user_message, None).await
    }

    /// Run one user turn, emitting progress on `tx` (rusts/kode-style).
    pub async fn run_with_tx(
        &mut self,
        user_message: &str,
        tx: Option<mpsc::UnboundedSender<AgentEvent>>,
    ) -> Result<String, AgentError> {
        let tools = self.tool_defs();
        let system_prompt = self.system_prompt();

        self.history.push(Message {
            role: Role::User,
            content: user_message.to_string(),
            tool_calls: None,
            tool_call_id: None,
        });

        let max_turns = self.config.max_turns.max(1);
        for _turn in 0..max_turns {
            let mut messages = vec![Message {
                role: Role::System,
                content: system_prompt.clone(),
                tool_calls: None,
                tool_call_id: None,
            }];
            messages.extend(self.history.clone());

            let req = CompletionRequest {
                messages,
                tools: tools.clone(),
                temperature: self.config.temperature,
                max_tokens: self.config.max_tokens,
                // Non-streaming complete is reliable for tool_calls across providers.
                stream: false,
            };

            let output = self.provider.complete(req).await?;

            match output {
                CompletionOutput::Full {
                    text, tool_calls, ..
                } => {
                    if tool_calls.is_empty() {
                        if text.is_empty() {
                            let err = "empty assistant response".to_string();
                            if let Some(ref t) = tx {
                                let _ = t.send(AgentEvent::Error(err.clone()));
                            }
                            return Err(AgentError::Provider(err));
                        }
                        self.history.push(Message {
                            role: Role::Assistant,
                            content: text.clone(),
                            tool_calls: None,
                            tool_call_id: None,
                        });
                        if let Some(ref t) = tx {
                            let _ = t.send(AgentEvent::Delta(text.clone()));
                            let _ = t.send(AgentEvent::Done(text.clone()));
                        }
                        return Ok(text);
                    }

                    self.history.push(Message {
                        role: Role::Assistant,
                        content: text,
                        tool_calls: Some(tool_calls.clone()),
                        tool_call_id: None,
                    });

                    for tc in &tool_calls {
                        let args_str = tc.arguments.to_string();
                        if let Some(ref t) = tx {
                            let _ = t.send(AgentEvent::ToolStart {
                                name: tc.name.clone(),
                                args: args_str,
                            });
                        }

                        let result = self.execute_tool(tc).await;

                        if let Some(ref t) = tx {
                            let _ = t.send(AgentEvent::ToolResult {
                                name: tc.name.clone(),
                                preview: tools::preview(&result, 120),
                            });
                        }

                        self.history.push(Message {
                            role: Role::Tool,
                            content: result,
                            tool_calls: None,
                            tool_call_id: Some(tc.id.clone()),
                        });
                    }
                }
                CompletionOutput::Streamed(_) => {
                    return Err(AgentError::Provider(
                        "Unexpected streaming response in non-streaming call".into(),
                    ));
                }
            }
        }

        let err = format!("stopped: exceeded max tool turns ({max_turns})");
        if let Some(ref t) = tx {
            let _ = t.send(AgentEvent::Error(err.clone()));
        }
        Err(AgentError::MaxTurns(max_turns))
    }

    async fn execute_tool(&self, call: &ToolCall) -> String {
        let root = self.project_root.clone();
        let name = call.name.clone();
        let args = call.arguments.clone();
        let index = self.symbol_index.clone();

        match tokio::task::spawn_blocking(move || {
            tools::execute(&root, &name, &args, index.as_ref().map(|a| a.as_ref()))
        })
        .await
        {
            Ok(s) => s,
            Err(e) => format!("error: tool task failed: {e}"),
        }
    }
}

// Ensure SharedIndex type is used publicly via Agent API.
#[allow(dead_code)]
fn _assert_index_send(idx: SymbolIndex) -> SharedIndex {
    Arc::new(idx)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ProviderKind;
    use crate::provider::{CompletionOutput, ToolDefinition};
    use async_trait::async_trait;
    use std::sync::Mutex;

    struct ScriptedProvider {
        responses: Mutex<Vec<CompletionOutput>>,
    }

    #[async_trait]
    impl Provider for ScriptedProvider {
        async fn complete(&self, _req: CompletionRequest) -> Result<CompletionOutput, AgentError> {
            let mut guard = self.responses.lock().unwrap();
            guard
                .pop()
                .ok_or_else(|| AgentError::Provider("no scripted response".into()))
        }

        fn name(&self) -> &str {
            "scripted"
        }
    }

    fn cfg() -> AgentConfig {
        AgentConfig {
            provider: ProviderKind::Ollama,
            model: "test".into(),
            api_key: None,
            max_turns: 5,
            ..AgentConfig::default()
        }
    }

    #[tokio::test]
    async fn run_returns_text_without_tools() {
        let provider = Box::new(ScriptedProvider {
            responses: Mutex::new(vec![CompletionOutput::Full {
                text: "hello from model".into(),
                tool_calls: vec![],
                usage: None,
            }]),
        });
        let mut agent = Agent::new(cfg(), provider, None, PathBuf::from("/tmp"));
        agent.set_tools_enabled(false);
        let out = agent.run("hi").await.unwrap();
        assert_eq!(out, "hello from model");
    }

    #[tokio::test]
    async fn run_executes_list_dir_tool() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.rs"), "fn main(){}").unwrap();

        let tool_call = ToolCall {
            id: "call_1".into(),
            name: "list_dir".into(),
            arguments: serde_json::json!({"path": "."}),
        };
        // Stack: pop() serves tool call first, then final text.
        let provider = Box::new(ScriptedProvider {
            responses: Mutex::new(vec![
                CompletionOutput::Full {
                    text: "Here is the listing.".into(),
                    tool_calls: vec![],
                    usage: None,
                },
                CompletionOutput::Full {
                    text: String::new(),
                    tool_calls: vec![tool_call],
                    usage: None,
                },
            ]),
        });

        let mut agent = Agent::new(cfg(), provider, None, dir.path().to_path_buf());
        let (tx, mut rx) = mpsc::unbounded_channel();
        let out = agent.run_with_tx("list the root", Some(tx)).await.unwrap();
        assert!(out.contains("listing") || out.contains("Here"));

        let mut saw_tool = false;
        while let Ok(ev) = rx.try_recv() {
            if matches!(ev, AgentEvent::ToolStart { .. }) {
                saw_tool = true;
            }
        }
        assert!(saw_tool);
    }

    #[test]
    fn tool_defs_include_fs_tools() {
        let provider = Box::new(ScriptedProvider {
            responses: Mutex::new(vec![]),
        });
        let agent = Agent::new(cfg(), provider, None, PathBuf::from("/tmp"));
        let defs: Vec<ToolDefinition> = agent.tool_defs();
        assert!(defs.iter().any(|d| d.function.name == "grep"));
    }
}
