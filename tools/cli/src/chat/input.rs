use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Wrap};
use ratatui::Frame;

use super::theme::Theme;

pub struct InputBuffer {
    pub content: String,
    pub cursor: usize,
    pub history: Vec<String>,
    pub history_pos: isize,
    pub disabled: bool,
}

impl InputBuffer {
    pub fn new() -> Self {
        Self {
            content: String::new(),
            cursor: 0,
            history: Vec::new(),
            history_pos: -1,
            disabled: false,
        }
    }

    pub fn insert(&mut self, c: char) {
        self.content.insert(self.cursor, c);
        self.cursor += 1;
    }

    pub fn backspace(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
            self.content.remove(self.cursor);
        }
    }

    pub fn delete(&mut self) {
        if self.cursor < self.content.len() {
            self.content.remove(self.cursor);
        }
    }

    pub fn cursor_left(&mut self) {
        self.cursor = self.cursor.saturating_sub(1);
    }

    pub fn cursor_right(&mut self) {
        if self.cursor < self.content.len() {
            self.cursor += 1;
        }
    }

    pub fn cursor_home(&mut self) {
        self.cursor = 0;
    }

    pub fn cursor_end(&mut self) {
        self.cursor = self.content.len();
    }

    pub fn history_up(&mut self) {
        if self.history.is_empty() {
            return;
        }
        if self.history_pos == -1 {
            self.history_pos = self.history.len() as isize - 1;
        } else {
            self.history_pos = (self.history_pos - 1).max(0);
        }
        self.content = self.history[self.history_pos as usize].clone();
        self.cursor = self.content.len();
    }

    pub fn history_down(&mut self) {
        if self.history_pos == -1 {
            return;
        }
        if self.history_pos as usize >= self.history.len() - 1 {
            self.history_pos = -1;
            self.content.clear();
            self.cursor = 0;
        } else {
            self.history_pos += 1;
            self.content = self.history[self.history_pos as usize].clone();
            self.cursor = self.content.len();
        }
    }

    pub fn submit(&mut self) -> String {
        let content = self.content.clone();
        if !content.trim().is_empty() {
            self.history.push(content.clone());
        }
        self.content.clear();
        self.cursor = 0;
        self.history_pos = -1;
        content
    }

    pub fn clear(&mut self) {
        self.content.clear();
        self.cursor = 0;
    }

    pub fn delete_word_backward(&mut self) {
        if self.cursor == 0 {
            return;
        }
        let before = &self.content[..self.cursor];
        let trimmed = before.trim_end();
        let word_start = trimmed.rfind(' ').map(|i| i + 1).unwrap_or(0);
        let len = self.cursor - word_start;
        for _ in 0..len {
            self.cursor -= 1;
            self.content.remove(self.cursor);
        }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let prompt = Span::styled(
            "›",
            Style::default()
                .fg(theme.muted)
                .add_modifier(Modifier::BOLD),
        );
        let (content_span, cursor_offset) = if self.disabled {
            (
                Span::styled(" ⏳ Waiting...", Style::default().fg(theme.muted)),
                0,
            )
        } else if self.content.is_empty() {
            (
                Span::styled(" Ask kode...", Style::default().fg(theme.muted)),
                0,
            )
        } else {
            (Span::raw(format!(" {}", self.content)), self.cursor as u16)
        };
        let line = Line::from(vec![Span::raw("  "), prompt, content_span]);
        frame.render_widget(Paragraph::new(line).wrap(Wrap { trim: false }), area);

        if !self.disabled && !self.content.is_empty() {
            let cursor_x = area.x + 4 + cursor_offset;
            frame.set_cursor_position((cursor_x, area.y));
        } else if !self.disabled {
            let cursor_x = area.x + 4;
            frame.set_cursor_position((cursor_x, area.y));
        }
    }
}

impl Default for InputBuffer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn buf() -> InputBuffer {
        InputBuffer::new()
    }

    #[test]
    fn test_new_buffer_empty() {
        let b = buf();
        assert_eq!(b.content, "");
        assert_eq!(b.cursor, 0);
        assert!(b.history.is_empty());
    }

    #[test]
    fn test_insert_character() {
        let mut b = buf();
        b.insert('a');
        assert_eq!(b.content, "a");
        assert_eq!(b.cursor, 1);
    }

    #[test]
    fn test_insert_at_cursor_position() {
        let mut b = buf();
        b.content = "ac".into();
        b.cursor = 1;
        b.insert('b');
        assert_eq!(b.content, "abc");
        assert_eq!(b.cursor, 2);
    }

    #[test]
    fn test_backspace_removes_before_cursor() {
        let mut b = buf();
        b.content = "ab".into();
        b.cursor = 2;
        b.backspace();
        assert_eq!(b.content, "a");
        assert_eq!(b.cursor, 1);
    }

    #[test]
    fn test_backspace_at_start_is_noop() {
        let mut b = buf();
        b.content = "a".into();
        b.cursor = 0;
        b.backspace();
        assert_eq!(b.content, "a");
    }

    #[test]
    fn test_delete_removes_at_cursor() {
        let mut b = buf();
        b.content = "ab".into();
        b.cursor = 0;
        b.delete();
        assert_eq!(b.content, "b");
        assert_eq!(b.cursor, 0);
    }

    #[test]
    fn test_delete_at_end_is_noop() {
        let mut b = buf();
        b.content = "a".into();
        b.cursor = 1;
        b.delete();
        assert_eq!(b.content, "a");
    }

    #[test]
    fn test_cursor_left() {
        let mut b = buf();
        b.content = "ab".into();
        b.cursor = 2;
        b.cursor_left();
        assert_eq!(b.cursor, 1);
    }

    #[test]
    fn test_cursor_left_at_start() {
        let mut b = buf();
        b.cursor = 0;
        b.cursor_left();
        assert_eq!(b.cursor, 0);
    }

    #[test]
    fn test_cursor_right() {
        let mut b = buf();
        b.content = "ab".into();
        b.cursor = 0;
        b.cursor_right();
        assert_eq!(b.cursor, 1);
    }

    #[test]
    fn test_cursor_right_at_end() {
        let mut b = buf();
        b.content = "ab".into();
        b.cursor = 2;
        b.cursor_right();
        assert_eq!(b.cursor, 2);
    }

    #[test]
    fn test_cursor_home() {
        let mut b = buf();
        b.content = "abc".into();
        b.cursor = 2;
        b.cursor_home();
        assert_eq!(b.cursor, 0);
    }

    #[test]
    fn test_cursor_end() {
        let mut b = buf();
        b.content = "abc".into();
        b.cursor = 0;
        b.cursor_end();
        assert_eq!(b.cursor, 3);
    }

    #[test]
    fn test_history_up_empty_is_noop() {
        let mut b = buf();
        b.history_up();
        assert_eq!(b.content, "");
    }

    #[test]
    fn test_history_up_down() {
        let mut b = buf();
        b.history.push("hello".into());
        b.history.push("world".into());
        b.history_up();
        assert_eq!(b.content, "world");
        b.history_up();
        assert_eq!(b.content, "hello");
        b.history_down();
        assert_eq!(b.content, "world");
    }

    #[test]
    fn test_history_down_past_end_clears() {
        let mut b = buf();
        b.history.push("hello".into());
        b.history_up();
        assert_eq!(b.content, "hello");
        b.history_down();
        assert_eq!(b.content, "");
    }

    #[test]
    fn test_submit_adds_to_history() {
        let mut b = buf();
        b.content = "query".into();
        b.cursor = 5;
        let result = b.submit();
        assert_eq!(result, "query");
        assert_eq!(b.history.len(), 1);
        assert_eq!(b.content, "");
    }

    #[test]
    fn test_submit_whitespace_not_added_to_history() {
        let mut b = buf();
        b.content = "   ".into();
        b.submit();
        assert!(b.history.is_empty());
    }

    #[test]
    fn test_clear() {
        let mut b = buf();
        b.content = "text".into();
        b.cursor = 4;
        b.clear();
        assert_eq!(b.content, "");
        assert_eq!(b.cursor, 0);
    }

    #[test]
    fn test_delete_word_backward() {
        let mut b = buf();
        b.content = "hello world".into();
        b.cursor = 11;
        b.delete_word_backward();
        assert_eq!(b.content, "hello ");
        assert_eq!(b.cursor, 6);
    }

    #[test]
    fn test_delete_word_backward_at_start() {
        let mut b = buf();
        b.content = "hello".into();
        b.cursor = 0;
        b.delete_word_backward();
        assert_eq!(b.content, "hello");
    }

    #[test]
    fn test_delete_word_backward_single_word() {
        let mut b = buf();
        b.content = "hello".into();
        b.cursor = 5;
        b.delete_word_backward();
        assert_eq!(b.content, "");
        assert_eq!(b.cursor, 0);
    }
}
