//! PyYAML's writers of a scalar: plain, single-quoted, double-quoted, each folding its long
//! lines at the width the way PyYAML does.

use super::analysis::BREAKS;
use super::emitter::Emitter;

/// The run of characters being written: where it starts, and whether it is spaces or breaks.
#[derive(Default)]
struct Run {
    start: usize,
    spaces: bool,
    breaks: bool,
}

impl Run {
    fn follow(&mut self, ch: Option<char>) {
        if let Some(ch) = ch {
            self.spaces = ch == ' ';
            self.breaks = BREAKS.contains(&ch);
        }
    }
}

/// PyYAML's escape of a character in double quotes.
fn escape(ch: char) -> String {
    let short = match ch {
        '\0' => "0",
        '\x07' => "a",
        '\x08' => "b",
        '\x09' => "t",
        '\x0a' => "n",
        '\x0b' => "v",
        '\x0c' => "f",
        '\x0d' => "r",
        '\x1b' => "e",
        '"' => "\"",
        '\\' => "\\",
        '\u{85}' => "N",
        '\u{a0}' => "_",
        '\u{2028}' => "L",
        '\u{2029}' => "P",
        _ => "",
    };
    let code = ch as u32;
    match short {
        "" if code <= 0xff => format!("\\x{code:02X}"),
        "" if code <= 0xffff => format!("\\u{code:04X}"),
        "" => format!("\\U{code:08X}"),
        short => format!("\\{short}"),
    }
}

impl Emitter<'_> {
    fn write_chars(&mut self, chars: &[char]) {
        self.write(&chars.iter().collect::<String>());
    }

    /// After a fold or line breaks, a plain scalar goes on as text.
    fn resume_text(&mut self) {
        self.whitespace = false;
        self.indention = false;
    }

    /// A run of spaces; past the width, a single one becomes a line break when it can fold.
    /// Whether it did.
    fn write_spaces(&mut self, spaces: &[char], can_fold: bool) -> bool {
        if can_fold && spaces.len() == 1 && self.column > self.best_width {
            self.write_indent();
            true
        } else {
            self.write_chars(spaces);
            false
        }
    }

    /// A plain scalar at the root leaves the document open; an empty one writes nothing.
    /// Whether there is text to write.
    fn begin_plain(&mut self, text: &[char]) -> bool {
        if self.root_context {
            self.open_ended = true;
        }
        if text.is_empty() {
            return false;
        }
        if !self.whitespace {
            self.write(" ");
        }
        self.resume_text();
        true
    }

    pub(super) fn write_plain(&mut self, text: &[char], split: bool) {
        if !self.begin_plain(text) {
            return;
        }
        let mut run = Run::default();
        for end in 0..=text.len() {
            let ch = text.get(end).copied();
            if run.spaces {
                if ch != Some(' ') {
                    if self.write_spaces(&text[run.start..end], split) {
                        self.resume_text();
                    }
                    run.start = end;
                }
            } else if run.breaks {
                if ch.is_none_or(|ch| !BREAKS.contains(&ch)) {
                    self.write_breaks(&text[run.start..end]);
                    self.resume_text();
                    run.start = end;
                }
            } else if ch.is_none_or(|ch| ch == ' ' || BREAKS.contains(&ch)) {
                self.write_chars(&text[run.start..end]);
                run.start = end;
            }
            run.follow(ch);
        }
    }

    pub(super) fn write_single_quoted(&mut self, text: &[char], split: bool) {
        self.write_indicator("'", true, false, false);
        let mut run = Run::default();
        for end in 0..=text.len() {
            let ch = text.get(end).copied();
            if run.spaces {
                if ch != Some(' ') {
                    let can_fold = split && run.start != 0 && end != text.len();
                    self.write_spaces(&text[run.start..end], can_fold);
                    run.start = end;
                }
            } else if run.breaks {
                if ch.is_none_or(|ch| !BREAKS.contains(&ch)) {
                    self.write_breaks(&text[run.start..end]);
                    run.start = end;
                }
            } else if ch.is_none_or(|ch| " '".contains(ch) || BREAKS.contains(&ch))
                && run.start < end
            {
                self.write_chars(&text[run.start..end]);
                run.start = end;
            }
            if ch == Some('\'') {
                self.write("''");
                run.start = end + 1;
            }
            run.follow(ch);
        }
        self.write_indicator("'", false, false, false);
    }

    pub(super) fn write_double_quoted(&mut self, text: &[char], split: bool) {
        self.write_indicator("\"", true, false, false);
        let mut start = 0;
        for end in 0..=text.len() {
            let ch = text.get(end).copied();
            if ch.is_none_or(|ch| self.needs_escape(ch)) {
                start = self.write_escaped(text, start, end, ch);
            }
            if split && self.should_fold(text, start, end, ch) {
                start = self.fold_double_quoted(text, start, end);
            }
        }
        self.write_indicator("\"", false, false, false);
    }

    fn needs_escape(&self, ch: char) -> bool {
        let is_printable = ('\x20'..='\x7e').contains(&ch)
            || (self.options.allow_unicode
                && (('\u{a0}'..='\u{d7ff}').contains(&ch)
                    || ('\u{e000}'..='\u{fffd}').contains(&ch)));
        "\"\\\u{85}\u{2028}\u{2029}\u{feff}".contains(ch) || !is_printable
    }

    /// The text before an escape, then the escape itself; where the text goes on.
    fn write_escaped(
        &mut self,
        text: &[char],
        mut start: usize,
        end: usize,
        ch: Option<char>,
    ) -> usize {
        if start < end {
            self.write_chars(&text[start..end]);
            start = end;
        }
        if let Some(ch) = ch {
            self.write(&escape(ch));
            start = end + 1;
        }
        start
    }

    fn should_fold(&self, text: &[char], start: usize, end: usize, ch: Option<char>) -> bool {
        // `end - start` is -1 right after an escape, as in Python.
        0 < end
            && end + 1 < text.len()
            && (ch == Some(' ') || start >= end)
            && self.column as isize + end as isize - start as isize > self.best_width as isize
    }

    /// A line folded with a trailing backslash; a space that starts the next line is escaped.
    fn fold_double_quoted(&mut self, text: &[char], start: usize, end: usize) -> usize {
        let mut data: String = text[start.min(end)..end].iter().collect();
        data.push('\\');
        let start = start.max(end);
        self.write(&data);
        self.write_indent();
        self.resume_text();
        if text[start] == ' ' {
            self.write("\\");
        }
        start
    }
}
