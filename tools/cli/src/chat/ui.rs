//! Ratatui drawing for kode chat — patterned on rusts/kode.

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;
use tui_textarea::TextArea;
use unicode_width::UnicodeWidthStr;

use super::state::{Role, RunStatus, UiState};

pub fn draw(frame: &mut Frame, state: &mut UiState, textarea: &TextArea<'_>) {
    let area = frame.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(5),
            Constraint::Length(5),
            Constraint::Length(2),
        ])
        .split(area);

    draw_header(frame, chunks[0], state);
    draw_chat(frame, chunks[1], state);
    draw_input(frame, chunks[2], textarea, state);
    draw_footer(frame, chunks[3], state);
}

fn draw_header(frame: &mut Frame, area: Rect, state: &UiState) {
    let status = match state.status {
        RunStatus::Idle => Span::styled(" idle ", Style::default().fg(Color::Green)),
        RunStatus::Streaming => Span::styled(
            format!(" {} streaming ", state.spinner_char()),
            Style::default().fg(Color::Yellow),
        ),
        RunStatus::Error => Span::styled(" error ", Style::default().fg(Color::Red)),
    };

    let title = Line::from(vec![
        Span::styled(
            " kode ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("· "),
        Span::styled(&state.model, Style::default().fg(Color::White)),
        Span::raw(" · "),
        Span::styled(&state.provider, Style::default().fg(Color::DarkGray)),
        Span::raw("  "),
        status,
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    frame.render_widget(Paragraph::new(title), inner);
}

fn draw_chat(frame: &mut Frame, area: Rect, state: &mut UiState) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" chat ")
        .border_style(Style::default().fg(Color::DarkGray));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let width = inner.width.max(1) as usize;
    let mut lines: Vec<Line> = Vec::new();

    for bubble in &state.bubbles {
        let (label, style) = match bubble.role {
            Role::User => (
                "you",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Role::Assistant => (
                "kode",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Role::Status => (" · ", Style::default().fg(Color::DarkGray)),
        };

        lines.push(Line::from(Span::styled(label.to_string(), style)));
        for raw in bubble.content.split('\n') {
            for wrapped in wrap_line(raw, width.saturating_sub(2)) {
                let content_style = match bubble.role {
                    Role::Status => Style::default().fg(Color::DarkGray),
                    _ => Style::default().fg(Color::White),
                };
                lines.push(Line::from(Span::styled(
                    format!("  {wrapped}"),
                    content_style,
                )));
            }
        }
        lines.push(Line::from(""));
    }

    if state.status == RunStatus::Streaming {
        if let Some(last) = lines.last_mut() {
            let mut spans = last.spans.clone();
            spans.push(Span::styled("▌", Style::default().fg(Color::Yellow)));
            *last = Line::from(spans);
        }
    }

    let total = lines.len();
    let height = inner.height as usize;
    let max_from_bottom = total.saturating_sub(height) as u16;
    state.clamp_scroll(max_from_bottom);

    let end = total.saturating_sub(state.scroll_from_bottom as usize);
    let start = end.saturating_sub(height);
    let visible: Vec<Line> = lines.into_iter().skip(start).take(height).collect();

    frame.render_widget(
        Paragraph::new(Text::from(visible)).wrap(Wrap { trim: false }),
        inner,
    );
}

fn draw_input(frame: &mut Frame, area: Rect, textarea: &TextArea<'_>, state: &UiState) {
    let title = if state.status == RunStatus::Streaming {
        " input (busy — wait for reply) "
    } else {
        " input · Enter send · Ctrl+J newline "
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(Style::default().fg(Color::Cyan));
    let mut ta = textarea.clone();
    ta.set_block(block);
    ta.set_cursor_line_style(Style::default());
    ta.set_style(Style::default().fg(Color::White));
    frame.render_widget(&ta, area);
}

fn draw_footer(frame: &mut Frame, area: Rect, state: &UiState) {
    let hints = "/help · /tools · Ctrl+L clear · Ctrl+C quit";
    let line1 = Line::from(vec![
        Span::styled(" status ", Style::default().fg(Color::DarkGray)),
        Span::raw(truncate(
            &state.status_line,
            area.width.saturating_sub(10) as usize,
        )),
    ]);
    let line2 = Line::from(vec![
        Span::styled(
            format!(
                " tools:{} index:{} ",
                if state.tools_enabled { "on" } else { "off" },
                if state.has_index { "yes" } else { "no" }
            ),
            Style::default().fg(Color::DarkGray),
        ),
        Span::raw(hints),
    ]);
    frame.render_widget(Paragraph::new(Text::from(vec![line1, line2])), area);
}

fn wrap_line(s: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![s.to_string()];
    }
    if s.is_empty() {
        return vec![String::new()];
    }
    let mut out = Vec::new();
    let mut current = String::new();
    for ch in s.chars() {
        let w = UnicodeWidthStr::width(ch.encode_utf8(&mut [0; 4]));
        let cur_w = UnicodeWidthStr::width(current.as_str());
        if cur_w + w > width && !current.is_empty() {
            out.push(std::mem::take(&mut current));
        }
        current.push(ch);
    }
    if !current.is_empty() || out.is_empty() {
        out.push(current);
    }
    out
}

fn truncate(s: &str, max: usize) -> String {
    if max == 0 {
        return String::new();
    }
    if UnicodeWidthStr::width(s) <= max {
        return s.to_string();
    }
    let mut out = String::new();
    for ch in s.chars() {
        let next = format!("{out}{ch}");
        if UnicodeWidthStr::width(next.as_str()) > max.saturating_sub(1) {
            break;
        }
        out.push(ch);
    }
    out.push('…');
    out
}
