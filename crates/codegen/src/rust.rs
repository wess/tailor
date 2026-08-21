//! A very small Rust source writer.
//!
//! Not a formatter — the generator emits already-formatted lines and this only
//! tracks indentation. Anything cleverer would have to be kept in step with
//! rustfmt, and the point of the output is that you can read it before you run
//! anything over it.

/// A growing block of source, indentation-aware.
#[derive(Debug, Default)]
pub struct Source {
    lines: Vec<String>,
    indent: usize,
}

impl Source {
    pub fn new() -> Self {
        Source::default()
    }

    pub fn line(&mut self, text: impl AsRef<str>) {
        let text = text.as_ref();
        if text.is_empty() {
            self.lines.push(String::new());
        } else {
            self.lines
                .push(format!("{}{}", "    ".repeat(self.indent), text));
        }
    }

    /// Append pre-built lines at the current indentation.
    pub fn block(&mut self, lines: impl IntoIterator<Item = String>) {
        for line in lines {
            self.line(line);
        }
    }

    pub fn blank(&mut self) {
        // Never two blank lines in a row, and never one at the top.
        if !self.lines.is_empty() && !self.lines.last().map(|l| l.is_empty()).unwrap_or(true) {
            self.lines.push(String::new());
        }
    }

    pub fn open(&mut self, text: impl AsRef<str>) {
        self.line(text);
        self.indent += 1;
    }

    pub fn close(&mut self, text: impl AsRef<str>) {
        self.indent = self.indent.saturating_sub(1);
        self.line(text);
    }

    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    pub fn finish(self) -> String {
        let mut out = self.lines.join("\n");
        out.push('\n');
        out
    }
}

/// Indent a block of already-built lines by one level.
pub fn indent(lines: &[String]) -> Vec<String> {
    lines
        .iter()
        .map(|line| {
            if line.is_empty() {
                line.clone()
            } else {
                format!("    {line}")
            }
        })
        .collect()
}

/// A Rust string literal. Escapes what has to be escaped and nothing else, so
/// the generated code reads like something a person typed.
pub fn string(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            // Anything else in the control range has no literal form and a raw
            // NUL is not even valid Rust source, so spell it out.
            ch if ch.is_control() => out.push_str(&format!("\\u{{{:x}}}", ch as u32)),
            _ => out.push(ch),
        }
    }
    out.push('"');
    out
}

/// An `f32` literal Rust accepts: `12.` rather than `12`.
pub fn float(value: f32) -> String {
    if value.fract() == 0.0 && value.abs() < 1e9 {
        format!("{}.", value.trunc())
    } else {
        let text = format!("{value}");
        if text.contains('.') || text.contains('e') {
            text
        } else {
            format!("{text}.")
        }
    }
}

/// `px(12.)`
pub fn px(value: f32) -> String {
    format!("px({})", float(value))
}

/// Wrap a comment so a blurb does not run off the side of the file.
pub fn comment(prefix: &str, text: &str, width: usize) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        if !current.is_empty() && current.len() + word.len() + 1 > width {
            out.push(format!("{prefix}{current}"));
            current = String::new();
        }
        if !current.is_empty() {
            current.push(' ');
        }
        current.push_str(word);
    }
    if !current.is_empty() {
        out.push(format!("{prefix}{current}"));
    }
    if out.is_empty() {
        out.push(prefix.trim_end().to_string());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn indentation_tracks_open_and_close() {
        let mut source = Source::new();
        source.open("fn main() {");
        source.line("let x = 1;");
        source.close("}");
        assert_eq!(source.finish(), "fn main() {\n    let x = 1;\n}\n");
    }

    #[test]
    fn blank_lines_never_double_up() {
        let mut source = Source::new();
        source.blank();
        source.line("a");
        source.blank();
        source.blank();
        source.line("b");
        assert_eq!(source.finish(), "a\n\nb\n");
    }

    #[test]
    fn strings_escape_only_what_they_must() {
        assert_eq!(string("hi"), "\"hi\"");
        assert_eq!(string("a\"b"), "\"a\\\"b\"");
        assert_eq!(string("line\nnext"), "\"line\\nnext\"");
        assert_eq!(string("café"), "\"café\"");
        assert_eq!(string("a\u{0}b"), "\"a\\u{0}b\"");
        assert_eq!(string("bell\u{7}"), "\"bell\\u{7}\"");
    }

    #[test]
    fn floats_always_have_a_point() {
        assert_eq!(float(12.0), "12.");
        assert_eq!(float(12.5), "12.5");
        assert_eq!(float(-3.0), "-3.");
        assert_eq!(px(8.0), "px(8.)");
    }

    #[test]
    fn comments_wrap_at_the_width() {
        let lines = comment("// ", "one two three four five", 12);
        assert_eq!(lines, ["// one two", "// three four", "// five"]);
    }
}
