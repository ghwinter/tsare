//! Commands that turn a document into something else.

use super::*;

pub(crate) fn cmd_audit(path: &Path) -> i32 {
    let loaded = match load(path) {
        Ok(loaded) => loaded,
        Err(error) => return fail(error),
    };
    let rows = crate::audit::audit(&loaded.decls);
    println!("kind audit: {}", path.display());
    for row in &rows {
        if !row.present {
            println!("  {} absent", pad(row.kind, 8));
            continue;
        }
        let lost_text = if row.lost.is_empty() {
            "-".to_string()
        } else {
            row.lost.join(", ")
        };
        let added_text = if row.added.is_empty() {
            "-".to_string()
        } else {
            row.added.join(", ")
        };
        let verdict = if row.lost.is_empty() && row.added.is_empty() {
            "eliminable: no diagnostic changes"
        } else if !row.lost.is_empty() && row.added.is_empty() {
            "NOT eliminable: the check disappears"
        } else if !row.lost.is_empty() && !row.added.is_empty() {
            "obligation survives in another code"
        } else {
            "rewrite adds diagnostics"
        };
        println!(
            "  {} present  lost: {} added: {} {verdict}",
            pad(row.kind, 8),
            pad(&lost_text, 28),
            pad(&added_text, 22)
        );
    }
    0
}

pub(crate) fn cmd_egraph(path: &Path, write: Option<&str>) -> i32 {
    let loaded = match load(path) {
        Ok(loaded) => loaded,
        Err(error) => return fail(error),
    };
    let program = Program::new(&loaded.decls);
    let (mut uf, groups) = crate::egraph::merge_groups(&program);
    let merged: Vec<&Vec<String>> = groups.iter().filter(|group| group.len() > 1).collect();
    println!("{} classes, {} merged", groups.len(), merged.len());
    for group in &merged {
        println!("  = {}", group.join("  "));
    }
    if let Some(out) = write {
        let mapping = crate::egraph::canonical_map(&mut uf);
        let new_decls = crate::egraph::rewrite(&loaded.decls, &mapping);
        fs::write(out, crate::fix::render(&new_decls)).expect("write file");
        let reloaded = match load(Path::new(out)) {
            Ok(reloaded) => reloaded,
            Err(error) => {
                println!("rewritten document does not parse: {error}");
                return 1;
            }
        };
        let before = crate::check::check(&loaded.decls);
        let after = crate::check::check(&reloaded.decls);
        println!(
            "wrote {out}: errors {}->{}, warnings {}->{}, declarations {}->{}",
            error_count(&before),
            error_count(&after),
            warning_count(&before),
            warning_count(&after),
            loaded.decls.len(),
            new_decls.len()
        );
    }
    0
}

pub(crate) fn cmd_graph(path: &Path, out: Option<&str>) -> i32 {
    let loaded = match load(path) {
        Ok(loaded) => loaded,
        Err(error) => return fail(error),
    };
    let program = Program::new(&loaded.decls);
    let text = crate::graph::to_mermaid(&program);
    let destination = match out {
        Some(target) => PathBuf::from(target),
        None => path.with_extension("net.mmd"),
    };
    fs::write(&destination, text).expect("write graph");
    let (nodes, edges) = crate::graph::summary(&program);
    println!("{nodes} nodes, {edges} edges -> {}", destination.display());
    let diagnostics = crate::check::check(&loaded.decls);
    let errors = error_count(&diagnostics);
    if errors > 0 {
        println!("note: {errors} error(s); dangling edges are part of the picture");
    }
    0
}

pub(crate) fn cmd_map(left: &Path, right: &Path, top: usize) -> i32 {
    let left_doc = match load(left) {
        Ok(loaded) => loaded,
        Err(error) => return fail(error),
    };
    let right_doc = match load(right) {
        Ok(loaded) => loaded,
        Err(error) => return fail(error),
    };
    let (rows, total) = crate::mapping::align(
        &Program::new(&left_doc.decls),
        &Program::new(&right_doc.decls),
        top,
    );
    if rows.is_empty() {
        println!("no shared structure");
        return 0;
    }
    for row in &rows {
        let shown: Vec<String> = row
            .shared
            .iter()
            .take(4)
            .map(|path| path.join(">"))
            .collect();
        println!(
            "{:.2} w={} {} ~ {}  shared: {}",
            row.score,
            pad(&row.weight.to_string(), 2),
            pad(&row.name_a, 18),
            pad(&row.name_b, 18),
            shown.join(", ")
        );
    }
    if total > rows.len() {
        println!("… {} more pair(s); raise --top", total - rows.len());
    }
    0
}

pub(crate) fn cmd_induce(path: &Path) -> i32 {
    let loaded = match load(path) {
        Ok(loaded) => loaded,
        Err(error) => return fail(error),
    };
    let program = Program::new(&loaded.decls);
    let diagnostics = crate::check::check(&loaded.decls);
    let errors = error_count(&diagnostics);
    if errors > 0 {
        println!(
            "note: {errors} error(s) in {}; learning ran on what resolved",
            path.display()
        );
    }
    let learnings: Vec<crate::induce::Learning> = program
        .of_kind("concept")
        .iter()
        .map(|concept| crate::induce::induce_concept(&program, concept))
        .collect();
    println!("{} {:>3} {:>3}  result", pad("concept", 16), "pos", "neg");
    for learning in &learnings {
        let result = match &learning.clause {
            Some(clause) => format!("learned: {}", crate::induce::clause_text(clause)),
            None => format!("FAILED: {}", learning.reason),
        };
        println!(
            "{} {:>3} {:>3}  {result}",
            pad(&learning.concept, 16),
            learning.positives.len(),
            learning.negatives.len()
        );
    }
    for learning in learnings.iter().filter(|item| item.clause.is_none()) {
        println!();
        if learning.witness.is_none() {
            println!("{}: blocked ({})", learning.concept, learning.reason);
            continue;
        }
        let (asks, answers) = crate::induce::auto_question(learning);
        println!("derived question for {}:", learning.concept);
        println!("  {asks}");
        for (index, answer) in answers.iter().enumerate() {
            println!("    {}. {answer}", index + 1);
        }
    }
    0
}

pub(crate) fn cmd_pattern(path: &Path, name: &str, with_vars: bool) -> i32 {
    let loaded = match load(path) {
        Ok(loaded) => loaded,
        Err(error) => return fail(error),
    };
    let program = Program::new(&loaded.decls);
    let Some(concept) = program.get(name) else {
        println!("{name}: not a concept");
        return 1;
    };
    if concept.kind != "concept" {
        println!("{name}: not a concept");
        return 1;
    }
    let positives = program::pos(concept);
    let negatives = program::neg(concept);
    println!("{}:", concept.name);
    println!(
        "  positives: {}",
        if positives.is_empty() {
            "-".to_string()
        } else {
            positives.join(", ")
        }
    );
    println!(
        "  negatives: {}",
        if negatives.is_empty() {
            "-".to_string()
        } else {
            negatives.join(", ")
        }
    );
    if with_vars {
        let pattern = crate::patterns::learn(&program, &positives, &negatives);
        match pattern {
            Some(pattern) => println!("  structure: {}", crate::patterns::render(&pattern)),
            None => println!("  structure: none found with variables"),
        }
        return 0;
    }
    let exclude: std::collections::HashSet<String> = [concept.name.clone()].into_iter().collect();
    let rule = crate::paths::learn(&program, &positives, &negatives, 2, &exclude);
    match &rule {
        Some(rule) => {
            println!("  structure: {}", rule.join(" and "));
            let expression: Vec<String> = rule
                .iter()
                .map(|feature| format!("\"{feature}\" in features"))
                .collect();
            println!(
                "  predicate: def {}(features): return {}",
                concept.name,
                expression.join(" and ")
            );
        }
        None => println!("  structure: none found up to two edges"),
    }
    println!("  positions:");
    for name in positives.iter().chain(negatives.iter()) {
        let side = if positives.contains(name) {
            "pos"
        } else {
            "neg"
        };
        let mut found: Vec<String> = crate::paths::features(&program, name, 2, &exclude)
            .into_iter()
            .collect();
        found.sort();
        println!(
            "    {side} {name}: {}",
            if found.is_empty() {
                "-".to_string()
            } else {
                found.join(", ")
            }
        );
    }
    0
}

pub(crate) fn cmd_build(path: &Path, out: &str, goals_path: Option<&str>) -> i32 {
    let loaded = match load(path) {
        Ok(loaded) => loaded,
        Err(error) => return fail(error),
    };
    let program = Program::new(&loaded.decls);
    let out_dir = PathBuf::from(out);
    if let Err(error) = fs::create_dir_all(&out_dir) {
        return fail(format!("{}: {error}", out_dir.display()));
    }
    if path.is_dir() {
        let learnings = crate::build::learnings_of(&program);
        fs::write(
            out_dir.join("OBLIGATIONS.md"),
            crate::build::render_obligations(&program, &learning_map(&learnings)),
        )
        .expect("write");
        fs::write(
            out_dir.join("LEARNED.md"),
            crate::build::render_learned(&learnings, &[]),
        )
        .expect("write");
        println!(
            "project {}: {} declarations, emitted {}",
            path.display(),
            program.decls.len(),
            out_dir.display()
        );
        return 0;
    }

    let diagnostics = crate::check::check(&loaded.decls);
    {
        let mut session =
            crate::session::Session::new(path.display().to_string()).with_source(&loaded.text);
        session.extend(diagnostics.clone());
        print!("{}", session.text());
        println!("{}", session.summary());
    }
    let mut goals = None;
    let mut all = diagnostics.clone();
    if let Some(goals_file) = goals_path {
        match fs::read_to_string(goals_file) {
            Ok(text) => {
                let parsed = crate::coverage::load_goals(&text);
                let report = crate::coverage::coverage(&program, &parsed);
                all.extend(crate::coverage::diagnostics(&report));
                goals = Some(report);
            }
            Err(error) => return fail(format!("{goals_file}: {error}")),
        }
    }
    if let Some(report) = &goals {
        let covered = report.iter().filter(|goal| goal.covered()).count();
        println!("coverage: {covered}/{} goals addressed", report.len());
    }
    let holes = collect_holes(&loaded.decls);
    if !holes.is_empty() {
        println!(
            "holes: {} unknown value(s), listed in OPEN.md after a build",
            holes.len()
        );
    }
    if error_count(&all) > 0 {
        println!("not emitted: fix the errors first");
        return 1;
    }

    let learnings = crate::build::learnings_of(&program);
    let mut structurals: Vec<(String, Option<crate::patterns::Pattern>)> = Vec::new();
    for concept in program.of_kind("concept") {
        let rule = crate::patterns::learn(&program, &program::pos(concept), &program::neg(concept));
        structurals.push((concept.name.clone(), rule));
    }
    let mapping = learning_map(&learnings);
    fs::write(
        out_dir.join("OBLIGATIONS.md"),
        crate::build::render_obligations(&program, &mapping),
    )
    .expect("write");
    fs::write(
        out_dir.join("LEARNED.md"),
        crate::build::render_learned(&learnings, &structurals),
    )
    .expect("write");
    fs::write(
        out_dir.join("OPEN.md"),
        crate::build::render_open(&program, &learnings, &holes),
    )
    .expect("write");
    fs::write(
        out_dir.join("NOTES.md"),
        crate::build::render_notes(&program),
    )
    .expect("write");
    let source = crate::build::predicates_source(
        &program,
        &mapping,
        &structurals,
        &path
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_default(),
    );
    fs::write(out_dir.join("predicates.py"), &source).expect("write");
    for name in [
        "OBLIGATIONS.md",
        "LEARNED.md",
        "OPEN.md",
        "NOTES.md",
        "predicates.py",
    ] {
        println!("emitted {}", out_dir.join(name).display());
    }
    let defined = learnings
        .iter()
        .filter(|item| item.clause.is_some())
        .count();
    let structured = structurals
        .iter()
        .filter(|(_, rule)| rule.is_some())
        .count();
    let problems = crate::build::verify_predicates(&program, &mapping, &structurals);
    if !problems.is_empty() {
        println!("predicates: FAILED on {} examples", problems.len());
        for problem in &problems {
            println!("  {problem}");
        }
        return 1;
    }
    println!(
        "predicates: {defined} attribute + {structured} structural, verified on every example"
    );
    if let Some(report) = &goals {
        fs::write(
            out_dir.join("COVERAGE.md"),
            crate::build::render_coverage(report),
        )
        .expect("write");
        println!("emitted {}", out_dir.join("COVERAGE.md").display());
    }
    0
}

pub(crate) fn cmd_outline(path: &Path) -> i32 {
    let loaded = match load(path) {
        Ok(loaded) => loaded,
        Err(error) => return fail(error),
    };
    let program = Program::new(&loaded.decls);
    let diagnostics = crate::check::check(&loaded.decls);
    print!(
        "{}",
        crate::outline::markdown(&program, &diagnostics, &path.display().to_string())
    );
    0
}

pub(crate) fn cmd_mem(recall: Option<&[String]>, k: usize) -> i32 {
    if let Some(codes) = recall {
        let entries = crate::cases::recall(codes, k);
        if entries.is_empty() {
            println!("no similar case");
            return 0;
        }
        for entry in entries {
            let outcome = if entry.improved {
                "improved"
            } else {
                "no better"
            };
            let codes = entry
                .entry
                .get("codes")
                .map(|value| value.strings())
                .unwrap_or_default();
            let action = entry
                .entry
                .get("action")
                .and_then(|value| value.as_str())
                .unwrap_or("");
            let errors = |side: &str| {
                entry
                    .entry
                    .get(side)
                    .and_then(|item| item.get("errors"))
                    .and_then(|item| item.as_f64())
                    .unwrap_or(0.0)
            };
            println!(
                "{} {} {}->{} errors  ({outcome})",
                crate::cases::list_repr(&codes),
                pad(action, 10),
                crate::program::fmt_g(errors("before")),
                crate::program::fmt_g(errors("after"))
            );
        }
        return 0;
    }
    let report = crate::cases::audit();
    println!("cases: {}", report.entries);
    println!(
        "expired:     {}  (recorded under a different checker)",
        report.expired
    );
    println!(
        "conflicting: {}  (same situation and action, opposite outcome)",
        report.conflicting
    );
    println!("no-op:       {}  (changed nothing either way)", report.noop);
    println!("unchanged:   {}", report.unchanged);
    println!(
        "orphans:     {}  (document no longer exists)",
        report.orphans
    );
    0
}
