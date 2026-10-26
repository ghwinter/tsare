//! One place that owns diagnostics: ordering, counts, text, JSON and the exit code.
//!
//! Every command used to assemble its own output, so the text format, the JSON shape and the
//! summary were three separate pieces of code that could disagree.  A session is the single
//! owner: commands emit into it, and it answers.

use crate::check::Diagnostic;

pub struct Session {
    diagnostics: Vec<Diagnostic>,
    /// The document being reported on, when there is exactly one.
    source: Option<String>,
    /// One entry per file, when there are several: a directory is reported file by file.
    sources: Vec<(String, String)>,
    display: String,
}

impl Session {
    pub fn new(display: impl Into<String>) -> Self {
        Session { diagnostics: Vec::new(), source: None, sources: Vec::new(), display: display.into() }
    }

    pub fn with_source(mut self, source: &str) -> Self {
        if !source.is_empty() {
            self.source = Some(source.to_string());
        }
        self
    }

    /// Report a directory: each file is rendered against its own text.
    pub fn with_sources(mut self, sources: &[(String, String)]) -> Self {
        self.sources = sources.to_vec();
        self
    }

    pub fn emit(&mut self, diagnostic: Diagnostic) {
        self.diagnostics.push(diagnostic);
    }

    pub fn extend(&mut self, diagnostics: impl IntoIterator<Item = Diagnostic>) {
        self.diagnostics.extend(diagnostics);
    }

    pub fn is_empty(&self) -> bool {
        self.diagnostics.is_empty()
    }

    pub fn errors(&self) -> usize {
        self.diagnostics.iter().filter(|item| item.code.starts_with('E')).count()
    }

    pub fn warnings(&self) -> usize {
        self.diagnostics.len() - self.errors()
    }

    pub fn exit_code(&self) -> i32 {
        if self.errors() > 0 {
            1
        } else {
            0
        }
    }

    pub fn summary(&self) -> String {
        format!("{} error(s), {} warning(s)", self.errors(), self.warnings())
    }

    /// Diagnostics in a stable order: file, then position, then code.
    fn sorted(&self) -> Vec<&Diagnostic> {
        let mut items: Vec<&Diagnostic> = self.diagnostics.iter().collect();
        items.sort_by(|left, right| {
            (&left.file, left.line, left.col, left.code).cmp(&(&right.file, right.line, right.col, right.code))
        });
        items
    }

    fn severity(code: &str) -> &'static str {
        if code.starts_with('E') {
            "error"
        } else {
            "warning"
        }
    }

    fn file_of(&self, item: &Diagnostic) -> String {
        if item.file.is_empty() {
            self.display.clone()
        } else {
            item.file.clone()
        }
    }

    /// Text output: rendered against the source when the session has one, flat otherwise.
    pub fn text(&self) -> String {
        if !self.sources.is_empty() {
            let mut out = String::new();
            for (name, text) in &self.sources {
                let mine: Vec<Diagnostic> =
                    self.sorted().into_iter().filter(|item| &item.file == name).cloned().collect();
                if mine.is_empty() {
                    continue;
                }
                out.push_str(&crate::render::render(text, name, &mine));
            }
            // a diagnostic that belongs to no loaded file still gets printed
            let inside = |item: &Diagnostic| self.sources.iter().any(|(name, _)| &item.file == name);
            for item in self.sorted() {
                if !inside(item) {
                    out.push_str(&format!(
                        "  {} {}:{} {}\n",
                        item.code,
                        self.file_of(item),
                        item.line,
                        item.message
                    ));
                }
            }
            return out;
        }
        match &self.source {
            Some(text) if self.diagnostics.iter().all(|item| item.file.is_empty()) => {
                crate::render::render(text, &self.display, &self.diagnostics)
            }
            _ => {
                let mut out = String::new();
                for item in self.sorted() {
                    out.push_str(&format!(
                        "  {} {}:{} {}\n",
                        item.code,
                        self.file_of(item),
                        item.line,
                        item.message
                    ));
                }
                out
            }
        }
    }

    /// The machine contract from PROTOCOL.md.
    pub fn json(&self) -> String {
        let mut lines: Vec<String> = Vec::new();
        for item in self.sorted() {
            lines.push(format!(
                "    {{\"code\": \"{}\", \"severity\": \"{}\", \"file\": \"{}\", \"line\": {}, \"col\": {}, \"message\": \"{}\"}}",
                item.code,
                Self::severity(item.code),
                crate::cli::escape(&self.file_of(item)),
                item.line,
                item.col,
                crate::cli::escape(&item.message)
            ));
        }
        let joined = if lines.is_empty() { String::new() } else { format!("\n{}\n", lines.join(",\n")) };
        format!(
            "{{\n  \"diagnostics\": [{joined}],\n  \"errors\": {},\n  \"warnings\": {}\n}}",
            self.errors(),
            self.warnings()
        )
    }
}
