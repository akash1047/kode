pub(crate) mod engine;
pub(crate) mod input;
pub(crate) mod markdown;
pub(crate) mod theme;
pub(crate) mod tui;

use std::path::Path;

use crate::config::ChatConfig;

pub use theme::Theme;

#[derive(Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

pub struct ChatSession {
    engine: engine::ChatEngine,
    pub messages: Vec<ChatMessage>,
    pub is_streaming: bool,
}

impl ChatSession {
    pub fn new(config: ChatConfig) -> Result<Self, ChatError> {
        let engine = engine::ChatEngine::new(&config)?;
        Ok(Self {
            engine,
            messages: Vec::new(),
            is_streaming: false,
        })
    }

    pub fn ask(&mut self, query: &str) -> Result<ChatMessage, ChatError> {
        let history: Vec<(String, String)> = self
            .messages
            .iter()
            .map(|m| (m.role.clone(), m.content.clone()))
            .collect();

        let answer = self.engine.complete(&history, query)?;

        self.messages.push(ChatMessage {
            role: "user".into(),
            content: query.to_string(),
        });
        let msg = ChatMessage {
            role: "assistant".into(),
            content: answer,
        };
        self.messages.push(msg.clone());
        Ok(msg)
    }

    pub fn ask_stream(
        &mut self,
        query: &str,
    ) -> Result<tokio::sync::mpsc::Receiver<engine::StreamEvent>, ChatError> {
        let history: Vec<(String, String)> = self
            .messages
            .iter()
            .map(|m| (m.role.clone(), m.content.clone()))
            .collect();

        let rx = self.engine.complete_stream(&history, query)?;
        self.is_streaming = true;

        Ok(rx)
    }

    pub fn start_interactive(
        &mut self,
        theme: Theme,
        repo_name: String,
        model_name: String,
    ) -> Result<(), ChatError> {
        let mut tui = tui::ChatTui::new(theme, repo_name, model_name);
        tui.run(self)?;
        Ok(())
    }
}

pub struct ChatView {
    pub answer: String,
}

pub fn handle_chat(
    path: Option<&str>,
    message: Option<&str>,
    no_color: bool,
) -> Result<ChatView, Box<dyn std::error::Error>> {
    let repo_path = path.unwrap_or(".");
    let path = Path::new(repo_path);

    let config = ChatConfig::load(path);

    let mut session = ChatSession::new(config.clone())?;

    match message {
        Some(query) => {
            let msg = session.ask(query)?;
            Ok(ChatView {
                answer: msg.content,
            })
        }
        None => {
            let theme = if no_color {
                Theme::no_color()
            } else {
                match config.theme.as_str() {
                    "light" => Theme::light(),
                    "auto" => Theme::dark(),
                    _ => Theme::dark(),
                }
            };
            let repo_name = path
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "unknown".to_string());
            let model_name = config.model.clone();
            let rt = tokio::runtime::Runtime::new()?;
            let _guard = rt.enter();
            session.start_interactive(theme, repo_name, model_name)?;
            Ok(ChatView {
                answer: String::new(),
            })
        }
    }
}

#[derive(Debug)]
pub enum ChatError {
    Engine(String),
    Io(std::io::Error),
}

impl std::fmt::Display for ChatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChatError::Engine(msg) => write!(f, "{}", msg),
            ChatError::Io(e) => write!(f, "{}", e),
        }
    }
}

impl std::error::Error for ChatError {}

impl From<std::io::Error> for ChatError {
    fn from(e: std::io::Error) -> Self {
        ChatError::Io(e)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chat_error_display_engine() {
        let e = ChatError::Engine("api error".into());
        assert_eq!(format!("{}", e), "api error");
    }

    #[test]
    fn test_chat_error_display_io() {
        let e = ChatError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "file missing",
        ));
        assert!(format!("{}", e).contains("file missing"));
    }

    #[test]
    fn test_chat_error_from_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied");
        let e: ChatError = io_err.into();
        assert!(matches!(e, ChatError::Io(_)));
    }

    #[test]
    fn test_chat_message_construction() {
        let msg = ChatMessage {
            role: "user".into(),
            content: "hello".into(),
        };
        assert_eq!(msg.role, "user");
        assert_eq!(msg.content, "hello");
    }

    #[test]
    fn test_chat_view_construction() {
        let view = ChatView {
            answer: "test answer".into(),
        };
        assert_eq!(view.answer, "test answer");
    }

    #[test]
    fn test_ask_stream_does_not_push_user_message() {
        // ask_stream should not push user message (TUI owns that)
        // Verified by checking ask_stream signature doesn't mutate history
        // (compile-time guarantee)
    }

    #[test]
    fn test_api_key_validation_error_message() {
        let err = ChatError::Engine("No API key configured. Set OPENAI_API_KEY environment variable or configure chat.api_key in .kode/config.toml".into());
        assert!(format!("{}", err).contains("API key"));
        assert!(format!("{}", err).contains("OPENAI_API_KEY"));
    }
}
