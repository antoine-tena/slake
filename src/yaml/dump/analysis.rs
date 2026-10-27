//! PyYAML's `analyze_scalar`: which styles can write a scalar.

pub const BREAKS: &[char] = &['\n', '\u{85}', '\u{2028}', '\u{2029}'];
const SPACES_AND_BREAKS: &[char] = &[
    '\0', ' ', '\t', '\r', '\n', '\u{85}', '\u{2028}', '\u{2029}',
];

pub struct Analysis {
    pub empty: bool,
    pub multiline: bool,
    pub allow_flow_plain: bool,
    pub allow_block_plain: bool,
    pub allow_single_quoted: bool,
}

/// What the scan of a scalar finds, before the styles it allows are decided.
#[derive(Default)]
struct Findings {
    block_indicators: bool,
    flow_indicators: bool,
    line_breaks: bool,
    special_characters: bool,
    leading_space: bool,
    leading_break: bool,
    trailing_space: bool,
    trailing_break: bool,
    break_space: bool,
    space_break: bool,
}

/// The kind of the character before, for spaces next to breaks.
#[derive(Clone, Copy, PartialEq)]
enum Previous {
    Space,
    Break,
    Other,
}

pub fn analyze(scalar: &[char], allow_unicode: bool) -> Analysis {
    if scalar.is_empty() {
        return Analysis {
            empty: true,
            multiline: false,
            allow_flow_plain: false,
            allow_block_plain: true,
            allow_single_quoted: true,
        };
    }
    scan(scalar, allow_unicode).analysis()
}

fn scan(scalar: &[char], allow_unicode: bool) -> Findings {
    let mut findings = Findings::default();
    let starts_with = |prefix: &str| scalar.iter().take(3).collect::<String>() == prefix;
    if starts_with("---") || starts_with("...") {
        findings.block_indicators = true;
        findings.flow_indicators = true;
    }
    let mut preceded_by_whitespace = true;
    let mut previous = Previous::Other;
    for (index, &ch) in scalar.iter().enumerate() {
        let followed_by_whitespace = scalar
            .get(index + 1)
            .is_none_or(|next| SPACES_AND_BREAKS.contains(next));
        findings.indicators(index, ch, preceded_by_whitespace, followed_by_whitespace);
        findings.line_breaks |= BREAKS.contains(&ch);
        findings.special_characters |= is_special(ch, allow_unicode);
        previous = findings.whitespace(index, ch, scalar.len(), previous);
        preceded_by_whitespace = SPACES_AND_BREAKS.contains(&ch);
    }
    findings
}

/// A character only double quotes can write.
fn is_special(ch: char, allow_unicode: bool) -> bool {
    if ch == '\n' || ('\x20'..='\x7e').contains(&ch) {
        return false;
    }
    let is_unicode = (ch == '\u{85}'
        || ('\u{a0}'..='\u{d7ff}').contains(&ch)
        || ('\u{e000}'..='\u{fffd}').contains(&ch)
        || ('\u{10000}'..'\u{10ffff}').contains(&ch))
        && ch != '\u{feff}';
    !is_unicode || !allow_unicode
}

impl Findings {
    /// The indicators that keep a scalar from being plain in flow or block context.
    fn indicators(&mut self, index: usize, ch: char, preceded: bool, followed: bool) {
        if index == 0 {
            if "#,[]{}&*!|>'\"%@`".contains(ch) || (ch == '-' && followed) {
                self.flow_indicators = true;
                self.block_indicators = true;
            }
            if "?:".contains(ch) {
                self.flow_indicators = true;
                self.block_indicators |= followed;
            }
        } else {
            if ",?[]{}".contains(ch) {
                self.flow_indicators = true;
            }
            if ch == ':' {
                self.flow_indicators = true;
                self.block_indicators |= followed;
            }
            if ch == '#' && preceded {
                self.flow_indicators = true;
                self.block_indicators = true;
            }
        }
    }

    /// Spaces and breaks at the ends, and a space right after a break or right before one.
    fn whitespace(
        &mut self,
        index: usize,
        ch: char,
        length: usize,
        previous: Previous,
    ) -> Previous {
        let (is_first, is_last) = (index == 0, index + 1 == length);
        if ch == ' ' {
            self.leading_space |= is_first;
            self.trailing_space |= is_last;
            self.break_space |= previous == Previous::Break;
            Previous::Space
        } else if BREAKS.contains(&ch) {
            self.leading_break |= is_first;
            self.trailing_break |= is_last;
            self.space_break |= previous == Previous::Space;
            Previous::Break
        } else {
            Previous::Other
        }
    }

    fn analysis(&self) -> Analysis {
        let has_edge_whitespace =
            self.leading_space || self.leading_break || self.trailing_space || self.trailing_break;
        // Spaces next to breaks, and special characters, need double quotes.
        let needs_double = self.break_space || self.space_break || self.special_characters;
        let cannot_be_plain = has_edge_whitespace || needs_double || self.line_breaks;
        Analysis {
            empty: false,
            multiline: self.line_breaks,
            allow_flow_plain: !(cannot_be_plain || self.flow_indicators),
            allow_block_plain: !(cannot_be_plain || self.block_indicators),
            allow_single_quoted: !needs_double,
        }
    }
}
