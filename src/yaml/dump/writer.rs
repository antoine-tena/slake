//! The emitter's writes: text, indicators, indentation and line breaks, with the column and the
//! whitespace state PyYAML tracks.

use super::emitter::Emitter;

impl Emitter<'_> {
    pub(super) fn write(&mut self, data: &str) {
        self.column += data.chars().count();
        self.out.push_str(data);
    }

    pub(super) fn write_indicator(
        &mut self,
        indicator: &str,
        need_whitespace: bool,
        whitespace: bool,
        indention: bool,
    ) {
        if !self.whitespace && need_whitespace {
            self.write(" ");
        }
        self.whitespace = whitespace;
        self.indention = self.indention && indention;
        self.open_ended = false;
        self.write(indicator);
    }

    pub(super) fn write_indent(&mut self) {
        let indent = self.indent.unwrap_or(0);
        if !self.indention || self.column > indent || (self.column == indent && !self.whitespace) {
            self.write_line_break('\n');
        }
        if self.column < indent {
            self.whitespace = true;
            let spaces = " ".repeat(indent - self.column);
            self.write(&spaces);
        }
    }

    fn write_line_break(&mut self, data: char) {
        self.whitespace = true;
        self.indention = true;
        self.column = 0;
        self.out.push(data);
    }

    pub(super) fn write_breaks(&mut self, text: &[char]) {
        if text[0] == '\n' {
            self.write_line_break('\n');
        }
        for &line_break in text {
            self.write_line_break(line_break);
        }
        self.write_indent();
    }
}
