use crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers,
};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap};
use ratatui::Frame;
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io::stdout;
use std::sync::OnceLock;
use std::time::Duration;
use std::time::Instant;
use tokio::sync::mpsc;

use super::engine::StreamEvent;
use super::input::InputBuffer;
use super::markdown::render_markdown;
use super::theme::Theme;
use super::{ChatMessage, ChatSession};

static PANIC_HOOK: OnceLock<()> = OnceLock::new();

struct TerminalGuard;

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(std::io::stdout(), LeaveAlternateScreen, DisableMouseCapture);
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Tab {
    Explore,
    Sources,
    Graph,
    Sessions,
    Settings,
}

impl Tab {
    fn label(self) -> &'static str {
        match self {
            Tab::Explore => "Explore",
            Tab::Sources => "Sources",
            Tab::Graph => "Graph",
            Tab::Sessions => "Sessions",
            Tab::Settings => "Settings",
        }
    }

    fn next(self) -> Self {
        match self {
            Tab::Explore => Tab::Sources,
            Tab::Sources => Tab::Graph,
            Tab::Graph => Tab::Sessions,
            Tab::Sessions => Tab::Settings,
            Tab::Settings => Tab::Explore,
        }
    }

    fn prev(self) -> Self {
        match self {
            Tab::Explore => Tab::Settings,
            Tab::Sources => Tab::Explore,
            Tab::Graph => Tab::Sources,
            Tab::Sessions => Tab::Graph,
            Tab::Settings => Tab::Sessions,
        }
    }

    fn from_key(c: char) -> Option<Self> {
        match c {
            '1' => Some(Tab::Explore),
            '2' => Some(Tab::Sources),
            '3' => Some(Tab::Graph),
            '4' => Some(Tab::Sessions),
            '5' => Some(Tab::Settings),
            _ => None,
        }
    }
}

pub struct ChatTui {
    theme: Theme,
    active_tab: Tab,
    scroll_offset: usize,
    scroll_target: usize,
    content_height: usize,
    visual_overflow: usize,
    scroll_locked: bool,
    input_buffer: InputBuffer,
    partial_assistant_content: String,
    streaming_active: bool,
    stream_rx: Option<mpsc::Receiver<StreamEvent>>,
    repo_name: String,
    model_name: String,
    spinner_start: Option<Instant>,
    frame_count: u64,
    transition_frame: u8,
}

impl ChatTui {
    pub fn new(theme: Theme, repo_name: String, model_name: String) -> Self {
        Self {
            theme,
            active_tab: Tab::Explore,
            scroll_offset: 0,
            scroll_target: 0,
            content_height: 0,
            visual_overflow: 0,
            scroll_locked: false,
            input_buffer: InputBuffer::new(),
            partial_assistant_content: String::new(),
            streaming_active: false,
            stream_rx: None,
            repo_name,
            model_name,
            spinner_start: None,
            frame_count: 0,
            transition_frame: 0,
        }
    }

    pub fn run(&mut self, session: &mut ChatSession) -> Result<(), super::ChatError> {
        PANIC_HOOK.get_or_init(|| {
            let prev = std::panic::take_hook();
            std::panic::set_hook(Box::new(move |info| {
                let _ = disable_raw_mode();
                let _ = execute!(std::io::stdout(), LeaveAlternateScreen, DisableMouseCapture);
                prev(info);
            }));
        });

        enable_raw_mode().map_err(super::ChatError::Io)?;
        let mut stdout = stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture).map_err(super::ChatError::Io)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal =
            Terminal::new(backend).map_err(|e| super::ChatError::Io(std::io::Error::other(e)))?;

        let _guard = TerminalGuard;

        let result = self.run_loop(&mut terminal, session);

        match &result {
            Ok(()) => {
                let _ = disable_raw_mode();
                let _ = execute!(std::io::stdout(), LeaveAlternateScreen, DisableMouseCapture);
            }
            Err(_) => {
                let _ = disable_raw_mode();
                let _ = execute!(std::io::stdout(), LeaveAlternateScreen, DisableMouseCapture);
            }
        }
        result
    }

    fn run_loop(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
        session: &mut ChatSession,
    ) -> Result<(), super::ChatError> {
        loop {
            self.poll_stream(session);

            terminal
                .draw(|f| self.render(f, session))
                .map_err(|e| super::ChatError::Io(std::io::Error::other(e)))?;

            self.frame_count += 1;

            if event::poll(Duration::from_millis(50)).map_err(super::ChatError::Io)? {
                match event::read().map_err(super::ChatError::Io)? {
                    Event::Key(key) if key.kind == KeyEventKind::Press => {
                        if !self.handle_key(key, session) {
                            break;
                        }
                    }
                    Event::Mouse(mouse)
                        if self.active_tab == Tab::Explore
                            && (mouse.kind == crossterm::event::MouseEventKind::ScrollDown
                                || mouse.kind == crossterm::event::MouseEventKind::ScrollUp) =>
                    {
                        let delta = if mouse.kind == crossterm::event::MouseEventKind::ScrollDown {
                            8
                        } else {
                            -8i32
                        };
                        self.scroll_target =
                            (self.scroll_offset as i32).saturating_add(delta).max(0) as usize;
                        self.scroll_locked = true;
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    }

    fn handle_key(&mut self, key: crossterm::event::KeyEvent, session: &mut ChatSession) -> bool {
        // Tab navigation always works
        match key.code {
            KeyCode::Tab => {
                self.active_tab = self.active_tab.next();
                return true;
            }
            KeyCode::BackTab => {
                self.active_tab = self.active_tab.prev();
                return true;
            }
            _ => {}
        }

        // Number keys switch tabs when not streaming
        if !self.streaming_active {
            if let KeyCode::Char(c) = key.code {
                if key.modifiers.is_empty() {
                    if let Some(tab) = Tab::from_key(c) {
                        self.active_tab = tab;
                        return true;
                    }
                }
            }
        }

        // Chat/input keys only work on Explore tab
        if self.active_tab != Tab::Explore {
            return true;
        }

        match key.code {
            KeyCode::Esc | KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if self.streaming_active {
                    self.streaming_active = false;
                    self.partial_assistant_content.clear();
                    self.stream_rx = None;
                    session.is_streaming = false;
                    self.input_buffer.disabled = false;
                    return true;
                }
                return false;
            }
            KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                return false;
            }
            KeyCode::Char('l') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.scroll_locked = !self.scroll_locked;
                if !self.scroll_locked {
                    self.scroll_target = self.content_height.saturating_sub(1);
                }
                return true;
            }
            KeyCode::Enter if !self.streaming_active => {
                if key.modifiers.contains(KeyModifiers::SHIFT) {
                    self.input_buffer.insert('\n');
                } else {
                    let msg = self.input_buffer.submit();
                    if !msg.trim().is_empty() {
                        let user_msg = ChatMessage {
                            role: "user".into(),
                            content: msg.clone(),
                        };
                        session.messages.push(user_msg);
                        self.scroll_target = 0;
                        self.scroll_offset = 0;
                        self.content_height = 0;
                        self.visual_overflow = 0;
                        self.scroll_locked = false;
                        self.streaming_active = true;
                        self.spinner_start = Some(Instant::now());
                        self.input_buffer.disabled = true;
                        self.partial_assistant_content.clear();

                        if session.messages.len() == 1 {
                            self.transition_frame = 8;
                        }

                        let result = session.ask_stream(&msg);
                        match result {
                            Ok(rx) => {
                                self.stream_rx = Some(rx);
                            }
                            Err(e) => {
                                session.messages.push(ChatMessage {
                                    role: "system".into(),
                                    content: format!("error: {}", e),
                                });
                                self.streaming_active = false;
                                self.input_buffer.disabled = false;
                            }
                        }
                    }
                }
            }
            KeyCode::Char(c) if !self.streaming_active && key.modifiers.is_empty() => {
                self.input_buffer.insert(c);
            }
            KeyCode::Backspace if !self.streaming_active => {
                self.input_buffer.backspace();
            }
            KeyCode::Delete if !self.streaming_active => {
                self.input_buffer.delete();
            }
            KeyCode::Left if !self.streaming_active => {
                self.input_buffer.cursor_left();
            }
            KeyCode::Right if !self.streaming_active => {
                self.input_buffer.cursor_right();
            }
            KeyCode::Home => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    self.scroll_target = 0;
                    self.scroll_locked = true;
                } else if !self.streaming_active {
                    self.input_buffer.cursor_home();
                }
            }
            KeyCode::End => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    self.scroll_offset = self.visual_overflow;
                    self.scroll_target = self.visual_overflow;
                    self.scroll_locked = false;
                } else if !self.streaming_active {
                    self.input_buffer.cursor_end();
                }
            }
            KeyCode::PageUp => {
                self.scroll_target = self
                    .scroll_target
                    .saturating_sub(self.content_height.saturating_sub(1).max(5));
                self.scroll_locked = true;
            }
            KeyCode::PageDown => {
                self.scroll_target = self
                    .scroll_target
                    .saturating_add(self.content_height.saturating_sub(1).max(5));
                if self.scroll_target >= self.visual_overflow {
                    self.scroll_target = self.visual_overflow;
                    self.scroll_locked = false;
                }
            }
            KeyCode::Up if !self.streaming_active => {
                self.input_buffer.history_up();
            }
            KeyCode::Down if !self.streaming_active => {
                self.input_buffer.history_down();
            }
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if !self.streaming_active {
                    self.input_buffer.clear();
                }
            }
            KeyCode::Char('w')
                if key.modifiers.contains(KeyModifiers::CONTROL) && !self.streaming_active =>
            {
                self.input_buffer.delete_word_backward();
            }
            _ => {}
        }
        true
    }

    fn poll_stream(&mut self, session: &mut ChatSession) {
        let rx = match &mut self.stream_rx {
            Some(rx) => rx,
            None => return,
        };

        loop {
            match rx.try_recv() {
                Ok(StreamEvent::Token(t)) => {
                    self.partial_assistant_content.push_str(&t);
                }
                Ok(StreamEvent::Done) => {
                    let content = std::mem::take(&mut self.partial_assistant_content);
                    session.messages.push(ChatMessage {
                        role: "assistant".into(),
                        content,
                    });
                    self.streaming_active = false;
                    self.input_buffer.disabled = false;
                    self.stream_rx = None;
                    session.is_streaming = false;
                    break;
                }
                Err(mpsc::error::TryRecvError::Empty) => break,
                Err(mpsc::error::TryRecvError::Disconnected) => {
                    self.streaming_active = false;
                    self.input_buffer.disabled = false;
                    self.stream_rx = None;
                    session.is_streaming = false;
                    break;
                }
            }
        }
    }

    // ── Render ──────────────────────────────────────────────────────

    fn render(&mut self, frame: &mut Frame, session: &ChatSession) {
        let area = frame.area();

        let [header_area, main_area, footer_area] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .areas(area);

        self.render_header(frame, header_area);

        match self.active_tab {
            Tab::Explore => {
                let line_count = self.input_buffer.content.lines().count().max(1) as u16;
                let composer_h = line_count.min(10);
                let input_h = composer_h + 1;

                let [screen_area, input_area] =
                    Layout::vertical([Constraint::Min(1), Constraint::Length(input_h)])
                        .margin(1)
                        .areas(main_area);

                if self.transition_frame > 0 {
                    self.render_transition(frame, screen_area);
                    self.transition_frame -= 1;
                } else if session.messages.is_empty() {
                    self.render_hero(frame, screen_area);
                } else {
                    self.render_chat(frame, screen_area, session);
                }
                self.render_input_area(frame, input_area);
            }
            _ => {
                self.render_placeholder(frame, main_area);
            }
        }

        self.render_footer(frame, footer_area, session);
    }

    // ── Header ───────────────────────────────────────────────────────

    fn render_header(&self, frame: &mut Frame, area: Rect) {
        let all_tabs = [
            Tab::Explore,
            Tab::Sources,
            Tab::Graph,
            Tab::Sessions,
            Tab::Settings,
        ];
        let mut spans: Vec<Span> = Vec::new();

        spans.push(Span::styled(
            "kode",
            Style::default().add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::raw("  "));

        for tab in all_tabs {
            let style = if tab == self.active_tab {
                Style::default()
                    .fg(self.theme.accent)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(self.theme.muted)
            };
            spans.push(Span::styled(format!(" {} ", tab.label()), style));
        }

        let status = if self.streaming_active {
            "streaming"
        } else {
            "ready"
        };
        let right = format!(" {} {}", self.model_name, status);
        let right_width = right.len() as u16;
        let padding = area.width.saturating_sub(
            spans.iter().map(|s| s.content.len() as u16).sum::<u16>() + right_width + 2,
        );

        if padding > 0 {
            spans.push(Span::raw(" ".repeat(padding as usize)));
        } else {
            spans.push(Span::raw(" "));
        }
        spans.push(Span::styled(right, Style::default().fg(self.theme.muted)));

        frame.render_widget(Paragraph::new(Line::from(spans)), area);
    }

    // ── Footer ───────────────────────────────────────────────────────

    fn render_footer(&self, frame: &mut Frame, area: Rect, session: &ChatSession) {
        let msg_count = session.messages.len();
        let stats = if msg_count > 0 {
            format!("{} messages", msg_count)
        } else {
            format!("{} · {}", self.repo_name, self.model_name)
        };
        let status = if self.streaming_active {
            "streaming"
        } else {
            "ready"
        };

        let stats_width = stats.len() as u16;
        let status_width = status.len() as u16;
        let padding = area.width.saturating_sub(stats_width + status_width + 2);

        let spans = vec![
            Span::styled(stats, Style::default().fg(self.theme.muted)),
            Span::raw(" ".repeat(padding.max(1) as usize)),
            Span::styled(status, Style::default().fg(self.theme.muted)),
        ];
        frame.render_widget(Paragraph::new(Line::from(spans)), area);
    }

    // ── Transition ───────────────────────────────────────────────────

    fn render_transition(&self, frame: &mut Frame, area: Rect) {
        let spinner = match self.spinner_start {
            Some(start) => {
                let elapsed = start.elapsed().as_millis() as usize;
                let frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
                let idx = (elapsed / 80) % frames.len();
                frames[idx]
            }
            None => "▶",
        };
        let content = vec![
            Line::from(""),
            Line::from(Span::styled(
                format!(" {} starting...", spinner),
                Style::default().fg(self.theme.muted),
            )),
            Line::from(""),
        ];

        let [_, text_area, _] = Layout::vertical([
            Constraint::Fill(1),
            Constraint::Length(3),
            Constraint::Fill(1),
        ])
        .areas(area);
        let [_, centered_area, _] = Layout::horizontal([
            Constraint::Fill(1),
            Constraint::Min(20),
            Constraint::Fill(1),
        ])
        .areas(text_area);

        frame.render_widget(Paragraph::new(content), centered_area);
    }

    // ── Hero ─────────────────────────────────────────────────────────

    fn render_hero(&self, frame: &mut Frame, area: Rect) {
        let [left, right] =
            Layout::horizontal([Constraint::Percentage(60), Constraint::Percentage(40)])
                .areas(area);

        // ── Left column: title, description, capabilities, action ──
        let caps = [
            "Repository Analysis",
            "Documentation Search",
            "Architecture Mapping",
            "Knowledge Graph",
            "Persistent Memory",
        ];
        let mut left_lines = vec![
            Line::from(""),
            Line::from(Span::styled(
                "kode_",
                Style::default().add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled("software intelligence", Style::default())),
            Line::from(""),
            Line::from(Span::styled(
                "Explore repositories, documentation, and architecture.",
                Style::default().fg(self.theme.muted),
            )),
            Line::from(""),
        ];
        for cap in &caps {
            left_lines.push(Line::from(Span::styled(
                format!("  {}", cap),
                Style::default().fg(self.theme.muted),
            )));
        }
        left_lines.push(Line::from(""));
        left_lines.push(Line::from(Span::styled(
            "  ▶ Start Research",
            Style::default().add_modifier(Modifier::BOLD),
        )));
        left_lines.push(Line::from(""));

        // ── Right column: ASCII graph ──
        let graph_lines = vec![
            Line::from(""),
            Line::from(Span::styled("       kode", Style::default())),
            Line::from(Span::styled("      /    \\", Style::default())),
            Line::from(Span::styled("  repos   docs", Style::default())),
            Line::from(Span::styled("    |       |", Style::default())),
            Line::from(Span::styled("  code ── graph", Style::default())),
            Line::from(Span::styled("       \\", Style::default())),
            Line::from(Span::styled("      symbols", Style::default())),
        ];

        let left_par = Paragraph::new(left_lines).wrap(Wrap { trim: false });
        let right_par = Paragraph::new(graph_lines);

        frame.render_widget(left_par, left);
        frame.render_widget(right_par, right);
    }

    // ── Placeholder (non-Explore tabs) ───────────────────────────────

    fn render_placeholder(&self, frame: &mut Frame, area: Rect) {
        let label = self.active_tab.label();
        let content = vec![
            Line::from(""),
            Line::from(Span::styled(
                format!("  {} — under construction", label),
                Style::default().fg(self.theme.muted),
            )),
        ];

        let [_, text_area, _] = Layout::vertical([
            Constraint::Fill(1),
            Constraint::Length(1),
            Constraint::Fill(1),
        ])
        .areas(area);

        frame.render_widget(Paragraph::new(content), text_area);
    }

    // ── Chat ─────────────────────────────────────────────────────────

    fn render_chat(&mut self, frame: &mut Frame, area: Rect, session: &ChatSession) {
        let muted = Style::default().fg(self.theme.muted);

        let mut lines: Vec<Line> = Vec::new();
        lines.push(Line::from(""));

        for (i, msg) in session.messages.iter().enumerate() {
            let is_user = msg.role == "user";
            let prefix_char = if is_user { "›" } else { "•" };
            let prefix_style = if is_user {
                muted.add_modifier(Modifier::BOLD)
            } else {
                muted
            };

            let content_lines = render_markdown(&msg.content, &self.theme);
            let mut first = true;
            for content_line in &content_lines {
                let pfx = if first { prefix_char } else { " " };
                first = false;
                let mut spans = vec![
                    Span::raw("  "),
                    Span::styled(pfx, prefix_style),
                    Span::raw(" "),
                ];
                spans.extend(content_line.spans.iter().cloned());
                lines.push(Line::from(spans));
            }
            lines.push(Line::from(""));

            // Turn separator after each assistant message (not last)
            if msg.role == "assistant" && i + 1 < session.messages.len() {
                let sep_width = area.width.saturating_sub(4).max(3) as usize;
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled("─".repeat(sep_width), muted),
                ]));
                lines.push(Line::from(""));
            }
        }

        // Streaming partial assistant response
        if self.streaming_active {
            let spinner = match self.spinner_start {
                Some(start) => {
                    let elapsed = start.elapsed().as_millis() as usize;
                    let frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
                    let idx = (elapsed / 80) % frames.len();
                    frames[idx]
                }
                None => "•",
            };

            if self.partial_assistant_content.is_empty() {
                let prefix = Span::styled(spinner, muted);
                let text = Span::styled(" thinking...", muted);
                lines.push(Line::from(vec![Span::raw("  "), prefix, text]));
            } else {
                let stream_lines = render_markdown(&self.partial_assistant_content, &self.theme);
                for (j, content_line) in stream_lines.iter().enumerate() {
                    let pfx = if j == 0 { spinner } else { " " };
                    let prefix = Span::styled(pfx, muted);
                    let mut spans = vec![Span::raw("  "), prefix, Span::raw(" ")];
                    spans.extend(content_line.spans.iter().cloned());
                    lines.push(Line::from(spans));
                }
            }

            let cursor_visible = self.frame_count % 6 < 3;
            if cursor_visible {
                if let Some(last) = lines.last_mut() {
                    last.spans.push(Span::styled("▊", muted));
                }
            }
        }

        self.content_height = lines.len();

        let par = Paragraph::new(lines.clone()).wrap(Wrap { trim: false });
        let visual_lines = par.line_count(area.width);
        let overflow = visual_lines.saturating_sub(area.height as usize);
        self.visual_overflow = overflow;

        if overflow == 0 {
            self.scroll_target = 0;
            self.scroll_offset = 0;
        } else if self.scroll_locked {
            self.scroll_target = self.scroll_target.min(overflow);
        } else {
            self.scroll_target = overflow;
        }

        let diff = self.scroll_offset.abs_diff(self.scroll_target);
        let step = if diff > 8 { diff / 4 } else { 1.max(diff) };
        if self.scroll_offset < self.scroll_target {
            self.scroll_offset = self
                .scroll_offset
                .saturating_add(step)
                .min(self.scroll_target);
        } else if self.scroll_offset > self.scroll_target {
            self.scroll_offset = self
                .scroll_offset
                .saturating_sub(step)
                .max(self.scroll_target);
        }

        let scrollbar_needed = overflow > 0;
        let output_area = if scrollbar_needed {
            Rect::new(area.x, area.y, area.width.saturating_sub(1), area.height)
        } else {
            area
        };

        frame.render_widget(par.scroll((self.scroll_offset as u16, 0)), output_area);

        if scrollbar_needed {
            let scrollbar_area = Rect::new(
                area.x + area.width.saturating_sub(1),
                area.y,
                1,
                area.height,
            );
            let mut state =
                ScrollbarState::new(overflow).position(self.scroll_offset.min(overflow));
            frame.render_stateful_widget(
                Scrollbar::new(ScrollbarOrientation::VerticalRight),
                scrollbar_area,
                &mut state,
            );
        }
    }

    // ── Input area ───────────────────────────────────────────────────

    fn render_input_area(&self, frame: &mut Frame, area: Rect) {
        let line_count = self.input_buffer.content.lines().count().max(1) as u16;
        let composer_h = line_count.min(10);
        let footer_h = 1u16;

        let [composer_area, footer_area] =
            Layout::vertical([Constraint::Length(composer_h), Constraint::Length(footer_h)])
                .areas(area);

        self.input_buffer.render(frame, composer_area, &self.theme);

        let footer = Span::styled(
            "Tab: navigate · Enter: send · Ctrl-C: cancel · Ctrl-D: exit",
            Style::default().fg(self.theme.muted),
        );
        frame.render_widget(Paragraph::new(Line::from(footer)), footer_area);
    }
}
