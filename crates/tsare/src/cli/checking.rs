//! Commands that read a document and say what is wrong with it.

use super::*;
use crate::json::Json as J;

/// The definitions a rendered page states: a heading and the term column the caller names (or the
/// bold first cell when it names none).  A directory is read page by page in name order, so one
/// chapter list becomes one document with one numbering.
pub(crate) fn cmd_extract(
    path: &Path,
    json: bool,
    terms: Option<&str>,
    write: Option<&str>,
) -> Outcome {
    let pages: Vec<std::path::PathBuf> = if path.is_dir() {
        let mut pages: Vec<std::path::PathBuf> = match fs::read_dir(path) {
            Ok(entries) => entries
                .filter_map(|entry| entry.ok().map(|item| item.path()))
                .filter(|item| item.extension().map(|ext| ext == "html").unwrap_or(false))
                .collect(),
            Err(error) => return fail(format!("{}: {error}", path.display())),
        };
        pages.sort();
        pages
    } else {
        vec![path.to_path_buf()]
    };
    let terms: Vec<&str> = terms
        .map(|list| {
            list.split(',')
                .map(str::trim)
                .filter(|word| !word.is_empty())
                .collect()
        })
        .unwrap_or_default();
    let mut found = Vec::new();
    for page in &pages {
        let text = match fs::read_to_string(page) {
            Ok(text) => text,
            Err(error) => return fail(format!("{}: {error}", page.display())),
        };
        found.extend(crate::extract::definitions(
            &text,
            &page.display().to_string(),
            &terms,
        ));
    }
    if let Some(out) = write {
        return write_definitions(&found, Path::new(out));
    }
    if json {
        let items: Vec<J> = found
            .iter()
            .map(|item| {
                J::Obj(vec![
                    ("term".to_string(), J::Str(item.term.clone())),
                    ("section".to_string(), J::Str(item.section.clone())),
                    ("file".to_string(), J::Str(item.file.clone())),
                    ("line".to_string(), J::Num(item.line as f64)),
                    ("col".to_string(), J::Num(item.col as f64)),
                ])
            })
            .collect();
        println!(
            "{}",
            crate::json::serialize(&J::Obj(vec![("definitions".to_string(), J::Arr(items))]))
        );
        return Outcome::Ok;
    }
    if found.is_empty() {
        println!(
            "no definitions: no bold term opens a table row under a heading, and no --terms header names a column"
        );
        return Outcome::Ok;
    }
    for item in &found {
        println!(
            "defines  {:<28} -> {:<24} {}:{}:{}",
            item.section, item.term, item.file, item.line, item.col
        );
    }
    let sections: usize = found
        .iter()
        .map(|item| item.section.as_str())
        .collect::<std::collections::HashSet<_>>()
        .len();
    println!("\n{} definition(s) in {sections} section(s)", found.len());
    Outcome::Ok
}

/// The same definitions as a document the checker can read: one fact per term, its evidence the
/// page it was read from.  The extraction is a claim about the page, so the claim says what the
/// page states and where -- never what the term means.
pub(crate) fn write_definitions(found: &[crate::extract::Definition], out: &Path) -> Outcome {
    let escape = |text: &str| text.replace('\\', "\\\\").replace('"', "\\\"");
    let mut text = String::from("tsare 1\n");
    for (index, item) in found.iter().enumerate() {
        let name = format!("D_{:04}", index + 1);
        let page = std::path::Path::new(&item.file)
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| item.file.clone());
        // a document that names a Windows path in its evidence is not portable, and the goldens
        // would differ between platforms
        let page_path = item.file.replace('\\', "/");
        text.push_str(&format!(
            "\n# {page}:{} -- {}\nfact {name} {{\n  claim: \"{} defines 「{}」 under 「{}」\",\n  evidence: doc(\"{}\")\n}}\n",
            item.line,
            item.term,
            page,
            escape(&item.term),
            escape(&item.section),
            escape(&page_path)
        ));
    }
    match fs::write(out, text) {
        Ok(()) => {
            println!("{} definition(s) written to {}", found.len(), out.display());
            Outcome::Ok
        }
        Err(error) => fail(format!("{}: {error}", out.display())),
    }
}

/// Errors decide the exit code.  `--strict` lets a warning decide it too: a warning is the engine
/// saying the document states something it cannot settle, and a loop that reads only the number
/// should not be able to ignore that.
fn exit_code(session: &crate::session::Session, strict: bool) -> Outcome {
    if strict && session.warnings() > 0 {
        Outcome::Failed
    } else {
        Outcome::from_code(session.exit_code())
    }
}

/// The flags the check accepts, in one value so the call site stays readable.
pub(crate) struct CheckOptions {
    pub json: bool,
    pub sugar: bool,
    pub goals: Option<String>,
    pub cache: bool,
    pub against: Option<String>,
    pub strict: bool,
    pub budget: Option<usize>,
}

pub(crate) fn cmd_check(path: &Path, options: CheckOptions) -> Outcome {
    let CheckOptions {
        json,
        sugar,
        goals: goals_path,
        cache: use_cache,
        against,
        strict,
        budget,
    } = options;
    let loaded = if sugar {
        let raw = match fs::read_to_string(path) {
            Ok(raw) => raw,
            Err(error) => return fail(format!("{}: {error}", path.display())),
        };
        let expanded = crate::sugar::expand(&raw);
        match syntax::parse(&expanded) {
            Ok(decls) => Loaded {
                decls,
                text: expanded,
                syntax: Vec::new(),
                sources: Vec::new(),
            },
            Err(error) => {
                // `--json` is a promise about the shape of the output; a parse error is output
                // like any other, so it goes through the session instead of to the console.
                let diagnostic = crate::check::Diagnostic {
                    code: error.code,
                    message: error.message,
                    line: error.line,
                    col: error.col,
                    end_line: error.line,
                    end_col: error.col + 1,
                    file: path.display().to_string(),
                    witness: None,
                };
                let mut session =
                    crate::session::Session::new(path.display().to_string()).with_source(&expanded);
                session.emit(diagnostic);
                if json {
                    println!("{}", session.json());
                } else {
                    print!("{}", session.text());
                    println!("{}", session.summary());
                }
                return Outcome::from_code(session.exit_code());
            }
        }
    } else {
        match load_for_check(path) {
            Ok(loaded) => loaded,
            Err(error) => return fail(error),
        }
    };
    if let Some(limit) = budget {
        if loaded.decls.len() > limit {
            println!(
                "unfinished: {} declarations over the budget {limit}; not checked",
                loaded.decls.len()
            );
            return Outcome::Failed;
        }
    }
    if let Some(kernel_path) = against.as_deref() {
        return check_against(path, &loaded, Path::new(kernel_path), json, strict);
    }
    let display = path.display().to_string();
    let mut session = crate::session::Session::new(display)
        .with_source(&loaded.text)
        .with_sources(&loaded.sources);
    session.extend(loaded.syntax.clone());
    let mut coverage = None;
    // Recovery already kept the declarations that parsed, whether or not another file (or this
    // one) also produced E-SYNTAX.  Skipping the semantic pass because one file failed hid every
    // error in the files that did not, so fixing the first syntax error revealed the next.  The
    // semantic diagnostics are computed over what parsed, and the syntax diagnostics stay.
    {
        // The cache key is the source text, but E-OUT-MISMATCH reads an artifact's bytes.  A
        // document that names one is not content-addressed: its answer depends on files outside
        // the source, so replaying it would answer the old state.
        let names_artifact = loaded.decls.iter().any(|decl| {
            decl.fields
                .iter()
                .any(|(key, _)| key == "artifact" || key == "out")
        });
        let cacheable = use_cache && !sugar && !path.is_dir() && !names_artifact;
        let cache_key = if cacheable {
            Some(crate::cache::key(&loaded.text))
        } else {
            None
        };
        let diagnostics = match cache_key
            .as_ref()
            .and_then(|key| crate::cache::load().get(key).cloned())
        {
            Some(payload) => crate::cache::restore(&payload),
            None => {
                let computed = crate::check::check(&loaded.decls);
                if let Some(key) = &cache_key {
                    let payload = match crate::json::parse(&crate::cache::dump(&computed)) {
                        Ok(payload) => payload,
                        Err(_) => crate::json::Json::Arr(Vec::new()),
                    };
                    let mut entries: Vec<(String, crate::json::Json)> =
                        crate::cache::load().into_iter().collect();
                    entries.retain(|(existing, _)| existing != key);
                    entries.push((key.clone(), payload));
                    entries.sort_by(|left, right| left.0.cmp(&right.0));
                    let _ = crate::cache::save(&entries);
                }
                computed
            }
        };
        session.extend(diagnostics);
        if let Some(goals_file) = goals_path {
            let text = match fs::read_to_string(&goals_file) {
                Ok(text) => text,
                Err(error) => return fail(format!("{goals_file}: {error}")),
            };
            let goals = crate::coverage::load_goals(&text);
            let report = crate::coverage::coverage(&Program::new(&loaded.decls), &goals);
            session.extend(crate::coverage::diagnostics(&report));
            coverage = Some((
                report.iter().filter(|goal| goal.covered()).count(),
                report.len(),
            ));
        }
    }
    if json {
        println!("{}", session.json());
        return exit_code(&session, strict);
    }
    print!("{}", session.text());
    if let Some((covered, total)) = coverage {
        println!("coverage: {covered}/{total} goals addressed");
    }
    let holes = collect_holes(&loaded.decls);
    if !holes.is_empty() {
        println!(
            "holes: {} unknown value(s), listed in OPEN.md after a build",
            holes.len()
        );
    }
    println!("{}", session.summary());
    exit_code(&session, strict)
}

pub(crate) fn check_against(
    path: &Path,
    loaded: &Loaded,
    kernel_path: &Path,
    json: bool,
    strict: bool,
) -> Outcome {
    let kernel = match load_for_check(kernel_path) {
        Ok(kernel) => kernel,
        Err(error) => return fail(error),
    };
    let kernel_name = kernel_path.display().to_string();
    let target_name = path.display().to_string();

    let kernel_decls = tagged(kernel_path, &kernel);
    let target_decls = tagged(path, loaded);
    let mut merged = kernel_decls.clone();
    merged.extend(target_decls.clone());
    let merged_diagnostics = crate::check::check(&merged);

    // The condition is that the other document compiles *in this context*, not on its own: an
    // objection refers to the thing it objects to, so it cannot be self-contained.  What must
    // hold is that nothing on that side is unresolvable once both are present.
    let other_errors: Vec<crate::check::Diagnostic> = merged_diagnostics
        .iter()
        .filter(|item| item.file == kernel_name && item.code.starts_with('E'))
        .cloned()
        .collect();
    if !other_errors.is_empty() {
        let mut session =
            crate::session::Session::new(kernel_name.clone()).with_source(&kernel.text);
        session.extend(other_errors);
        if json {
            println!("{}", session.json());
            return Outcome::Failed;
        }
        print!("{}", session.text());
        println!("{}", session.summary());
        println!(
            "the other document does not compile in this context; fix it before checking against it"
        );
        return Outcome::Failed;
    }

    let shadows = crate::kernel::check(&kernel_decls, &target_decls);
    let shadowed: std::collections::HashSet<(String, usize)> = shadows
        .iter()
        .map(|item| (item.file.clone(), item.line))
        .collect();
    // the merge makes a shadowed name a plain duplicate; the kernel code says it better
    let mut diagnostics: Vec<crate::check::Diagnostic> = merged_diagnostics
        .iter()
        .filter(|item| item.file == target_name)
        .filter(|item| {
            !(item.code == "E-DUP-ID" && shadowed.contains(&(item.file.clone(), item.line)))
        })
        .cloned()
        .collect();
    diagnostics.extend(shadows);

    let mut session = crate::session::Session::new(target_name.clone())
        .with_source(&loaded.text)
        .with_sources(&[
            (kernel_name.clone(), kernel.text.clone()),
            (target_name.clone(), loaded.text.clone()),
        ]);
    session.extend(diagnostics);
    if json {
        println!("{}", session.json());
        return exit_code(&session, strict);
    }
    print!("{}", session.text());
    let kernel_warnings = merged_diagnostics
        .iter()
        .filter(|item| item.file == kernel_name && !item.code.starts_with('E'))
        .count();
    if kernel_warnings > 0 {
        println!("kernel: {kernel_warnings} warning(s) of its own");
    }
    println!("{}", session.summary());
    exit_code(&session, strict)
}

pub(crate) fn cmd_project(path: &Path) -> Outcome {
    // `project` is `check`, grouped by file: it must use the same loading path, so it sees the
    // recovered declarations and every syntax diagnostic, not just the first one.
    let loaded = match load_for_check(path) {
        Ok(loaded) => loaded,
        Err(error) => return fail(error),
    };
    let display = path.display().to_string();
    let mut diagnostics = loaded.syntax.clone();
    diagnostics.extend(crate::check::check(&loaded.decls));
    // `check` sorts through Session; project prints its own rows, so it sorts them itself.
    diagnostics.sort_by(|left, right| {
        (&left.file, left.line, left.col, left.code, &left.message).cmp(&(
            &right.file,
            right.line,
            right.col,
            right.code,
            &right.message,
        ))
    });
    let name_of = |item: &crate::check::Diagnostic| {
        if item.file.is_empty() {
            display.clone()
        } else {
            item.file.clone()
        }
    };
    let mut current: Option<String> = None;
    for item in &diagnostics {
        let file = name_of(item);
        if current.as_deref() != Some(file.as_str()) {
            println!("# {file}");
            current = Some(file.clone());
        }
        println!("  {} {file}:{} {}", item.code, item.line, item.message);
    }
    println!(
        "{} error(s), {} warning(s)",
        error_count(&diagnostics),
        warning_count(&diagnostics)
    );
    if error_count(&diagnostics) > 0 {
        Outcome::Failed
    } else {
        Outcome::Ok
    }
}

/// Record one run, or append it to a session trace.
pub(crate) fn cmd_trace(path: &Path, out: Option<&str>, append: Option<&str>) -> Outcome {
    if out.is_some() && append.is_some() {
        println!("usage: tsare trace PATH [--out FILE | --append FILE]");
        return Outcome::Usage;
    }
    let run = match crate::trace::record(path) {
        Ok(run) => run,
        Err(error) => return fail(error),
    };
    if let Some(file) = append {
        let trace = match crate::trace::append(Path::new(file), run) {
            Ok(trace) => trace,
            Err(error) => return fail(error),
        };
        print_trace_summary(&trace, file, true);
        return Outcome::Ok;
    }
    let trace = crate::trace::Trace::new(run);
    match out {
        Some(file) => {
            if let Err(error) = fs::write(file, trace.to_json()) {
                return fail(format!("{file}: {error}"));
            }
            print_trace_summary(&trace, file, false);
            Outcome::Ok
        }
        None => {
            print!("{}", trace.to_json());
            Outcome::Ok
        }
    }
}

/// The line that says what was recorded: how many runs, the last ledger, the last step statuses.
fn print_trace_summary(trace: &crate::trace::Trace, file: &str, session: bool) {
    let Some(last) = trace.runs.last() else {
        println!("trace: {file} (no runs)");
        return;
    };
    let short: String = last.ledger_sha256.chars().take(12).collect();
    let steps: Vec<String> = last
        .steps
        .iter()
        .map(|step| format!("{} {}", step.command, step.status))
        .collect();
    println!(
        "trace: {file} ({} {}, ledger {short}, {})",
        if session { "runs" } else { "run" },
        trace.runs.len(),
        steps.join(", ")
    );
}

/// Re-run a recorded session and compare every report and status; `diff` shows the write-backs.
pub(crate) fn cmd_replay(path: &Path, diff: bool) -> Outcome {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) => return fail(format!("{}: {error}", path.display())),
    };
    let trace = match crate::trace::Trace::parse(&text) {
        Ok(trace) => trace,
        Err(error) => {
            println!("not a tsare trace: {error}");
            return Outcome::Usage;
        }
    };
    if diff {
        if trace.runs.len() < 2 {
            println!("one run; nothing to diff");
        }
        for (index, pair) in trace.runs.windows(2).enumerate() {
            match crate::trace::change(&pair[0].ledger, &pair[1].ledger) {
                Some(change) => {
                    println!("run {} -> {}:", index + 1, index + 2);
                    print!("{change}");
                }
                None => println!("run {} -> {}: unchanged", index + 1, index + 2),
            }
        }
    }
    match crate::trace::replay(&trace) {
        Ok(result) if result.matched => {
            println!("match");
            Outcome::Ok
        }
        Ok(result) => {
            println!("mismatch: {}", result.detail);
            Outcome::Failed
        }
        Err(error) => {
            println!("cannot replay: {error}");
            Outcome::Usage
        }
    }
}

/// The loop's one line: the decision, the epistemic counts, and the next question.  Errors decide
/// the exit code; warnings are reported but do not.
pub(crate) fn cmd_status(
    path: &Path,
    json: bool,
    trace: Option<&str>,
    budget: Option<usize>,
) -> Outcome {
    let loaded = match load(path) {
        Ok(loaded) => loaded,
        Err(error) => return fail(error),
    };
    let report = match budget {
        Some(limit) => crate::status::report_with_budget(&loaded.decls, limit),
        None => crate::status::report(&loaded.decls),
    };
    let mut sink = crate::out::Stdout;
    crate::out::render(&report, json, &mut sink);
    let rendered_json = crate::json::serialize(&report.json());
    let unfinished = report
        .budget
        .as_ref()
        .map(|state| state.exceeded)
        .unwrap_or(false);
    if let Some(file) = trace {
        let status = if report.errors > 0 || unfinished {
            1
        } else {
            0
        };
        let step = crate::trace::Step {
            command: "status".to_string(),
            status,
            report: format!("{rendered_json}\n"),
        };
        let run = crate::trace::Run::new(&loaded.text, vec![step]);
        if let Err(error) = crate::trace::append(Path::new(file), run) {
            eprintln!("trace: {error}");
        }
    }
    if report.errors > 0 || unfinished {
        Outcome::Failed
    } else {
        Outcome::Ok
    }
}

/// Where each quoted claim sits in its source: the source, the line and the column.
pub(crate) fn cmd_provenance(path: &Path, json: bool) -> Outcome {
    let loaded = match load(path) {
        Ok(loaded) => loaded,
        Err(error) => return fail(error),
    };
    let rows = crate::provenance::rows(&loaded.decls);
    if json {
        let items: Vec<J> = rows
            .iter()
            .map(|row| {
                let line = match row.line {
                    Some(line) => J::Num(line as f64),
                    None => J::Null,
                };
                let col = match row.col {
                    Some(col) => J::Num(col as f64),
                    None => J::Null,
                };
                J::Obj(vec![
                    ("name".to_string(), J::Str(row.name.clone())),
                    ("kind".to_string(), J::Str(row.kind.clone())),
                    ("source".to_string(), J::Str(row.source.clone())),
                    ("source_name".to_string(), J::Str(row.source_name.clone())),
                    ("line".to_string(), line),
                    ("col".to_string(), col),
                    ("quote".to_string(), J::Str(row.quote.clone())),
                ])
            })
            .collect();
        let value = J::Obj(vec![("rows".to_string(), J::Arr(items))]);
        println!("{}", crate::json::serialize(&value));
        return Outcome::Ok;
    }
    if rows.is_empty() {
        println!("no quoted claims");
        return Outcome::Ok;
    }
    for row in &rows {
        let place = match (row.line, row.col) {
            (Some(line), Some(col)) => format!("{line}:{col}"),
            _ => "not found".to_string(),
        };
        println!(
            "{} {} {} ({}): {place} {}",
            pad(&row.name, 20),
            pad(&row.kind, 10),
            pad(&row.source, 16),
            row.source_name,
            row.quote
        );
    }
    Outcome::Ok
}

/// Store a ledger under its content hash.
pub(crate) fn cmd_save(path: &Path, store: &str) -> Outcome {
    match crate::version::save(path, Path::new(store)) {
        Ok(hash) => {
            let short: String = hash.chars().take(12).collect();
            println!("saved {short} ({store})");
            Outcome::Ok
        }
        Err(error) => fail(error),
    }
}

/// The content hashes the store holds.
pub(crate) fn cmd_log(store: &Path) -> Outcome {
    match crate::version::list(store) {
        Ok(hashes) if hashes.is_empty() => {
            println!("no saved versions");
            Outcome::Ok
        }
        Ok(hashes) => {
            for hash in hashes {
                println!("{hash}");
            }
            Outcome::Ok
        }
        Err(error) => fail(error),
    }
}

/// Write a stored ledger back, refusing bytes that do not match the hash.
pub(crate) fn cmd_restore(hash: &str, store: &str, out: &str) -> Outcome {
    match crate::version::restore(Path::new(store), hash, Path::new(out)) {
        Ok(()) => {
            let short: String = hash.chars().take(12).collect();
            println!("restored {short} -> {out}");
            Outcome::Ok
        }
        Err(error) => fail(error),
    }
}

/// Add one declaration from a source document to a library, if the library stays admissible.
pub(crate) fn cmd_grow(library: &Path, source: &Path, name: &str, out: &str) -> Outcome {
    let library_text = match fs::read_to_string(library) {
        Ok(text) => text,
        Err(error) => return fail(format!("{}: {error}", library.display())),
    };
    let source_text = match fs::read_to_string(source) {
        Ok(text) => text,
        Err(error) => return fail(format!("{}: {error}", source.display())),
    };
    match crate::growth::grow(&library_text, &source_text, name) {
        crate::growth::Verdict::Accepted(grown) => {
            if let Err(error) = fs::write(out, &grown.text) {
                return fail(format!("{out}: {error}"));
            }
            let short: String = grown.source_sha256.chars().take(12).collect();
            println!(
                "accepted {} (source {short}, {} warning(s)) -> {out}",
                grown.name, grown.warnings
            );
            Outcome::Ok
        }
        crate::growth::Verdict::Rejected {
            reason,
            diagnostics,
        } => {
            println!("rejected {name}: {reason}");
            for item in &diagnostics {
                let witness = item.witness.as_deref().unwrap_or("<file>");
                println!(
                    "  {} {} line {}: {}",
                    item.code, witness, item.line, item.message
                );
            }
            Outcome::Failed
        }
    }
}

/// The operator contracts: what each command receives, returns and touches.
pub(crate) fn cmd_contracts(json: bool) -> Outcome {
    let mut sink = crate::out::Stdout;
    crate::out::render(&crate::contracts::ContractsReport, json, &mut sink);
    Outcome::Ok
}

/// What the grown abstracts cover, and how often the cases use each.
pub(crate) fn cmd_library(path: &Path, cases: Option<&str>, json: bool) -> Outcome {
    let loaded = match load(path) {
        Ok(loaded) => loaded,
        Err(error) => return fail(error),
    };
    let report = crate::growth::report(&loaded.decls, &loaded.text, cases.map(Path::new));
    let mut sink = crate::out::Stdout;
    crate::out::render(&report, json, &mut sink);
    Outcome::Ok
}
