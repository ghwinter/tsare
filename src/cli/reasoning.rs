//! Commands that reason over the whole graph: what holds, what is attacked, what to ask next.

use super::*;

pub(crate) fn cmd_engine(path: &Path, json: bool) -> i32 {
    let loaded = match load(path) {
        Ok(loaded) => loaded,
        Err(error) => return fail(error),
    };
    let result = crate::closure::engine(&loaded.decls);
    if json {
        let quoted = |items: &[String]| {
            items
                .iter()
                .map(|item| format!("\"{}\"", escape(item)))
                .collect::<Vec<_>>()
                .join(", ")
        };
        println!("{{");
        println!("  \"held\": [{}],", quoted(&result.held));
        let conditional: Vec<String> = result
            .conditional
            .iter()
            .map(|(name, _)| format!("\"{}\"", escape(name)))
            .collect();
        println!("  \"conditional\": [{}],", conditional.join(", "));
        let blocked: Vec<String> = result
            .blocked
            .iter()
            .map(|(warrant, missing)| {
                format!(
                    "{{\"warrant\": \"{}\", \"missing\": [{}]}}",
                    escape(warrant),
                    quoted(missing)
                )
            })
            .collect();
        println!("  \"blocked\": [{}],", blocked.join(", "));
        let chains: Vec<String> = result
            .chains
            .iter()
            .map(|(name, chain)| format!("\"{}\": \"{}\"", escape(name), escape(chain)))
            .collect();
        println!("  \"chains\": {{{}}},", chains.join(", "));
        let gates: Vec<String> = result
            .gates
            .iter()
            .map(|(name, weak)| format!("\"{}\": [{}]", escape(name), quoted(weak)))
            .collect();
        println!("  \"gates\": {{{}}}", gates.join(", "));
        println!("}}");
        return 0;
    }
    println!(
        "held {}  conditional {}  blocked {}",
        result.held.len(),
        result.conditional.len(),
        result.blocked.len()
    );
    for (name, chain) in &result.chains {
        let gates = result
            .gates
            .iter()
            .find(|(gate, _)| gate == name)
            .map(|(_, weak)| weak.clone())
            .unwrap_or_default();
        let suffix = if gates.is_empty() {
            String::new()
        } else {
            let shown: Vec<String> = gates.iter().take(3).cloned().collect();
            format!("  gates: {}", shown.join(", "))
        };
        println!("  held {chain}{suffix}");
    }
    for (name, reasons) in &result.conditional {
        let shown: Vec<String> = reasons.iter().take(2).cloned().collect();
        println!("  conditional {name}: {}", shown.join("; "));
    }
    for (warrant, missing) in &result.blocked {
        println!("  blocked {warrant} needs {}", missing.join(", "));
    }
    0
}

pub(crate) fn cmd_argue(path: &Path, against: Option<&Path>) -> i32 {
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
    for row in &rows {
        println!(
            "{}  {} {}",
            pad(&row.name, width),
            pad(row.status, 10),
            row.why
        );
    }
    let errors = error_count(&diagnostics);
    if errors > 0 {
        println!(
            "note: {errors} error(s) in {}; conclusions are computed on the resolved graph",
            path.display()
        );
    }
    0
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

pub(crate) fn cmd_derive(path: &Path) -> i32 {
    let loaded = match load(path) {
        Ok(loaded) => loaded,
        Err(error) => return fail(error),
    };
    let program = Program::new(&loaded.decls);
    let analysis = crate::derive::analyze(&program);
    println!(
        "grounded: {} proposition(s) from {} facts",
        analysis.grounded.len(),
        analysis.facts.len()
    );
    for name in &analysis.grounded {
        println!("  derived {name}");
    }
    if analysis.conditional.is_empty() {
        println!("every supported conclusion is grounded in facts");
    } else {
        println!("supported but not grounded: {}", analysis.conditional.len());
        for name in &analysis.conditional {
            println!("  {name}");
            let reasons = analysis.reasons.get(name).cloned().unwrap_or_default();
            for reason in reasons.iter().take(3) {
                println!("    {reason}");
            }
        }
    }
    0
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
    }
    0
}

pub(crate) fn cmd_next(path: &Path) -> i32 {
    let loaded = match load(path) {
        Ok(loaded) => loaded,
        Err(error) => return fail(error),
    };
    let program = Program::new(&loaded.decls);
    let rows = crate::impact::decisions(&program);
    if rows.is_empty() {
        println!("no open decisions");
        return 0;
    }
    let width = rows
        .iter()
        .map(|row| row.label.chars().count())
        .max()
        .unwrap_or(0);
    for row in &rows {
        println!("{}  touches {} claim(s)", pad(&row.label, width), row.count);
    }
    0
}

pub(crate) fn cmd_ask(path: &Path, args: &Args) -> i32 {
    let loaded = match load(path) {
        Ok(loaded) => loaded,
        Err(error) => return fail(error),
    };
    let program = Program::new(&loaded.decls);
    let asked = crate::ask::questions(&program);
    let one = args.has("--one");
    if args.has("--json") {
        let asked = if one {
            &asked[..asked.len().min(1)]
        } else {
            &asked[..]
        };
        let items: Vec<crate::json::Json> = asked
            .iter()
            .map(|item| {
                let mut fields = vec![
                    ("kind", crate::json::Json::Str(item.kind.to_string())),
                    ("subject", crate::json::Json::Str(item.subject.clone())),
                    ("decides", crate::json::Json::Num(item.decides as f64)),
                    ("question", crate::json::Json::Str(item.question.clone())),
                ];
                if let Some(fact) = &item.fact {
                    fields.push(("fact", crate::json::Json::Str(fact.clone())));
                }
                if let Some(key) = &item.key {
                    fields.push(("key", crate::json::Json::Str(key.clone())));
                }
                if let Some(value) = &item.value {
                    fields.push(("value", crate::json::Json::Str(value.clone())));
                }
                crate::json::Json::Obj(
                    fields
                        .into_iter()
                        .map(|(k, v)| (k.to_string(), v))
                        .collect(),
                )
            })
            .collect();
        println!(
            "{}",
            crate::json::serialize(&crate::json::Json::Obj(vec![(
                "questions".to_string(),
                crate::json::Json::Arr(items)
            )]))
        );
        return 0;
    }
    print!("{}", crate::ask::text(&program, one));
    0
}

pub(crate) fn cmd_novel(path: &Path, args: &Args) -> i32 {
    let Some(attribute) = args.option("--attribute") else {
        println!("novel needs --attribute NAME; add --holds F_A,F_B for the facts it is true of");
        return 2;
    };
    let loaded = match load(path) {
        Ok(loaded) => loaded,
        Err(error) => return fail(error),
    };
    let program = Program::new(&loaded.decls);
    let holds: Vec<String> = args
        .option("--holds")
        .map(|list| {
            list.split(',')
                .map(|id| id.trim().to_string())
                .filter(|id| !id.is_empty())
                .collect()
        })
        .unwrap_or_default();
    for id in &holds {
        if !program.has(id) {
            println!("no such declaration: {id}");
            return 1;
        }
    }
    let verdict = crate::novel::decide(&program, &holds);
    print!("{}", crate::novel::text(&verdict, attribute));
    0
}

pub(crate) fn cmd_abstract(path: &Path, args: &Args) -> i32 {
    let loaded = match load(path) {
        Ok(loaded) => loaded,
        Err(error) => return fail(error),
    };
    let program = Program::new(&loaded.decls);
    let support = args.count("--min-support", 2);
    let max_intent = args.count("--max-intent", 3);
    let (found, truncated) = crate::abstraction::candidates(&program, support, max_intent);
    print!("{}", crate::abstraction::text(&found, truncated));
    0
}

/// The forms a document's declarations share, and the positions that vary inside each form.
/// A form with one instance is not a structure, so only shared ones are printed.
pub(crate) fn cmd_shape(path: &Path, top: usize, depth: usize, layers: bool) -> i32 {
    let loaded = match load(path) {
        Ok(loaded) => loaded,
        Err(error) => return fail(error),
    };
    let program = Program::new(&loaded.decls);
    if layers {
        let fibres = crate::shape::fibres(&program, depth);
        print!("{}", crate::shape::fibre_text(&fibres, depth));
        return 0;
    }
    let groups = crate::shape::shapes_at(&program, depth);
    print!(
        "{}",
        crate::shape::text(&groups, loaded.decls.len(), top, depth)
    );
    0
}
pub(crate) fn cmd_imply(path: &Path, args: &Args) -> i32 {
    let loaded = match load(path) {
        Ok(loaded) => loaded,
        Err(error) => return fail(error),
    };
    let program = Program::new(&loaded.decls);
    let max = args.count("--max-antecedent", 2);
    let support = args.count("--min-support", 3);
    let (found, search) = crate::imply::implications(&program, max, support);
    if args.has("--json") {
        let items: Vec<crate::json::Json> = found
            .iter()
            .map(|item| {
                let mut fields = vec![
                    (
                        "antecedent".to_string(),
                        crate::json::Json::Arr(
                            item.antecedent
                                .iter()
                                .map(|pair| crate::json::Json::Str(pair.clone()))
                                .collect(),
                        ),
                    ),
                    (
                        "consequent".to_string(),
                        crate::json::Json::Str(item.consequent.clone()),
                    ),
                    (
                        "support".to_string(),
                        crate::json::Json::Num(item.support as f64),
                    ),
                    (
                        "objects".to_string(),
                        crate::json::Json::Num(item.objects as f64),
                    ),
                ];
                if let Some(witness) = &item.converse_witness {
                    fields.push((
                        "converse_fails".to_string(),
                        crate::json::Json::Str(witness.clone()),
                    ));
                }
                crate::json::Json::Obj(fields)
            })
            .collect();
        println!(
            "{}",
            crate::json::serialize(&crate::json::Json::Obj(vec![(
                "implications".to_string(),
                crate::json::Json::Arr(items)
            )]))
        );
        return 0;
    }
    print!("{}", crate::imply::text(&found, &search));
    0
}

pub(crate) fn cmd_cost(path: &Path, top: usize) -> i32 {
    let loaded = match load(path) {
        Ok(loaded) => loaded,
        Err(error) => return fail(error),
    };
    let program = Program::new(&loaded.decls);
    let cost = crate::cost::report(&program);
    println!(
        "decompression cost: {} external confirmation(s), depth {}",
        cost.total, cost.depth
    );
    if !cost.by_kind.is_empty() {
        let joined: Vec<String> = cost
            .by_kind
            .iter()
            .map(|(kind, count)| format!("{kind}={count}"))
            .collect();
        println!("by kind: {}", joined.join(", "));
    }
    for (name, found) in cost.rows.iter().take(top) {
        println!("  {} {}  {}", pad(name, 18), found.len(), found.join("; "));
    }
    if cost.depth > 0 {
        println!("chain: {}", cost.chain.join(" -> "));
    }
    0
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
    0
}

pub(crate) fn cmd_cat(path: &Path) -> i32 {
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
    let (kept, total, failures) = crate::cat::extent_functor(&program);
    println!("Ext:     {kept}/{total} parent arrows preserved");
    for failure in &failures {
        println!("  FAIL {failure}");
    }
    let (visible, crossing, leaks) = crate::cat::expose_functor(&program);
    println!("Expose:  {visible}/{crossing} cross-layer arrows visible");
    for leak in &leaks {
        println!("  FAIL {leak}");
    }
    println!("checker: {} error(s)", error_count(&diagnostics));
    println!("stages:");
    for stage in crate::cat::stages() {
        println!(
            "  {} {} {}",
            pad(stage.name, 24),
            pad(stage.label, 16),
            stage.why
        );
    }
    0
}

pub(crate) fn cmd_moves(path: &Path) -> i32 {
    let loaded = match load(path) {
        Ok(loaded) => loaded,
        Err(error) => return fail(error),
    };
    let program = Program::new(&loaded.decls);
    let rows = crate::moves::suggest(&program);
    if rows.is_empty() {
        println!("no representation move suggested");
        return 0;
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
    0
}

pub(crate) fn cmd_assumptions(path: &Path, top: usize) -> i32 {
    let loaded = match load(path) {
        Ok(loaded) => loaded,
        Err(error) => return fail(error),
    };
    let program = Program::new(&loaded.decls);
    let rows = crate::assumptions::report(&program, top);
    if rows.is_empty() {
        println!("no load-bearing unsupported declarations");
        return 0;
    }
    println!("{} load-bearing assumption(s):", rows.len());
    for row in &rows {
        println!(
            "  {} {} touches {:>2}  {}",
            pad(&row.name, 18),
            pad(&row.kind, 12),
            row.count,
            row.why
        );
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

pub(crate) fn cmd_fca(path: &Path) -> i32 {
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
    0
}

pub(crate) fn cmd_kinds() -> i32 {
    println!("{} {} discharge", pad("kind", 12), pad("obligation", 52));
    for (kind, obligation, discharge) in program::TABLE {
        println!("{} {} {}", pad(kind, 12), pad(obligation, 52), discharge);
    }
    0
}
