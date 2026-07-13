//! Async fullscreen chat loop — patterned on rusts/kode.

use std::io::{self, Stdout};
use std::sync::Arc;
use std::time::Duration;

use crossterm::event::{Event, EventStream, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use futures_util::StreamExt;
use kode_agent::{Agent, AgentEvent};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use tokio::sync::{mpsc, Mutex};
use tui_textarea::{Input, Key, TextArea};

use super::state::{RunStatus, UiState};
use super::ui;

struct TerminalGuard {
    terminal: Terminal<CrosstermBackend<Stdout>>,
}

impl TerminalGuard {
    fn new() -> Result<Self, std::io::Error> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;
        Ok(Self { terminal })
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(self.terminal.backend_mut(), LeaveAlternateScreen);
        let _ = self.terminal.show_cursor();
    }
}

/// Run the interactive chat TUI with a shared agent.
pub async fn run(agent: Arc<Mutex<Agent>>, model: String, provider: String) -> Result<(), String> {
    let mut guard = TerminalGuard::new().map_err(|e| e.to_string())?;

    let (tools_enabled, has_index) = {
        let a = agent.lock().await;
        (a.tools_enabled(), a.has_symbol_index())
    };

    let mut state = UiState::new(&model, &provider, tools_enabled, has_index);
    let mut textarea = TextArea::default();
    textarea
        .set_placeholder_text("Message kode…  (/help · tools: list_dir grep read_file symbols)");

    let mut events = EventStream::new();
    let (stream_tx, mut stream_rx) = mpsc::unbounded_channel::<AgentEvent>();
    let mut tick = tokio::time::interval(Duration::from_millis(80));
    let mut streaming = false;

    loop {
        guard
            .terminal
            .draw(|f| ui::draw(f, &mut state, &textarea))
            .map_err(|e| e.to_string())?;

        if state.should_quit {
            break;
        }

        tokio::select! {
            _ = tick.tick() => {
                if state.status == RunStatus::Streaming {
                    state.tick_spinner();
                }
            }
            maybe_ev = events.next() => {
                let Some(ev) = maybe_ev else { break; };
                let ev = ev.map_err(|e| e.to_string())?;
                if let Event::Key(key) = ev {
                    if key.kind != KeyEventKind::Press {
                        continue;
                    }
                    handle_key(
                        key,
                        &mut state,
                        &mut textarea,
                        &agent,
                        &stream_tx,
                        &mut streaming,
                    ).await;
                }
            }
            maybe_stream = stream_rx.recv() => {
                match maybe_stream {
                    Some(AgentEvent::Delta(d)) => state.append_delta(&d),
                    Some(AgentEvent::ToolStart { name, args }) => {
                        state.note_tool_start(&name, &args);
                    }
                    Some(AgentEvent::ToolResult { name, preview }) => {
                        state.note_tool_result(&name, &preview);
                    }
                    Some(AgentEvent::Done(full)) => {
                        streaming = false;
                        if full.is_empty() {
                            state.fail_stream("empty response".into());
                        } else {
                            state.finish_stream(full);
                        }
                    }
                    Some(AgentEvent::Error(e)) => {
                        streaming = false;
                        state.fail_stream(e);
                    }
                    None => {}
                }
            }
        }
    }

    Ok(())
}

async fn handle_key(
    key: KeyEvent,
    state: &mut UiState,
    textarea: &mut TextArea<'_>,
    agent: &Arc<Mutex<Agent>>,
    stream_tx: &mpsc::UnboundedSender<AgentEvent>,
    streaming: &mut bool,
) {
    if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
        state.should_quit = true;
        return;
    }

    match (key.code, key.modifiers) {
        (KeyCode::Enter, KeyModifiers::NONE) => {
            if *streaming || state.status == RunStatus::Streaming {
                state.status_line = "already streaming…".into();
                return;
            }
            let text = textarea.lines().join("\n");
            let text = text.trim().to_string();
            if text.is_empty() {
                return;
            }
            *textarea = TextArea::default();
            textarea.set_placeholder_text(
                "Message kode…  (/help · tools: list_dir grep read_file symbols)",
            );

            if state.handle_slash(&text) {
                let mut a = agent.lock().await;
                a.set_tools_enabled(state.tools_enabled);
                if text.starts_with("/clear") {
                    a.clear_history();
                }
                return;
            }

            state.push_user(&text);
            state.begin_assistant();
            *streaming = true;

            let agent = Arc::clone(agent);
            let tx = stream_tx.clone();
            let tools_enabled = state.tools_enabled;
            tokio::spawn(async move {
                let mut a = agent.lock().await;
                a.set_tools_enabled(tools_enabled);
                if let Err(e) = a.run_with_tx(&text, Some(tx.clone())).await {
                    let _ = tx.send(AgentEvent::Error(e.to_string()));
                }
            });
            return;
        }
        (KeyCode::Char('j'), m) if m.contains(KeyModifiers::CONTROL) => {
            textarea.input(Input {
                key: Key::Enter,
                ctrl: false,
                alt: false,
                shift: false,
            });
            return;
        }
        (KeyCode::Char('l'), m) if m.contains(KeyModifiers::CONTROL) => {
            let _ = state.handle_slash("/clear");
            agent.lock().await.clear_history();
            return;
        }
        (KeyCode::PageUp, _) => {
            state.scroll_up(5);
            return;
        }
        (KeyCode::PageDown, _) => {
            state.scroll_down(5);
            return;
        }
        (KeyCode::Up, m) if m.contains(KeyModifiers::CONTROL) => {
            state.scroll_up(1);
            return;
        }
        (KeyCode::Down, m) if m.contains(KeyModifiers::CONTROL) => {
            state.scroll_down(1);
            return;
        }
        _ => {}
    }

    if !*streaming && state.status != RunStatus::Streaming {
        textarea.input(crossterm_to_textarea(key));
    }
}

fn crossterm_to_textarea(key: KeyEvent) -> Input {
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    let alt = key.modifiers.contains(KeyModifiers::ALT);
    let shift = key.modifiers.contains(KeyModifiers::SHIFT);
    let k = match key.code {
        KeyCode::Char(c) => Key::Char(c),
        KeyCode::Backspace => Key::Backspace,
        KeyCode::Enter => Key::Enter,
        KeyCode::Left => Key::Left,
        KeyCode::Right => Key::Right,
        KeyCode::Up => Key::Up,
        KeyCode::Down => Key::Down,
        KeyCode::Home => Key::Home,
        KeyCode::End => Key::End,
        KeyCode::PageUp => Key::PageUp,
        KeyCode::PageDown => Key::PageDown,
        KeyCode::Tab => Key::Tab,
        KeyCode::Delete => Key::Delete,
        KeyCode::Esc => Key::Esc,
        _ => Key::Null,
    };
    Input {
        key: k,
        ctrl,
        alt,
        shift,
    }
}
