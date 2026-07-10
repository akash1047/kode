use crate::chat::ChatView;

pub fn format(view: &ChatView) -> String {
    if view.answer.is_empty() {
        return String::new();
    }
    let mut out = view.answer.clone();
    out.push('\n');
    out
}

pub fn format_json(view: &ChatView) -> String {
    let json = serde_json::json!({
        "answer": view.answer,
    });
    serde_json::to_string_pretty(&json).unwrap_or_else(|_| "{}".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chat::ChatView;

    #[test]
    fn test_format_empty_answer() {
        let view = ChatView {
            answer: String::new(),
        };
        assert_eq!(format(&view), "");
    }

    #[test]
    fn test_format_with_answer() {
        let view = ChatView {
            answer: "Hello".into(),
        };
        assert_eq!(format(&view), "Hello\n");
    }

    #[test]
    fn test_format_json_output() {
        let view = ChatView {
            answer: "test".into(),
        };
        let out = format_json(&view);
        assert!(out.contains("\"answer\": \"test\""));
    }
}
