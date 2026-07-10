use std::sync::Arc;

use kode_query::QueryEngine;

use crate::config::AgentConfig;
use crate::error::AgentError;
use crate::provider::{
    CompletionOutput, CompletionRequest, Message, Provider, Role, StreamEvent, ToolCall,
    ToolDefinition, ToolFunction,
};

pub struct Agent {
    config: AgentConfig,
    provider: Box<dyn Provider>,
    query_engine: Option<Arc<QueryEngine>>,
    history: Vec<Message>,
}

impl Agent {
    pub fn new(
        config: AgentConfig,
        provider: Box<dyn Provider>,
        query_engine: Option<Arc<QueryEngine>>,
    ) -> Self {
        Self {
            config,
            provider,
            query_engine,
            history: Vec::new(),
        }
    }

    fn tool_definitions(&self) -> Vec<ToolDefinition> {
        if self.query_engine.is_some() {
            vec![ToolDefinition {
                kind: "function".into(),
                function: ToolFunction {
                    name: "search_symbols".into(),
                    description: "Search for code symbols by name or pattern in the repository. Returns matching symbols with file paths and line numbers.".into(),
                    parameters: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "query": {
                                "type": "string",
                                "description": "Search query for symbol names"
                            }
                        },
                        "required": ["query"]
                    }),
                },
            }]
        } else {
            Vec::new()
        }
    }

    pub async fn run(&mut self, user_message: &str) -> Result<String, AgentError> {
        let tools = self.tool_definitions();
        let has_tools = !tools.is_empty();
        let system_prompt = if has_tools {
            "You are a code intelligence assistant. Answer questions about the repository \
             using evidence from the knowledge graph and source files. Always cite specific \
             file paths and line numbers in your answers.\n\n\
             Available tools: search_symbols — search for code symbols by name or pattern. \
             Use this tool to find relevant code before answering."
                .to_string()
        } else {
            "You are a code intelligence assistant. Answer questions about the repository \
             using evidence from the knowledge graph and source files. Always cite specific \
             file paths and line numbers in your answers."
                .to_string()
        };

        self.history.push(Message {
            role: Role::User,
            content: user_message.to_string(),
            tool_calls: None,
            tool_call_id: None,
        });

        for _turn in 0..self.config.max_turns {
            let req = CompletionRequest {
                messages: {
                    let mut msgs = vec![Message {
                        role: Role::System,
                        content: system_prompt.clone(),
                        tool_calls: None,
                        tool_call_id: None,
                    }];
                    msgs.extend(self.history.clone());
                    msgs
                },
                tools: tools.clone(),
                temperature: self.config.temperature,
                max_tokens: self.config.max_tokens,
                stream: false,
            };

            let output = self.provider.complete(req).await?;

            match output {
                CompletionOutput::Full {
                    ref text,
                    ref tool_calls,
                    ..
                } => {
                    eprintln!(
                        "[agent] turn {}: text={:?} tool_calls={}",
                        _turn,
                        text,
                        tool_calls.len()
                    );
                    if !text.is_empty() {
                        eprintln!("[agent]   text: {}", text);
                    }
                    if tool_calls.is_empty() {
                        self.history.push(Message {
                            role: Role::Assistant,
                            content: text.clone(),
                            tool_calls: None,
                            tool_call_id: None,
                        });
                        return Ok(text.clone());
                    }

                    self.history.push(Message {
                        role: Role::Assistant,
                        content: text.clone(),
                        tool_calls: Some(tool_calls.clone()),
                        tool_call_id: None,
                    });

                    for tc in tool_calls {
                        let result = self.execute_tool(tc).await?;
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

        Err(AgentError::MaxTurns(self.config.max_turns))
    }

    pub async fn run_stream(
        &mut self,
        user_message: &str,
    ) -> Result<tokio::sync::mpsc::Receiver<StreamEvent>, AgentError> {
        let tools = self.tool_definitions();
        let has_tools = !tools.is_empty();
        let system_prompt = if has_tools {
            "You are a code intelligence assistant. Answer questions about the repository \
             using evidence from the knowledge graph and source files. Always cite specific \
             file paths and line numbers in your answers.\n\n\
             Available tools: search_symbols — search for code symbols by name or pattern. \
             Use this tool to find relevant code before answering."
                .to_string()
        } else {
            "You are a code intelligence assistant. Answer questions about the repository \
             using evidence from the knowledge graph and source files. Always cite specific \
             file paths and line numbers in your answers."
                .to_string()
        };

        self.history.push(Message {
            role: Role::User,
            content: user_message.to_string(),
            tool_calls: None,
            tool_call_id: None,
        });

        let req = CompletionRequest {
            messages: {
                let mut msgs = vec![Message {
                    role: Role::System,
                    content: system_prompt,
                    tool_calls: None,
                    tool_call_id: None,
                }];
                msgs.extend(self.history.clone());
                msgs
            },
            tools: tools.clone(),
            temperature: self.config.temperature,
            max_tokens: self.config.max_tokens,
            stream: true,
        };

        let output = self.provider.complete(req).await?;

        match output {
            CompletionOutput::Streamed(rx) => Ok(rx),
            CompletionOutput::Full { .. } => Err(AgentError::Provider(
                "Provider does not support streaming".into(),
            )),
        }
    }

    async fn execute_tool(&self, call: &ToolCall) -> Result<String, AgentError> {
        match call.name.as_str() {
            "search_symbols" => {
                let engine = self
                    .query_engine
                    .as_ref()
                    .ok_or_else(|| AgentError::Tool("No query engine available".into()))?;
                let query = call.arguments["query"].as_str().unwrap_or("").to_string();
                let results = engine
                    .search_symbols(&query, None)
                    .map_err(AgentError::Query)?;
                if results.is_empty() {
                    Ok("No matching symbols found.".into())
                } else {
                    let mut lines = vec![format!("Found {} symbols:", results.len())];
                    for s in &results {
                        lines.push(format!(
                            "  - {} ({}) at {}:{}",
                            s.name,
                            s.kind.as_str(),
                            s.file_path.display(),
                            s.start_line
                        ));
                    }
                    Ok(lines.join("\n"))
                }
            }
            _ => Err(AgentError::Tool(format!("Unknown tool: {}", call.name))),
        }
    }
}
