//! Commands that read a document and say what is wrong with it.

use super::*;

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
                path: path.to_path_buf(),
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
