use ratatui::style::Color;

#[derive(Debug, Clone)]
pub struct Theme {
    pub code_bg: Color,
    pub muted: Color,
    pub accent: Color,
}

impl Theme {
    pub fn dark() -> Self {
        Self {
            code_bg: Color::Rgb(20, 20, 24),
            muted: Color::Gray,
            accent: Color::Cyan,
        }
    }

    pub fn light() -> Self {
        Self {
            code_bg: Color::Rgb(245, 245, 245),
            muted: Color::DarkGray,
            accent: Color::Rgb(0, 95, 135),
        }
    }

    pub fn no_color() -> Self {
        Self {
            code_bg: Color::Reset,
            muted: Color::Reset,
            accent: Color::Reset,
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::dark()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dark_theme() {
        let t = Theme::dark();
        assert_eq!(t.code_bg, Color::Rgb(20, 20, 24));
        assert_eq!(t.muted, Color::Gray);
        assert_eq!(t.accent, Color::Cyan);
    }

    #[test]
    fn test_light_theme() {
        let t = Theme::light();
        assert_eq!(t.code_bg, Color::Rgb(245, 245, 245));
        assert_eq!(t.muted, Color::DarkGray);
        assert_eq!(t.accent, Color::Rgb(0, 95, 135));
    }

    #[test]
    fn test_no_color_theme() {
        let t = Theme::no_color();
        assert_eq!(t.code_bg, Color::Reset);
        assert_eq!(t.muted, Color::Reset);
        assert_eq!(t.accent, Color::Reset);
    }

    #[test]
    fn test_default_is_dark() {
        let t = Theme::default();
        assert_eq!(t.code_bg, Color::Rgb(20, 20, 24));
    }
}
