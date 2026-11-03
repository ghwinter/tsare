//! The transport: a command writes lines to a sink, so its text can be checked without a process.
//!
//! The report types own the rendering; a command only picks a sink and writes.

/// Somewhere one line of output goes.
pub trait Sink {
    fn line(&mut self, text: &str);
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
    fn a_buffer_keeps_every_line() {
        let mut buffer = Buffer::default();
        buffer.line("a");
        buffer.line("b");
        assert_eq!(buffer.lines, vec!["a".to_string(), "b".to_string()]);
    }
}
