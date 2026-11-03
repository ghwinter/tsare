//! Commands that read a document and say what is wrong with it.

use super::*;
use crate::json::Json as J;

/// Errors decide the exit code.  `--strict` lets a warning decide it too: a warning is the engine
/// saying the document states something it cannot settle, and a loop that reads only the number
/// should not be able to ignore that.
fn exit_code(session: &crate::session::Session, strict: bool) -> i32 {
    if strict && session.warnings() > 0 {
        1
    } else {
        session.exit_code()
    }
}

pub(crate) fn cmd_check(
    path: &Path,
    json: bool,
    sugar: bool,
    goals_path: Option<&str>,
    use_cache: bool,
    against: Option<&str>,
    strict: bool,
) -> i32 {
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
                return session.exit_code();
            }
        }
    } else {
        match load_for_check(path) {
            Ok(loaded) => loaded,
            Err(error) => return fail(error),
        }
    };
    if let Some(kernel_path) = against {
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
            let text = match fs::read_to_string(goals_file) {
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
) -> i32 {
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
            return 1;
        }
        print!("{}", session.text());
        println!("{}", session.summary());
        println!(
            "the other document does not compile in this context; fix it before checking against it"
        );
        return 1;
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

pub(crate) fn cmd_project(path: &Path) -> i32 {
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
        1
    } else {
        0
    }
}

/// Record one run, or append it to a session trace.
pub(crate) fn cmd_trace(path: &Path, out: Option<&str>, append: Option<&str>) -> i32 {
    if out.is_some() && append.is_some() {
        println!("usage: tsare trace PATH [--out FILE | --append FILE]");
        return 2;
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
        return 0;
    }
    let trace = crate::trace::Trace::new(run);
    match out {
        Some(file) => {
            if let Err(error) = fs::write(file, trace.to_json()) {
                return fail(format!("{file}: {error}"));
            }
            print_trace_summary(&trace, file, false);
            0
        }
        None => {
            print!("{}", trace.to_json());
            0
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
pub(crate) fn cmd_replay(path: &Path, diff: bool) -> i32 {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) => return fail(format!("{}: {error}", path.display())),
    };
    let trace = match crate::trace::Trace::parse(&text) {
        Ok(trace) => trace,
        Err(error) => {
            println!("not a tsare trace: {error}");
            return 2;
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
            0
        }
        Ok(result) => {
            println!("mismatch: {}", result.detail);
            1
        }
        Err(error) => {
            println!("cannot replay: {error}");
            2
        }
    }
}

/// The loop's one line: the decision, the epistemic counts, and the next question.  Errors decide
/// the exit code; warnings are reported but do not.
pub(crate) fn cmd_status(path: &Path, json: bool, trace: Option<&str>) -> i32 {
    let loaded = match load(path) {
        Ok(loaded) => loaded,
        Err(error) => return fail(error),
    };
    let diagnostics = crate::check::check(&loaded.decls);
    let errors = diagnostics
        .iter()
        .filter(|item| item.code.starts_with('E'))
        .count();
    let warnings = diagnostics
        .iter()
        .filter(|item| item.code.starts_with('W'))
        .count();
    let verdict = crate::decide::report(&loaded.decls);
    let rows = crate::epistemic::report(&loaded.decls);
    let verified = rows
        .iter()
        .filter(|row| row.knowing == crate::epistemic::Knowing::Verified)
        .count();
    let inferred = rows
        .iter()
        .filter(|row| row.knowing == crate::epistemic::Knowing::Inferred)
        .count();
    let unverified = rows.len() - verified - inferred;
    let program = crate::program::Program::new(&loaded.decls);
    let question = crate::ask::questions(&program)
        .into_iter()
        .next()
        .map(|item| item.question);
    let first = match diagnostics.first() {
        Some(item) => J::Obj(vec![
            ("code".to_string(), J::Str(item.code.to_string())),
            (
                "name".to_string(),
                J::Str(item.witness.clone().unwrap_or_default()),
            ),
            ("message".to_string(), J::Str(item.message.clone())),
        ]),
        None => J::Null,
    };
    let debt = match verdict.items.first() {
        Some(item) => J::Obj(vec![
            ("name".to_string(), J::Str(item.name.clone())),
            ("kind".to_string(), J::Str(item.kind.clone())),
            ("reason".to_string(), J::Str(item.reason.to_string())),
        ]),
        None => J::Null,
    };
    let open_chains = crate::chain::open(&loaded.decls).len();
    let value = J::Obj(vec![
        (
            "decision".to_string(),
            J::Str(verdict.decision.label().to_string()),
        ),
        ("verified".to_string(), J::Num(verified as f64)),
        ("inferred".to_string(), J::Num(inferred as f64)),
        ("unverified".to_string(), J::Num(unverified as f64)),
        ("errors".to_string(), J::Num(errors as f64)),
        ("warnings".to_string(), J::Num(warnings as f64)),
        (
            "question".to_string(),
            match &question {
                Some(text) => J::Str(text.clone()),
                None => J::Null,
            },
        ),
        ("first".to_string(), first),
        ("claims".to_string(), J::Num(loaded.decls.len() as f64)),
        ("debt".to_string(), debt),
        ("open_chains".to_string(), J::Num(open_chains as f64)),
    ]);
    let report = format!("{}\n", crate::json::serialize(&value));
    if json {
        print!("{report}");
    } else {
        println!("{}", crate::status::line(&loaded.decls));
    }
    if let Some(file) = trace {
        let status = if errors > 0 { 1 } else { 0 };
        let step = crate::trace::Step {
            command: "status".to_string(),
            status,
            report: report.clone(),
        };
        let run = crate::trace::Run::new(&loaded.text, vec![step]);
        if let Err(error) = crate::trace::append(Path::new(file), run) {
            eprintln!("trace: {error}");
        }
    }
    if errors > 0 {
        1
    } else {
        0
    }
}

/// Where each quoted claim sits in its source: the source, the line and the column.
pub(crate) fn cmd_provenance(path: &Path, json: bool) -> i32 {
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
        return 0;
    }
    if rows.is_empty() {
        println!("no quoted claims");
        return 0;
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
    0
}

/// Store a ledger under its content hash.
pub(crate) fn cmd_save(path: &Path, store: &str) -> i32 {
    match crate::version::save(path, Path::new(store)) {
        Ok(hash) => {
            let short: String = hash.chars().take(12).collect();
            println!("saved {short} ({store})");
            0
        }
        Err(error) => fail(error),
    }
}

/// The content hashes the store holds.
pub(crate) fn cmd_log(store: &Path) -> i32 {
    match crate::version::list(store) {
        Ok(hashes) if hashes.is_empty() => {
            println!("no saved versions");
            0
        }
        Ok(hashes) => {
            for hash in hashes {
                println!("{hash}");
            }
            0
        }
        Err(error) => fail(error),
    }
}

/// Write a stored ledger back, refusing bytes that do not match the hash.
pub(crate) fn cmd_restore(hash: &str, store: &str, out: &str) -> i32 {
    match crate::version::restore(Path::new(store), hash, Path::new(out)) {
        Ok(()) => {
            let short: String = hash.chars().take(12).collect();
            println!("restored {short} -> {out}");
            0
        }
        Err(error) => fail(error),
    }
}

/// Add one declaration from a source document to a library, if the library stays admissible.
pub(crate) fn cmd_grow(library: &Path, source: &Path, name: &str, out: &str) -> i32 {
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
            0
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
            1
        }
    }
}
