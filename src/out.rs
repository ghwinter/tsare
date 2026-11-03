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
