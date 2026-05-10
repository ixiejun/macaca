//! Provider-neutral token estimation helpers.

/// Estimate tokens using the same CJK-aware heuristic used by the runtime
/// context window. This is intentionally approximate and provider-neutral.
pub fn estimate_text_tokens(text: &str) -> u32 {
    let cjk_chars = text
        .chars()
        .filter(|c| {
            matches!(*c as u32,
                0x4E00..=0x9FFF | 0x3400..=0x4DBF | 0xF900..=0xFAFF |
                0x3000..=0x303F | 0xFF00..=0xFFEF
            )
        })
        .count();
    let total_chars = text.chars().count();
    let ascii_chars = total_chars.saturating_sub(cjk_chars);
    (cjk_chars as f64 / 1.5 + ascii_chars as f64 / 4.0) as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn estimates_ascii_and_cjk() {
        assert_eq!(estimate_text_tokens("abcd"), 1);
        assert!(estimate_text_tokens("上下文工程") >= 2);
    }
}
