use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Tag, TagEnd};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use syntect::highlighting::ThemeSet;
use syntect::parsing::SyntaxSet;

use super::theme::Theme;

fn syntax_set() -> &'static SyntaxSet {
    static SYNTAX_SET: std::sync::OnceLock<SyntaxSet> = std::sync::OnceLock::new();
    SYNTAX_SET.get_or_init(SyntaxSet::load_defaults_newlines)
}

fn theme_set() -> &'static ThemeSet {
    static THEME_SET: std::sync::OnceLock<ThemeSet> = std::sync::OnceLock::new();
    THEME_SET.get_or_init(ThemeSet::load_defaults)
}

/// Convert markdown text to ratatui Lines. Code blocks get background fill.
/// Syntax highlighting applied for code blocks with language hint.
pub fn render_markdown(text: &str, theme: &Theme) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = Vec::new();
    let mut current_line_spans: Vec<Span<'static>> = Vec::new();
    let mut emphasis_stack: Vec<Modifier> = Vec::new();
    let mut link_active: u32 = 0;
    let mut in_code_block = false;
    let mut code_block_lang = String::new();
    let mut code_block_content = String::new();
    let mut in_blockquote = false;
    let mut blockquote_first = true;

    let push_line = |lines: &mut Vec<Line<'static>>, spans: &mut Vec<Span<'static>>| {
        if !spans.is_empty() {
            lines.push(Line::from(std::mem::take(spans)));
        } else {
            lines.push(Line::from(""));
        }
    };

    let parser = pulldown_cmark::Parser::new(text);
    for event in parser {
        match event {
            Event::Start(Tag::Paragraph) => {}
            Event::End(TagEnd::Paragraph) => {
                push_line(&mut lines, &mut current_line_spans);
            }
            Event::Start(Tag::Heading { level, .. }) => {
                let boldness = match level {
                    HeadingLevel::H1 => Modifier::BOLD | Modifier::UNDERLINED,
                    HeadingLevel::H2 => Modifier::BOLD,
                    HeadingLevel::H3 => Modifier::BOLD | Modifier::ITALIC,
                    _ => Modifier::ITALIC,
                };
                emphasis_stack.push(boldness);
            }
            Event::End(TagEnd::Heading(_)) => {
                emphasis_stack.pop();
                push_line(&mut lines, &mut current_line_spans);
            }
            Event::Start(Tag::Emphasis) => {
                emphasis_stack.push(Modifier::ITALIC);
            }
            Event::End(TagEnd::Emphasis) => {
                emphasis_stack.pop();
            }
            Event::Start(Tag::Strong) => {
                emphasis_stack.push(Modifier::BOLD);
            }
            Event::End(TagEnd::Strong) => {
                emphasis_stack.pop();
            }
            Event::Start(Tag::CodeBlock(kind)) => {
                in_code_block = true;
                code_block_lang = match kind {
                    CodeBlockKind::Fenced(lang) => lang.to_string(),
                    CodeBlockKind::Indented => String::new(),
                };
                code_block_content.clear();
            }
            Event::End(TagEnd::CodeBlock) => {
                in_code_block = false;
                let highlighted = if !code_block_lang.is_empty() {
                    highlight_code(&code_block_content, &code_block_lang, theme)
                } else {
                    plain_code_block(&code_block_content, theme)
                };
                for hl_line in highlighted {
                    lines.push(hl_line);
                }
                lines.push(Line::from(""));
                current_line_spans.clear();
            }
            Event::Start(Tag::List(_)) => {}
            Event::End(TagEnd::List(_)) => {}
            Event::Start(Tag::Item) => {
                if current_line_spans.is_empty() {
                    current_line_spans.push(Span::raw("  \u{2022} "));
                }
            }
            Event::End(TagEnd::Item) => {
                push_line(&mut lines, &mut current_line_spans);
            }
            Event::Start(Tag::Link { .. }) => {
                link_active += 1;
            }
            Event::End(TagEnd::Link) => {
                link_active = link_active.saturating_sub(1);
            }
            Event::Start(Tag::BlockQuote(_)) => {
                in_blockquote = true;
                blockquote_first = true;
            }
            Event::End(TagEnd::BlockQuote(_)) => {
                in_blockquote = false;
            }
            Event::Text(t) => {
                if in_code_block {
                    code_block_content.push_str(&t);
                    continue;
                }
                let mut style = Style::default();
                for m in &emphasis_stack {
                    style = style.add_modifier(*m);
                }
                if link_active > 0 {
                    style = style.fg(theme.accent).add_modifier(Modifier::UNDERLINED);
                }
                if in_blockquote {
                    style = style.fg(theme.muted);
                }
                let text = t.to_string();
                if in_blockquote && blockquote_first && !current_line_spans.is_empty() {
                    current_line_spans.insert(0, Span::styled("> ", style));
                    blockquote_first = false;
                }
                current_line_spans.push(Span::styled(text, style));
            }
            Event::Code(t) => {
                if in_code_block {
                    code_block_content.push_str(&t);
                    continue;
                }
                let style = Style::default().fg(theme.accent);
                current_line_spans.push(Span::styled(t.to_string(), style));
            }
            Event::SoftBreak | Event::HardBreak => {
                push_line(&mut lines, &mut current_line_spans);
            }
            Event::FootnoteReference(t) => {
                current_line_spans.push(Span::styled(
                    format!("[^{}]", t),
                    Style::default().fg(Color::Blue),
                ));
            }
            _ => {}
        }
    }

    if !current_line_spans.is_empty() {
        lines.push(Line::from(current_line_spans));
    }

    lines
}

fn highlight_code(code: &str, lang: &str, theme: &Theme) -> Vec<Line<'static>> {
    let ss = syntax_set();
    let syntax = ss.find_syntax_by_token(lang);
    let syntax = match syntax {
        Some(s) => s,
        None => return plain_code_block(code, theme),
    };

    let ts = theme_set();
    let syntect_theme = ts.themes.get("base16-ocean.dark");
    let syntect_theme = match syntect_theme {
        Some(t) => t,
        None => return plain_code_block(code, theme),
    };

    let mut highlighter = syntect::easy::HighlightLines::new(syntax, syntect_theme);
    let bg_style = Style::default().bg(theme.code_bg);
    let mut lines: Vec<Line<'static>> = Vec::new();
    for line in syntect::util::LinesWithEndings::from(code) {
        match highlighter.highlight_line(line, ss) {
            Ok(ranges) => {
                let spans: Vec<Span<'static>> = ranges
                    .iter()
                    .map(|(syntect_style, text)| {
                        let fg = syntect_style.foreground;
                        let color = Color::Rgb(fg.r, fg.g, fg.b);
                        Span::styled(
                            text.to_string(),
                            Style::default().fg(color).bg(theme.code_bg),
                        )
                    })
                    .collect();
                if spans.is_empty() {
                    lines.push(Line::from(Span::styled("", bg_style)));
                } else {
                    lines.push(Line::from(spans));
                }
            }
            Err(_) => {
                let span = Span::styled(line.to_string(), bg_style);
                lines.push(Line::from(vec![span]));
            }
        }
    }
    if lines.is_empty() {
        lines.push(Line::from(Span::styled("", bg_style)));
    }
    lines
}

fn plain_code_block(code: &str, theme: &Theme) -> Vec<Line<'static>> {
    let bg_style = Style::default().bg(theme.code_bg);
    let mut lines: Vec<Line<'static>> = Vec::new();
    for line in code.lines() {
        let span = Span::styled(line.to_string(), bg_style);
        lines.push(Line::from(vec![span]));
    }
    if lines.is_empty() {
        lines.push(Line::from(Span::styled("", bg_style)));
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dark_theme() -> Theme {
        Theme::dark()
    }

    #[test]
    fn test_render_plain_text() {
        let lines = render_markdown("hello", &dark_theme());
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].to_string(), "hello");
    }

    #[test]
    fn test_render_empty_string() {
        let lines = render_markdown("", &dark_theme());
        assert!(lines.is_empty() || lines[0].to_string().is_empty());
    }

    #[test]
    fn test_render_heading() {
        let lines = render_markdown("# Title\n\nbody", &dark_theme());
        assert!(lines.len() >= 2);
        assert_eq!(lines[0].to_string(), "Title");
    }

    #[test]
    fn test_render_code_block() {
        let lines = render_markdown("```rust\nfn foo() {}\n```", &dark_theme());
        assert!(lines.len() >= 2);
    }

    #[test]
    fn test_render_inline_code() {
        let lines = render_markdown("use `kode`", &dark_theme());
        assert!(!lines.is_empty());
        let text = lines[0].to_string();
        assert!(text.contains("kode"));
    }

    #[test]
    fn test_render_emphasis() {
        let lines = render_markdown("_italic_", &dark_theme());
        assert!(!lines.is_empty());
        assert!(lines[0].to_string().contains("italic"));
    }

    #[test]
    fn test_render_strong() {
        let lines = render_markdown("**bold**", &dark_theme());
        assert!(!lines.is_empty());
        assert!(lines[0].to_string().contains("bold"));
    }

    #[test]
    fn test_render_list() {
        let lines = render_markdown("- item1\n- item2", &dark_theme());
        assert!(lines.len() >= 2);
        assert!(lines[0].to_string().contains("item1"));
        assert!(lines[1].to_string().contains("item2"));
    }

    #[test]
    fn test_render_link() {
        let lines = render_markdown("[text](url)", &dark_theme());
        assert!(!lines.is_empty());
        assert!(lines[0].to_string().contains("text"));
    }
}
