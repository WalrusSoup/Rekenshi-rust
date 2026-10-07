//! Shared palette and spacing for every built-in and SDK settings control.
#![forbid(unsafe_code)]

/// MyGUI::Colour is four consecutive floats in the inspected native headers.
#[repr(C)]
#[derive(Clone, Copy)]
pub(super) struct Colour(pub f32, pub f32, pub f32, pub f32);
impl Colour {
    const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self(r as f32 / 255., g as f32 / 255., b as f32 / 255., 1.)
    }
}
pub(super) const BACKGROUND: Colour = Colour::rgb(18, 22, 29);
pub(super) const SURFACE: Colour = Colour::rgb(27, 33, 42);
pub(super) const CONTROL: Colour = Colour::rgb(42, 51, 64);
pub(super) const HOVER: Colour = Colour::rgb(57, 70, 85);
pub(super) const BORDER: Colour = Colour::rgb(69, 82, 99);
pub(super) const TEXT: Colour = Colour::rgb(235, 239, 244);
pub(super) const MUTED: Colour = Colour::rgb(171, 183, 199);
pub(super) const DISABLED: Colour = Colour::rgb(129, 144, 163);
pub(super) const ACCENT: Colour = Colour::rgb(132, 214, 179);
pub(super) const WARNING: Colour = Colour::rgb(242, 190, 132);
pub(super) const TEXT_SKIN: &str = "Kenshi_TextBoxEmptySkin";
pub(super) const SURFACE_SKIN: &str = "WhiteSkin";
pub(super) const FONT: &str = "Kenshi_StandardFont_Medium";
pub(super) const ROW_HEIGHT: i32 = 56;
pub(super) const ROW_START: i32 = 72;
pub(super) const FOOTER: i32 = 78;

/// Native SimpleText supports explicit newlines. Wrap on word boundaries and
/// bound the caption to two lines so long native-adapter errors stay in a row.
pub(super) fn description(text: &str, width: i32) -> String {
    let columns = (width / 7).max(12) as usize;
    let mut result = String::new();
    let mut used = 0;
    let mut lines = 1;
    for word in text.split_whitespace() {
        let length = word.chars().count();
        if used > 0 && used + 1 + length > columns {
            if lines == 2 {
                // Reserve room for the suffix without splitting a UTF-8 codepoint.
                while used > columns.saturating_sub(3) {
                    result.pop();
                    used -= 1;
                }
                while result.ends_with(' ') {
                    result.pop();
                }
                result.push_str("...");
                break;
            }
            result.push('\n');
            lines += 1;
            used = 0;
        }
        if used > 0 {
            result.push(' ');
            used += 1;
        }
        let available = columns.saturating_sub(used);
        if length > available {
            result.extend(word.chars().take(available.saturating_sub(3)));
            result.push_str("...");
            break;
        }
        result.push_str(word);
        used += length;
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn luminance(colour: Colour) -> f32 {
        let linear = |channel: f32| {
            if channel <= 0.04045 {
                channel / 12.92
            } else {
                ((channel + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * linear(colour.0) + 0.7152 * linear(colour.1) + 0.0722 * linear(colour.2)
    }

    #[test]
    fn small_text_and_checkbox_indicators_remain_legible() {
        for foreground in [TEXT, MUTED, DISABLED, WARNING] {
            for background in [BACKGROUND, SURFACE] {
                assert!((luminance(foreground) + 0.05) / (luminance(background) + 0.05) >= 4.5);
            }
        }
        assert!((luminance(ACCENT) + 0.05) / (luminance(SURFACE) + 0.05) >= 3.);
    }

    #[test]
    fn long_descriptions_fit_two_lines_without_splitting_unicode() {
        for input in [
            "Short description",
            "A long unavailable message with many words explaining the missing native adapter",
            "日本語日本語日本語日本語日本語日本語日本語",
            "aaaaaa bbbbb cccccc ddddd eeeee ffffff",
        ] {
            let result = description(input, 84);
            assert!(result.lines().count() <= 2);
            assert!(result.lines().all(|line| line.chars().count() <= 12));
        }
    }
}
