use std::borrow::Cow;

use reedline::{Prompt, PromptEditMode, PromptHistorySearch, PromptHistorySearchStatus};

const BRIGHT_CYAN: &str = "\x1b[96m";
const BOLD: &str = "\x1b[1m";
const DIM: &str = "\x1b[2m";
const MAGENTA: &str = "\x1b[35m";
const GREEN: &str = "\x1b[32m";
const RESET: &str = "\x1b[0m";

pub struct KodePrompt {
    pub model: String,
}

impl Prompt for KodePrompt {
    fn render_prompt_left(&self) -> Cow<'_, str> {
        Cow::Owned(format!("{BOLD}{BRIGHT_CYAN}❯{RESET} "))
    }

    fn render_prompt_right(&self) -> Cow<'_, str> {
        Cow::Owned(format!("{DIM}{}{RESET}", self.model))
    }

    fn render_prompt_indicator(&self, _mode: PromptEditMode) -> Cow<'_, str> {
        Cow::Borrowed("")
    }

    fn render_prompt_multiline_indicator(&self) -> Cow<'_, str> {
        Cow::Owned(format!("{DIM}··{RESET} "))
    }

    fn render_prompt_history_search_indicator(
        &self,
        history_search: PromptHistorySearch,
    ) -> Cow<'_, str> {
        let prefix = match history_search.status {
            PromptHistorySearchStatus::Passing => "",
            PromptHistorySearchStatus::Failing => "failing ",
        };
        Cow::Owned(format!(
            "{MAGENTA}({prefix}reverse-search: {}){RESET} ",
            history_search.term
        ))
    }

    fn get_prompt_color(&self) -> reedline::Color {
        reedline::Color::Reset
    }

    fn get_prompt_multiline_color(&self) -> nu_ansi_term::Color {
        nu_ansi_term::Color::Default
    }

    fn get_indicator_color(&self) -> reedline::Color {
        reedline::Color::Reset
    }

    fn get_prompt_right_color(&self) -> reedline::Color {
        reedline::Color::Reset
    }
}

pub struct ContinuationValidator;

impl reedline::Validator for ContinuationValidator {
    fn validate(&self, line: &str) -> reedline::ValidationResult {
        if line.trim_end().ends_with('\\') {
            reedline::ValidationResult::Incomplete
        } else {
            reedline::ValidationResult::Complete
        }
    }
}

#[allow(dead_code)]
const _: &str = GREEN;
