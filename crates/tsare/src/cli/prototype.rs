//! Prototype commands.  Each one is a thin shell over `tsare-proto`: they read a document, call
//! one function, and print.  They are here to be argued about, not relied on -- `check` is still
//! the only thing that decides whether a document is well formed.

use super::*;

/// Given a goal in words, which command reads this document for it.
pub(crate) fn cmd_route(path: &Path, goal: &str) -> Outcome {
    let loaded = match load(path) {
        Ok(loaded) => loaded,
        Err(error) => return fail(error),
    };
    let mut kinds: Vec<&str> = loaded.decls.iter().map(|decl| decl.kind.as_str()).collect();
    kinds.sort_unstable();
    kinds.dedup();
    let commands: Vec<(&str, &str)> = registry::COMMANDS
        .iter()
        .map(|command| (command.name, command.usage))
        .collect();
    let ranked = tsare_proto::route::rank(goal, &commands, &kinds);
    if ranked.is_empty() {
        println!("nothing in the command table speaks to that goal");
        return Outcome::Ok;
    }
    for route in ranked.iter().take(5) {
        println!("{:<28} {:.2}  {}", route.name, route.score, route.why);
    }
    Outcome::Ok
}

/// Whether every fact, rule and attack in the document can hold at once.
pub(crate) fn cmd_sat(path: &Path) -> Outcome {
    let loaded = match load(path) {
        Ok(loaded) => loaded,
        Err(error) => return fail(error),
    };
    let answer = tsare_proto::sat::solve(&loaded.decls);
    if answer.satisfiable {
        println!(
            "satisfiable: a model sets {} name(s) true",
            answer.model.len()
        );
        return Outcome::Ok;
    }
    println!("unsatisfiable; the contradictory core is:");
    for line in &answer.core {
        println!("  {line}");
    }
    Outcome::Ok
}

/// What follows when a value is set rather than observed.
pub(crate) fn cmd_causal(path: &Path, do_name: &str) -> Outcome {
    let loaded = match load(path) {
        Ok(loaded) => loaded,
        Err(error) => return fail(error),
    };
    if !loaded.decls.iter().any(|decl| decl.name == do_name) {
        return fail(format!("{do_name} is not declared"));
    }
    let effect = tsare_proto::cause::intervene(&loaded.decls, do_name);
    println!(
        "observing reaches {} name(s); setting {do_name} reaches {}",
        effect.observed.len(),
        effect.intervened.len()
    );
    for name in &effect.gained {
        println!("  gains {name}");
    }
    for name in &effect.lost {
        println!("  loses {name}");
    }
    Outcome::Ok
}

/// Which declarations are near each other in their own words.
pub(crate) fn cmd_similar(path: &Path, top: usize) -> Outcome {
    let loaded = match load(path) {
        Ok(loaded) => loaded,
        Err(error) => return fail(error),
    };
    let pairs = tsare_proto::index::nearest(&loaded.decls, top);
    if pairs.is_empty() {
        println!("no two declarations share a word");
        return Outcome::Ok;
    }
    for pair in pairs {
        println!(
            "{:.2}  {:<24} {:<24}  {}",
            pair.score,
            pair.left,
            pair.right,
            pair.shared.join(" ")
        );
    }
    Outcome::Ok
}
