use ab_glyph::Font;
use imageproc::drawing;

/// Displays a number in thousands, or millions if over a million.
/// Expects input to be positive.
pub fn display_num_thousands(num: f64) -> String {
    return match num {
        0.0..10000.0 => format!("{:.2}k", num / 1000.0),
        10000.0..100000.0 => format!("{:.1}k", num / 1000.0),
        100000.0..1000000.0 => format!("{:.0}k", num / 1000.0),
        1000000.0..10000000.0 => format!("{:.2}M", num / 1000000.0),
        10000000.0..100000000.0 => format!("{:.1}M", num / 1000000.0),
        100000000.0.. => format!("{:.0}M", num / 1000000.0),
        _ => "ERROR".to_string(),
    };
}

/// Expects input to be positive.
pub fn display_num(num: f64) -> String {
    return match num {
        0.0..1000.0 => format!("{num:.0}"),
        1000.0..10000.0 => format!("{:.2}k", num / 1000.0),
        10000.0..100000.0 => format!("{:.1}k", num / 1000.0),
        100000.0..1000000.0 => format!("{:.0}k", num / 1000.0),
        1000000.0..10000000.0 => format!("{:.2}M", num / 1000000.0),
        10000000.0..100000000.0 => format!("{:.1}M", num / 1000000.0),
        100000000.0.. => format!("{:.0}M", num / 1000000.0),
        _ => "ERROR".to_string(),
    };
}

/// Assumes whitespace is only a single space between words
pub fn text_wrap(text: &str, font: &impl Font, scale: f32, width: u32) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut line = String::new();

    for part in text.split_ascii_whitespace() {
        let prospective = if line.is_empty() {
            part.to_string()
        } else {
            format!("{line} {part}")
        };
        if drawing::text_size(scale, font, &prospective).0 > width {
            out.push(line);
            line = part.to_string();
        } else {
            line = prospective;
        }
    }
    if !line.is_empty() {
        out.push(line);
    }
    return out;
}
