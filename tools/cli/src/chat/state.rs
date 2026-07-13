//! Chat UI state (bubbles, scroll, status) — patterned on rusts/kode.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    User,
    Assistant,
    Status,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bubble {
    pub role: Role,
    pub content: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunStatus {
    Idle,
    Streaming,
    Error,
}

pub struct UiState {
    pub bubbles: Vec<Bubble>,
    pub status: RunStatus,
    pub status_line: String,
    pub scroll_from_bottom: u16,
    pub stick_to_bottom: bool,
    pub spinner_frame: usize,
    pub should_quit: bool,
    pub model: String,
    pub provider: String,
    pub tools_enabled: bool,
    pub has_index: bool,
}

impl UiState {
    pub fn new(model: &str, provider: &str, tools_enabled: bool, has_index: bool) -> Self {
        let mut s = Self {
            bubbles: Vec::new(),
            status: RunStatus::Idle,
            status_line: String::new(),
            scroll_from_bottom: 0,
            stick_to_bottom: true,
            spinner_frame: 0,
            should_quit: false,
            model: model.to_string(),
            provider: provider.to_string(),
            tools_enabled,
            has_index,
        };
        let index = if has_index {
            "graph index loaded"
        } else {
            "no index — run `kode scan` for symbol tools"
        };
        s.push_status(format!(
            "kode chat · {} · {}\ntools: list_dir, grep, read_file{} · Enter send · Ctrl+J newline · /help · Ctrl+C quit",
            s.model,
            index,
            if has_index {
                ", search_symbols, find_symbol"
            } else {
                ""
            }
        ));
        s.status_line = format!(
            "ready · tools {} · {}",
            if tools_enabled { "on" } else { "off" },
            if has_index { "indexed" } else { "no index" }
        );
        s
    }

    pub fn push_status(&mut self, text: impl Into<String>) {
        self.bubbles.push(Bubble {
            role: Role::Status,
            content: text.into(),
        });
        if self.stick_to_bottom {
            self.scroll_from_bottom = 0;
        }
    }

    pub fn push_user(&mut self, text: &str) {
        self.bubbles.push(Bubble {
            role: Role::User,
            content: text.to_string(),
        });
        if self.stick_to_bottom {
            self.scroll_from_bottom = 0;
        }
    }

    pub fn begin_assistant(&mut self) {
        self.bubbles.push(Bubble {
            role: Role::Assistant,
            content: String::new(),
        });
        self.status = RunStatus::Streaming;
        self.status_line = "thinking…".into();
        if self.stick_to_bottom {
            self.scroll_from_bottom = 0;
        }
    }

    pub fn append_delta(&mut self, delta: &str) {
        let need_new = !matches!(self.bubbles.last(), Some(b) if b.role == Role::Assistant);
        if need_new {
            self.bubbles.push(Bubble {
                role: Role::Assistant,
                content: String::new(),
            });
        }
        if let Some(last) = self.bubbles.last_mut() {
            if last.role == Role::Assistant {
                last.content.push_str(delta);
            }
        }
        self.status = RunStatus::Streaming;
        self.status_line = "streaming…".into();
        if self.stick_to_bottom {
            self.scroll_from_bottom = 0;
        }
    }

    pub fn finish_stream(&mut self, full: String) {
        if let Some(last) = self.bubbles.last_mut() {
            if last.role == Role::Assistant {
                if (last.content.is_empty() || last.content != full) && !full.is_empty() {
                    last.content = full;
                }
            } else if !full.is_empty() {
                self.bubbles.push(Bubble {
                    role: Role::Assistant,
                    content: full,
                });
            }
        } else if !full.is_empty() {
            self.bubbles.push(Bubble {
                role: Role::Assistant,
                content: full,
            });
        }
        self.status = RunStatus::Idle;
        self.status_line = format!(
            "ready · tools {}",
            if self.tools_enabled { "on" } else { "off" }
        );
    }

    pub fn fail_stream(&mut self, err: String) {
        if let Some(last) = self.bubbles.last() {
            if last.role == Role::Assistant && last.content.is_empty() {
                self.bubbles.pop();
            }
        }
        self.status = RunStatus::Error;
        self.status_line = err.clone();
        self.push_status(format!("error: {err}"));
    }

    pub fn note_tool_start(&mut self, name: &str, args: &str) {
        let args = if args.len() > 80 {
            format!("{}…", &args[..80])
        } else {
            args.to_string()
        };
        self.push_status(format!("⚙ {name} {args}"));
        self.status_line = format!("tool: {name}");
        self.status = RunStatus::Streaming;
    }

    pub fn note_tool_result(&mut self, name: &str, preview: &str) {
        self.push_status(format!("✓ {name}: {preview}"));
    }

    pub fn scroll_up(&mut self, n: u16) {
        self.stick_to_bottom = false;
        self.scroll_from_bottom = self.scroll_from_bottom.saturating_add(n);
    }

    pub fn scroll_down(&mut self, n: u16) {
        if self.scroll_from_bottom <= n {
            self.scroll_from_bottom = 0;
            self.stick_to_bottom = true;
        } else {
            self.scroll_from_bottom -= n;
        }
    }

    pub fn clamp_scroll(&mut self, max_from_bottom: u16) {
        if self.scroll_from_bottom > max_from_bottom {
            self.scroll_from_bottom = max_from_bottom;
        }
        if self.scroll_from_bottom == 0 {
            self.stick_to_bottom = true;
        }
    }

    pub fn tick_spinner(&mut self) {
        self.spinner_frame = self.spinner_frame.wrapping_add(1);
    }

    pub fn spinner_char(&self) -> char {
        const FRAMES: &[char] = &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
        FRAMES[self.spinner_frame % FRAMES.len()]
    }

    /// Handle slash commands. Returns true if the input was consumed.
    pub fn handle_slash(&mut self, input: &str) -> bool {
        if !input.starts_with('/') {
            return false;
        }
        let parts: Vec<&str> = input.split_whitespace().collect();
        let cmd = parts.first().copied().unwrap_or("");
        match cmd {
            "/help" | "/?" => {
                self.push_status(
                    "commands:\n  /help          this text\n  /clear         clear conversation\n  /tools [on|off] toggle tools\n  /quit          exit",
                );
            }
            "/clear" => {
                self.bubbles.clear();
                self.push_status("conversation cleared.");
                self.status = RunStatus::Idle;
                self.status_line = "ready".into();
                self.stick_to_bottom = true;
                self.scroll_from_bottom = 0;
            }
            "/quit" | "/exit" => {
                self.should_quit = true;
            }
            "/tools" => {
                if parts.get(1) == Some(&"on") {
                    self.tools_enabled = true;
                } else if parts.get(1) == Some(&"off") {
                    self.tools_enabled = false;
                } else {
                    self.tools_enabled = !self.tools_enabled;
                }
                let msg = format!("tools: {}", if self.tools_enabled { "on" } else { "off" });
                self.status_line = msg.clone();
                self.push_status(msg);
            }
            _ => {
                self.push_status(format!("unknown command `{cmd}` — try /help"));
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stream_deltas_append() {
        let mut ui = UiState::new("m", "ollama", true, false);
        ui.push_user("hi");
        ui.begin_assistant();
        ui.append_delta("Hel");
        ui.append_delta("lo");
        ui.finish_stream("Hello".into());
        assert_eq!(ui.bubbles.last().unwrap().content, "Hello");
        assert_eq!(ui.status, RunStatus::Idle);
    }

    #[test]
    fn slash_clear() {
        let mut ui = UiState::new("m", "ollama", true, true);
        ui.push_user("x");
        assert!(ui.handle_slash("/clear"));
        assert!(ui.bubbles.iter().all(|b| b.role == Role::Status));
    }

    #[test]
    fn scroll_stickiness() {
        let mut ui = UiState::new("m", "ollama", true, false);
        ui.scroll_up(5);
        assert!(!ui.stick_to_bottom);
        assert_eq!(ui.scroll_from_bottom, 5);
        ui.scroll_down(5);
        assert!(ui.stick_to_bottom);
        assert_eq!(ui.scroll_from_bottom, 0);
    }
}
