//! Commands that read a document and say what is wrong with it.

use super::*;

pub(crate) fn cmd_check(
    path: &Path,
    json: bool,
    sugar: bool,
    goals_path: Option<&str>,
    use_cache: bool,
    against: Option<&str>,
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
            Err(error) => return fail(format!("{}: {error}", path.display())),
        }
    } else {
        match load_for_check(path) {
            Ok(loaded) => loaded,
            Err(error) => return fail(error),
        }
    };
    if let Some(kernel_path) = against {
        return check_against(path, &loaded, Path::new(kernel_path), json);
    }
    let display = path.display().to_string();
    let mut session = crate::session::Session::new(display)
        .with_source(&loaded.text)
        .with_sources(&loaded.sources);
    session.extend(loaded.syntax.clone());
    let mut coverage = None;
    if loaded.syntax.is_empty() {
        let cacheable = use_cache && !sugar && !path.is_dir();
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
        return session.exit_code();
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
    session.exit_code()
}

pub(crate) fn check_against(path: &Path, loaded: &Loaded, kernel_path: &Path, json: bool) -> i32 {
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
        return session.exit_code();
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
    session.exit_code()
}

pub(crate) fn cmd_project(path: &Path) -> i32 {
    let loaded = match load(path) {
        Ok(loaded) => loaded,
        Err(error) => return fail(error),
    };
    let diagnostics = crate::check::check(&loaded.decls);
    let mut by_file: Vec<(String, Vec<&crate::check::Diagnostic>)> = Vec::new();
    for item in &diagnostics {
        match by_file.iter_mut().find(|(file, _)| file == &item.file) {
            Some((_, list)) => list.push(item),
            None => by_file.push((item.file.clone(), vec![item])),
        }
    }
    by_file.sort_by(|left, right| left.0.cmp(&right.0));
    for (file, list) in &by_file {
        println!("# {file}");
        for item in list {
            println!("  {} {file}:{} {}", item.code, item.line, item.message);
        }
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
