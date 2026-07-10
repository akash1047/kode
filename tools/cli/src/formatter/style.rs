pub fn header(cmd: &str) -> String {
    let width = 56usize.saturating_sub(cmd.len() + 6);
    format!(
        "\u{2500}\u{2500} kode {cmd} {}\u{2500}\u{2500}\n",
        "\u{2500}".repeat(width)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_header_short_name() {
        let h = header("status");
        assert!(h.starts_with("\u{2500}\u{2500} kode status "));
        assert!(h.ends_with("\u{2500}\u{2500}\n"));
    }

    #[test]
    fn test_header_long_name() {
        let h = header("verylongcommandname");
        assert!(h.starts_with("\u{2500}\u{2500} kode verylongcommandname "));
    }

    #[test]
    fn test_header_very_long_name_saturates() {
        let h = header("averyveryveryveryveryveryverylongcommandnamethatexceedswidth");
        assert!(h.starts_with("\u{2500}\u{2500} kode "));
    }
}
