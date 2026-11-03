use super::*;

pub(crate) fn cmd_argue(path: &Path, against: Option<&Path>, json: bool) -> i32 {
    let loaded = match load(path) {
        Ok(loaded) => loaded,
        Err(error) => return fail(error),
    };
    let mut decls = loaded.decls.clone();
    let mut diagnostics = crate::check::check(&loaded.decls);
    // With another document, the graph is the merge: a rule in this file may be attacked by a
    // rule in that one, and the conclusion's status is computed over both.
    if let Some(other_path) = against {
        let other = match load(other_path) {
            Ok(other) => other,
            Err(error) => return fail(error),
        };
        let mut merged = tagged(other_path, &other);
        merged.extend(decls.clone());
        decls = merged;
        diagnostics = crate::check::check(&decls);
    }
    let program = Program::new(&decls);
    let mine: std::collections::HashSet<&str> =
        loaded.decls.iter().map(|decl| decl.name.as_str()).collect();
    let rows: Vec<crate::argue::Conclusion> = crate::argue::conclusions(&program)
        .into_iter()
        .filter(|row| mine.contains(row.name.as_str()))
        .collect();
    let width = rows
        .iter()
        .map(|row| row.name.chars().count())
        .max()
        .unwrap_or(10);
    let errors = error_count(&diagnostics);
    if json {
        let items: Vec<J> = rows
            .iter()
            .map(|row| {
                obj(vec![
                    ("name", J::Str(row.name.clone())),
                    ("status", J::Str(row.status.to_string())),
                    ("why", J::Str(row.why.clone())),
                ])
            })
            .collect();
        emit(obj(vec![
            ("conclusions", J::Arr(items)),
            ("errors", J::Num(errors as f64)),
        ]));
    } else {
        for row in &rows {
            println!(
                "{}  {} {}",
                pad(&row.name, width),
                pad(row.status, 10),
                row.why
            );
        }
    }
    if errors > 0 {
        if !json {
            println!(
                "note: {errors} error(s) in {}; conclusions are computed on the resolved graph",
                path.display()
            );
        }
        1
    } else {
        0
    }
}

/// Two documents attacking each other.  The report is the argument, not a verdict: `check
/// --against` decides admissibility, this shows which rules point at which, and which attacks
/// nobody answered.  Exit 1 when the first document leaves an attack unanswered.
pub(crate) fn cmd_confront(left: &Path, right: &Path) -> i32 {
    let loads = [load(left), load(right)];
    let [ours, theirs] = match loads {
        [Ok(ours), Ok(theirs)] => [ours, theirs],
        [Err(error), _] | [_, Err(error)] => return fail(error),
    };
    let left_decls = tagged(left, &ours);
    let right_decls = tagged(right, &theirs);
    let ours_names: std::collections::HashSet<&str> =
        left_decls.iter().map(|decl| decl.name.as_str()).collect();
    let theirs_names: std::collections::HashSet<&str> =
        right_decls.iter().map(|decl| decl.name.as_str()).collect();
    let mut merged = right_decls.clone();
    merged.extend(left_decls.clone());

    // who attacks whom, across the pair only: an attack inside one document is that document's
    // own business and `argue` already reports it
    let mut from_right: Vec<(String, String)> = Vec::new();
    let mut from_left: Vec<(String, String)> = Vec::new();
    for decl in merged.iter().filter(|decl| decl.kind == "warrant") {
        for target in crate::closure::field_ids(decl, "attacks") {
            let pair = (decl.name.clone(), target.clone());
            if theirs_names.contains(decl.name.as_str()) && ours_names.contains(target.as_str()) {
                from_right.push(pair);
            } else if ours_names.contains(decl.name.as_str())
                && theirs_names.contains(target.as_str())
            {
                from_left.push(pair);
            }
        }
    }
    // an attack is answered when the attacked document holds a rule that attacks the attacker
    let answers = |document: &[crate::syntax::Decl]| -> std::collections::HashSet<String> {
        document
            .iter()
            .filter(|decl| decl.kind == "warrant")
            .flat_map(|decl| crate::closure::field_ids(decl, "attacks"))
            .collect()
    };
    let our_answers = answers(&left_decls);
    let their_answers = answers(&right_decls);

    println!("confront {} {}", left.display(), right.display());
    println!();
    println!(
        "{} attacks {} ({}):",
        right.display(),
        left.display(),
        from_right.len()
    );
    let mut unanswered = 0;
    for (attacker, target) in &from_right {
        if our_answers.contains(attacker) {
            println!("  {attacker} -> {target}   answered");
        } else {
            unanswered += 1;
            println!("  {attacker} -> {target}   W-UNANSWERED");
        }
    }
    println!();
    println!(
        "{} attacks {} ({}):",
        left.display(),
        right.display(),
        from_left.len()
    );
    for (attacker, target) in &from_left {
        if their_answers.contains(attacker) {
            println!("  {attacker} -> {target}   answered");
        } else {
            println!("  {attacker} -> {target}   undefended on the other side");
        }
    }
    println!();
    println!(
        "{} attack(s) across the pair, {unanswered} unanswered on {}",
        from_right.len() + from_left.len(),
        left.display()
    );
    for (attacker, target) in &from_right {
        if !our_answers.contains(attacker) {
            println!(
                "W-UNANSWERED: {target} is attacked by {attacker}, and {} has no rule that attacks {attacker} back",
                left.display()
            );
        }
    }
    if unanswered > 0 {
        1
    } else {
        0
    }
}

pub(crate) fn cmd_debt(path: &Path, all: bool) -> i32 {
    let loaded = match load(path) {
        Ok(loaded) => loaded,
        Err(error) => return fail(error),
    };
    let program = Program::new(&loaded.decls);
    let rows = crate::debt::report(&program, !all);
    if rows.is_empty() {
        println!("every claim rests on measured facts");
        return 0;
    }
    for row in &rows {
        println!(
            "{} {} gates {}/{} facts",
            pad(&row.name, 20),
            pad(row.worst, 12),
            row.weak,
            row.total
        );
        for line in row.lines.iter().take(3) {
            println!("    {line}");
        }
        if row.lines.len() > 3 {
            println!("    … {} more", row.lines.len() - 3);
        }
    }
    0
}

pub(crate) fn cmd_impact(path: &Path) -> i32 {
    let loaded = match load(path) {
        Ok(loaded) => loaded,
        Err(error) => return fail(error),
    };
    let program = Program::new(&loaded.decls);
    let diagnostics = crate::check::check(&loaded.decls);
    for line in crate::impact::report(&program) {
        println!("{line}");
    }
    for (name, affected) in crate::impact::retirement(&program) {
        if affected.is_empty() {
            println!("retired: {name} (nothing cites it anymore)");
        } else {
            println!("retired: {name} -> {}", affected.join(", "));
        }
    }
    let errors = error_count(&diagnostics);
    if errors > 0 {
        println!("note: {errors} error(s); analyses ran on the resolved part");
        1
    } else {
        0
    }
}

pub(crate) fn cmd_weigh(path: &Path, threshold: f64) -> i32 {
    let loaded = match load(path) {
        Ok(loaded) => loaded,
        Err(error) => return fail(error),
    };
    let program = Program::new(&loaded.decls);
    let diagnostics = crate::check::check(&loaded.decls);
    let errors = error_count(&diagnostics);
    if errors > 0 {
        println!("{errors} error(s); weights are computed on the resolved part");
    }
    let rows = crate::weigh::weights(&program);
    println!("threshold {}", program::fmt_g(threshold));
    for row in rows {
        let Some(value) = row.value else {
            println!("  {} {:>5}  {}", pad(&row.name, 16), "--", row.status);
            continue;
        };
        let mark = if value < threshold { "below" } else { "ok" };
        let detail: Vec<String> = row
            .detail
            .iter()
            .map(|(rule, score)| format!("{rule}={score:.2}"))
            .collect();
        println!(
            "  {} {value:>5.2}  {} {} {}",
            pad(&row.name, 16),
            pad(row.status, 8),
            pad(mark, 5),
            detail.join("  ")
        );
    }
    if errors > 0 {
        1
    } else {
        0
    }
}

pub(crate) fn cmd_assumptions(path: &Path, top: usize) -> i32 {
    let loaded = match load(path) {
        Ok(loaded) => loaded,
        Err(error) => return fail(error),
    };
    let program = Program::new(&loaded.decls);
    let (rows, total) = crate::assumptions::report(&program, top);
    if rows.is_empty() {
        println!("no load-bearing unsupported declarations");
        return 0;
    }
    println!("{total} load-bearing assumption(s):");
    for row in &rows {
        println!(
            "  {} {} touches {:>2}  {}",
            pad(&row.name, 18),
            pad(&row.kind, 12),
            row.count,
            row.why
        );
    }
    if total > rows.len() {
        println!("  … {} more; raise --top", total - rows.len());
    }
    0
}

pub(crate) fn cmd_whatif(path: &Path, name: &str) -> i32 {
    let loaded = match load(path) {
        Ok(loaded) => loaded,
        Err(error) => return fail(error),
    };
    let reduced = match crate::whatif::remove(&loaded.decls, name) {
        Ok(reduced) => reduced,
        Err(error) => {
            println!("{error}");
            return 1;
        }
    };
    let result = crate::whatif::compare(&loaded.decls, &reduced);
    println!("without {name}:");
    println!(
        "  errors {} -> {}, warnings {} -> {}",
        result.errors_before, result.errors_after, result.warnings_before, result.warnings_after
    );
    if !result.new_codes.is_empty() {
        println!("  new codes: {}", result.new_codes.join(", "));
    }
    if !result.lost_codes.is_empty() {
        println!("  lost codes: {}", result.lost_codes.join(", "));
    }
    if result.changed.is_empty() {
        println!("  no proposition changed status");
    } else {
        for (name, before, after) in &result.changed {
            println!("  {name}: {before} -> {after}");
        }
    }
    if !result.newly_asserted.is_empty() {
        println!("  newly asserted: {}", result.newly_asserted.join(", "));
    }
    0
}
