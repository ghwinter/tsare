//! The port a report renders through.
//!
//! `Sink` and `Report` live in the trusted base because every layer above produces a report and
//! none of them may reach upward for the trait it implements.  The transports (standard output, a
//! buffer, a writer) stay in the facade: they are adapters, not kernel.

use tsare_lang::json::Json;

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
        tsare_lang::json::serialize(&report.json())
    } else {
        report.text()
    };
    for line in rendered.lines() {
        sink.line(line);
    }
}
