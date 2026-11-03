use super::*;

pub struct Loaded {
    pub decls: Vec<Decl>,
    pub text: String,
    /// A document that does not parse has no declarations, but it does have a diagnostic.
    pub syntax: Vec<crate::check::Diagnostic>,
    /// One entry per file when the path is a directory, so each diagnostic can be rendered
    /// against its own source.
    pub sources: Vec<(String, String)>,
}

pub(crate) fn syntax_diagnostic(
    file: &str,
    error: &syntax::ParseError,
) -> crate::check::Diagnostic {
    crate::check::Diagnostic {
        code: error.code,
        message: error.message.clone(),
        line: error.line,
        col: error.col,
        end_line: error.line,
        end_col: error.col + 1,
        file: file.to_string(),
        witness: None,
    }
}

/// Load and parse, keeping syntax errors as diagnostics.  Callers that cannot analyse a
/// broken document use `load`, which turns the first one into a printed message.
fn read(path: &Path) -> Result<Loaded, String> {
    if path.as_os_str() == "-" {
        // a document from a pipe: editors and CI both want this
        let mut text = String::new();
        std::io::stdin()
            .read_to_string(&mut text)
            .map_err(|error| format!("standard input: {error}"))?;
        let (decls, errors) = syntax::parse_recovering(&text);
        let syntax = errors
            .iter()
            .map(|error| syntax_diagnostic("-", error))
            .collect();
        return Ok(Loaded {
            decls,
            text,
            syntax,
            sources: Vec::new(),
        });
    }
    if path.is_dir() {
        let mut files: Vec<PathBuf> = fs::read_dir(path)
            .map_err(|error| format!("{}: {error}", path.display()))?
            .filter_map(|entry| entry.ok().map(|item| item.path()))
            .filter(|item| item.extension().map(|ext| ext == "tsare").unwrap_or(false))
            .collect();
        files.sort();
        let mut decls = Vec::new();
        let mut syntax_errors = Vec::new();
        let mut sources: Vec<(String, String)> = Vec::new();
        for file in &files {
            let source =
                fs::read_to_string(file).map_err(|error| format!("{}: {error}", file.display()))?;
            let name = file
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            sources.push((name.clone(), source.clone()));
            let (mut parsed, errors) = syntax::parse_recovering(&source);
            for decl in &mut parsed {
                decl.file = name.clone();
            }
            decls.extend(parsed);
            for error in &errors {
                syntax_errors.push(syntax_diagnostic(&name, error));
            }
        }
        return Ok(Loaded {
            decls,
            text: String::new(),
            syntax: syntax_errors,
            sources,
        });
    }
    let text = fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let (decls, errors) = syntax::parse_recovering(&text);
    let syntax_errors: Vec<crate::check::Diagnostic> = errors
        .iter()
        .map(|error| syntax_diagnostic("", error))
        .collect();
    Ok(Loaded {
        decls,
        text,
        syntax: syntax_errors,
        sources: Vec::new(),
    })
}

/// How a document is read: strictly (a syntax error is an error) or recoveringly (it becomes a
/// diagnostic).  The reading itself is one implementation.
pub struct Loader {
    strict: bool,
}

impl Loader {
    pub fn strict() -> Self {
        Self { strict: true }
    }

    pub fn recovering() -> Self {
        Self { strict: false }
    }

    pub fn load(&self, path: &Path) -> Result<Loaded, String> {
        let loaded = read(path)?;
        if self.strict {
            if let Some(first) = loaded.syntax.first() {
                return Err(format!(
                    "{}:{}:{}: error[E-SYNTAX]: {}",
                    path.display(),
                    first.line,
                    first.col,
                    first.message
                ));
            }
        }
        Ok(loaded)
    }
}

pub fn load(path: &Path) -> Result<Loaded, String> {
    Loader::strict().load(path)
}

pub fn load_for_check(path: &Path) -> Result<Loaded, String> {
    Loader::recovering().load(path)
}

pub(crate) fn fail(error: String) -> Outcome {
    Outcome::Unavailable(error)
}

pub(crate) fn pad(text: &str, width: usize) -> String {
    let mut out = text.to_string();
    while out.chars().count() < width {
        out.push(' ');
    }
    out
}

pub(crate) fn error_count(diagnostics: &[crate::check::Diagnostic]) -> usize {
    diagnostics
        .iter()
        .filter(|item| item.code.starts_with("E-"))
        .count()
}

pub(crate) fn warning_count(diagnostics: &[crate::check::Diagnostic]) -> usize {
    diagnostics.len() - error_count(diagnostics)
}

/// Diagnostics as JSON records, the contract in PROTOCOL.md.
pub(crate) fn collect_holes(decls: &[Decl]) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for decl in decls {
        for (key, value) in &decl.fields {
            match value {
                Val::Hole => out.push((decl.name.clone(), key.clone())),
                Val::List(items) => {
                    for item in items {
                        if matches!(item, Val::Hole) {
                            out.push((decl.name.clone(), key.clone()));
                        }
                    }
                }
                _ => {}
            }
        }
    }
    out
}

// --------------------------------------------------------------------- subcommands

/// `check FILE --against KERNEL`: the kernel is a prelude. Its names may be used, not
/// redefined, and a claim inside one of its layers must stand on one of that layer's laws.
/// One document, with every declaration labelled by the file it came from.  A merge is only
/// honest if a reader can tell which side a name is on, and the diagnostics already carry that.
pub(crate) fn tagged(path: &Path, loaded: &Loaded) -> Vec<crate::syntax::Decl> {
    let name = path.display().to_string();
    let mut decls = loaded.decls.clone();
    for decl in &mut decls {
        decl.file = name.clone();
    }
    decls
}

pub(crate) fn learning_map(
    learnings: &[crate::induce::Learning],
) -> std::collections::HashMap<String, crate::induce::Learning> {
    learnings
        .iter()
        .map(|item| {
            (
                item.concept.clone(),
                crate::induce::Learning {
                    concept: item.concept.clone(),
                    positives: item.positives.clone(),
                    negatives: item.negatives.clone(),
                    clause: item.clause.clone(),
                    witness: item.witness.clone(),
                    reason: item.reason.clone(),
                },
            )
        })
        .collect()
}
