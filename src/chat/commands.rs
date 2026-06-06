use std::io::stdout;

use crossterm::{
    cursor::MoveTo,
    execute,
    terminal::{Clear, ClearType},
};

pub enum SlashCommand {
    Help,
    Clear,
    Model(Option<String>),
    Reset,
    Exit,
    Unknown(String),
}

pub fn parse(input: &str) -> Option<SlashCommand> {
    let trimmed = input.trim();
    if !trimmed.starts_with('/') {
        return None;
    }
    let mut parts = trimmed[1..].split_whitespace();
    let head = parts.next()?;
    let rest = parts.collect::<Vec<_>>().join(" ");
    Some(match head {
        "help" | "h" | "?" => SlashCommand::Help,
        "clear" | "cls" => SlashCommand::Clear,
        "reset" => SlashCommand::Reset,
        "exit" | "quit" | "q" => SlashCommand::Exit,
        "model" | "m" => {
            if rest.is_empty() {
                SlashCommand::Model(None)
            } else {
                SlashCommand::Model(Some(rest))
            }
        }
        other => SlashCommand::Unknown(other.to_string()),
    })
}

pub fn print_help() {
    let dim = "\x1b[2m";
    let bold = "\x1b[1m";
    let cyan = "\x1b[36m";
    let reset = "\x1b[0m";
    println!("{bold}Slash commands:{reset}");
    println!("  {cyan}/help{reset}              show this list");
    println!("  {cyan}/clear{reset}             clear the terminal");
    println!("  {cyan}/reset{reset}             start a fresh conversation (drops history)");
    println!("  {cyan}/model{reset} [name]      show or switch the LLM model");
    println!("  {cyan}/exit{reset}              quit");
    println!();
    println!("{dim}Multiline: end a line with `\\` to continue on next line.{reset}");
    println!("{dim}Ctrl-C cancels an in-flight reply. Ctrl-D exits.{reset}");
}

pub fn clear_terminal() {
    let _ = execute!(stdout(), Clear(ClearType::All), MoveTo(0, 0));
}
