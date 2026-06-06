use termimad::MadSkin;
use termimad::crossterm::style::Color as MadColor;

pub struct Renderer {
    skin: MadSkin,
}

impl Renderer {
    pub fn new() -> Self {
        let mut skin = MadSkin::default();

        skin.set_headers_fg(MadColor::Rgb { r: 0, g: 230, b: 230 });
        skin.bold.set_fg(MadColor::Rgb { r: 255, g: 255, b: 255 });
        skin.italic.set_fg(MadColor::Rgb { r: 220, g: 180, b: 255 });
        skin.inline_code.set_fg(MadColor::Rgb { r: 255, g: 180, b: 80 });
        skin.inline_code.set_bg(MadColor::Rgb { r: 38, g: 38, b: 38 });
        skin.code_block.set_fg(MadColor::Rgb { r: 220, g: 220, b: 220 });
        skin.code_block.set_bg(MadColor::Rgb { r: 28, g: 28, b: 28 });
        skin.quote_mark.set_fg(MadColor::Rgb { r: 130, g: 130, b: 130 });
        skin.bullet.set_fg(MadColor::Rgb { r: 0, g: 230, b: 230 });

        Self { skin }
    }

    pub fn print(&self, markdown: &str) {
        let trimmed = markdown.trim_end();
        if trimmed.is_empty() {
            return;
        }
        self.skin.print_text(trimmed);
    }
}

impl Default for Renderer {
    fn default() -> Self {
        Self::new()
    }
}
