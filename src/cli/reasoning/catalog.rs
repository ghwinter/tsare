use super::*;

pub(crate) fn cmd_cat(path: &Path) -> Outcome {
    let loaded = match load(path) {
        Ok(loaded) => loaded,
        Err(error) => return fail(error),
    };
    let program = Program::new(&loaded.decls);
    let diagnostics = crate::check::check(&loaded.decls);
    let objects: Vec<String> = crate::cat::objects(&program)
        .iter()
        .map(|(kind, count)| format!("{kind}={count}"))
        .collect();
    let arrows: Vec<String> = crate::cat::arrows(&program)
        .iter()
        .map(|(kind, count)| format!("{kind}={count}"))
        .collect();
    println!("objects: {}", objects.join(", "));
    println!("arrows:  {}", arrows.join(", "));
    for functor in crate::cat::FUNCTORS {
        let outcome = (functor.check)(&program);
        println!(
            "{:<8} {}/{}  {}  [{}]",
            format!("{}:", functor.name),
            outcome.kept,
            outcome.total,
            functor.law,
            functor.code
        );
        for failure in &outcome.failures {
            println!("  FAIL {failure}");
        }
    }
    println!("checker: {} error(s)", error_count(&diagnostics));
    let errors = error_count(&diagnostics);
    println!("stages:");
    for stage in crate::cat::stages() {
        println!(
            "  {} {} {}",
            pad(stage.name, 24),
            pad(stage.label, 16),
            stage.why
        );
    }
    if errors > 0 {
        Outcome::Failed
    } else {
        Outcome::Ok
    }
}

pub(crate) fn cmd_moves(path: &Path) -> Outcome {
    let loaded = match load(path) {
        Ok(loaded) => loaded,
        Err(error) => return fail(error),
    };
    let program = Program::new(&loaded.decls);
    let rows = crate::moves::suggest(&program);
    if rows.is_empty() {
        println!("no representation move suggested");
        return Outcome::Ok;
    }
    let names = crate::moves::moves();
    for row in &rows {
        let label = names
            .iter()
            .find(|(code, _)| *code == row.code)
            .map(|(_, label)| *label)
            .unwrap_or("");
        println!(
            "{} {} {} {}",
            row.code,
            pad(label, 26),
            pad(&row.target, 18),
            row.why
        );
    }
    Outcome::Ok
}

pub(crate) fn cmd_fca(path: &Path) -> Outcome {
    let loaded = match load(path) {
        Ok(loaded) => loaded,
        Err(error) => return fail(error),
    };
    let program = Program::new(&loaded.decls);
    let mut printed = 0;
    for concept in crate::fca::concepts(&program) {
        printed += 1;
        println!("{}", concept.name);
        println!("  listed        {} positive(s)", concept.listed);
        println!(
            "  intent        {}",
            crate::fca::render_pairs(&concept.intent)
        );
        if concept.implied.is_empty() {
            println!("  implied       -");
        } else {
            println!("  implied       {}", concept.implied.join(", "));
        }
    }
    for pile in crate::fca::piles(&program) {
        printed += 1;
        println!("{}", pile.name);
        println!("  elements      {}", pile.elements);
        println!("  shared        {}", crate::fca::render_pairs(&pile.shared));
        if pile.shared.is_empty() {
            println!("  verdict       the pile has no recorded common property, so nothing separates it yet");
        }
    }
    if printed == 0 {
        println!("no concept or pile to analyse");
    }
    Outcome::Ok
}

pub(crate) fn cmd_kinds() -> Outcome {
    println!("{} {} discharge", pad("kind", 12), pad("obligation", 52));
    for (kind, obligation, discharge) in program::TABLE {
        println!("{} {} {}", pad(kind, 12), pad(obligation, 52), discharge);
    }
    Outcome::Ok
}
