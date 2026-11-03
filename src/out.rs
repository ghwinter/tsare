//! The transport: a report renders to a sink, so its text can be checked without a process.
//!
//! A `Report` owns its two renderings; `render` picks one and writes it to any `Sink`.  The
//! commands only choose a report and a sink.

use std::io::Write;

use crate::json::Json;

/// Somewhere one line of output goes.
pub trait Sink {
    fn line(&mut self, text: &str);
}

/// A rendered report: the text for a reader, the JSON for a caller.
pub trait Report {
    fn text(&self) -> String;
    fn json(&self) -> Json;
}

/// Render a report into a sink: the JSON when asked, the text otherwise.
pub fn render<R: Report>(report: &R, json: bool, sink: &mut dyn Sink) {
    let rendered = if json {
        crate::json::serialize(&report.json())
    } else {
        report.text()
    };
    for line in rendered.lines() {
        sink.line(line);
    }
}

/// The process's standard output.
pub struct Stdout;

impl Sink for Stdout {
    fn line(&mut self, text: &str) {
        println!("{text}");
    }
}

/// A sink that keeps the lines, for tests and for callers embedding the CLI.
#[derive(Default)]
pub struct Buffer {
    pub lines: Vec<String>,
}

impl Sink for Buffer {
    fn line(&mut self, text: &str) {
        self.lines.push(text.to_string());
    }
}

/// A sink over any writer: a `Vec<u8>` in a test, a file, or a pipe.
pub struct Writer<W: Write> {
    inner: W,
}

impl<W: Write> Writer<W> {
    pub fn new(inner: W) -> Self {
        Self { inner }
    }
}

impl<W: Write> Sink for Writer<W> {
    fn line(&mut self, text: &str) {
        let _ = writeln!(self.inner, "{text}");
    }
}

/// What a command returns: the exit code, and the message that belongs to it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// No errors.
    Ok,
    /// The invocation is wrong (exit 2).
    Usage,
    /// The command failed and already reported why (exit 1).
    Failed,
    /// The command refused to write, with the reason (exit 1).
    Refused(String),
    /// The command could not run, with the reason (exit 1).
    Unavailable(String),
}

impl Outcome {
    pub fn code(&self) -> i32 {
        match self {
            Outcome::Ok => 0,
            Outcome::Usage => 2,
            Outcome::Failed | Outcome::Refused(_) | Outcome::Unavailable(_) => 1,
        }
    }

    pub fn message(&self) -> Option<&str> {
        match self {
            Outcome::Refused(message) | Outcome::Unavailable(message) => Some(message),
            Outcome::Ok | Outcome::Usage | Outcome::Failed => None,
        }
    }

    /// The outcome a raw exit code names, for the callers that still count in integers.
    pub fn from_code(code: i32) -> Self {
        match code {
            0 => Outcome::Ok,
            2 => Outcome::Usage,
            _ => Outcome::Failed,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_outcome_maps_to_the_exit_codes() {
        assert_eq!(Outcome::Ok.code(), 0);
        assert_eq!(Outcome::Usage.code(), 2);
        assert_eq!(Outcome::Failed.code(), 1);
        assert_eq!(Outcome::Refused("r".into()).code(), 1);
        assert_eq!(Outcome::Unavailable("u".into()).code(), 1);
        assert_eq!(Outcome::from_code(0), Outcome::Ok);
        assert_eq!(Outcome::from_code(2), Outcome::Usage);
        assert_eq!(Outcome::from_code(1), Outcome::Failed);
        assert_eq!(Outcome::Refused("r".into()).message(), Some("r"));
        assert_eq!(Outcome::Ok.message(), None);
    }

    #[test]
    fn a_buffer_keeps_every_line() {
        let mut buffer = Buffer::default();
        buffer.line("a");
        buffer.line("b");
        assert_eq!(buffer.lines, vec!["a".to_string(), "b".to_string()]);
    }

    #[test]
    fn a_report_renders_text_or_json_to_any_sink() {
        struct Doc;
        impl Report for Doc {
            fn text(&self) -> String {
                "hello\nthere".to_string()
            }
            fn json(&self) -> Json {
                Json::Str("hello".to_string())
            }
        }
        let mut buffer = Buffer::default();
        render(&Doc, false, &mut buffer);
        assert_eq!(buffer.lines, vec!["hello".to_string(), "there".to_string()]);
        let mut buffer = Buffer::default();
        render(&Doc, true, &mut buffer);
        assert_eq!(buffer.lines, vec!["\"hello\"".to_string()]);
    }

    #[test]
    fn a_writer_is_a_sink() {
        let mut writer = Writer::new(Vec::new());
        writer.line("a");
        writer.line("b");
        assert_eq!(writer.inner, b"a\nb\n".to_vec());
    }
}
