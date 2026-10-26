//! The command surface: one function per subcommand, text first and JSON on `--json`.
//!
//! Output is the reference implementation's, because a port that prints something else is
//! not a port anyone can compare.  Where the engine has a machine contract (`check`,
//! `engine`, `suggest`) the same command also answers `--json`.

use std::fs;
use std::path::{Path, PathBuf};

use crate::program::{self, Program};
use crate::syntax::{self, Decl, Val};

pub struct Loaded {
    pub decls: Vec<Decl>,
    pub text: String,
    pub path: PathBuf,
}

pub fn load(path: &Path) -> Result<Loaded, String> {
    if path.is_dir() {
        let mut files: Vec<PathBuf> = fs::read_dir(path)
            .map_err(|error| format!("{}: {error}", path.display()))?
            .filter_map(|entry| entry.ok().map(|item| item.path()))
            .filter(|item| item.extension().map(|ext| ext == "mf").unwrap_or(false))
            .collect();
        files.sort();
        let mut decls = Vec::new();
        for file in &files {
            let source = fs::read_to_string(file).map_err(|error| format!("{}: {error}", file.display()))?;
            let mut parsed = syntax::parse(&source)
                .map_err(|error| format!("{}: {error}", file.display()))?;
            let name = file.file_name().unwrap_or_default().to_string_lossy().to_string();
            for decl in &mut parsed {
                decl.file = name.clone();
            }
            decls.extend(parsed);
        }
        return Ok(Loaded { decls, text: String::new(), path: path.to_path_buf() });
    }
    let text = fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let decls = syntax::parse(&text).map_err(|error| format!("{}: {error}", path.display()))?;
    Ok(Loaded { decls, text, path: path.to_path_buf() })
}

fn fail(error: String) -> i32 {
    println!("{error}");
    1
}

fn escape(text: &str) -> String {
    let mut out = String::new();
    for ch in text.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            other => out.push(other),
        }
    }
    out
}

fn pad(text: &str, width: usize) -> String {
    let mut out = text.to_string();
    while out.chars().count() < width {
        out.push(' ');
    }
    out
}

fn error_count(diagnostics: &[crate::check::Diagnostic]) -> usize {
    diagnostics.iter().filter(|item| item.code.starts_with("E-")).count()
}

fn warning_count(diagnostics: &[crate::check::Diagnostic]) -> usize {
    diagnostics.len() - error_count(diagnostics)
}

/// Diagnostics as JSON records, the contract in PROTOCOL.md.
fn diagnostics_json(diagnostics: &[crate::check::Diagnostic], fallback: &str) -> String {
    let mut sorted = diagnostics.to_vec();
    sorted.sort_by_key(|item| (item.file.clone(), item.line, item.col, item.code));
    let mut lines: Vec<String> = Vec::new();
    for item in &sorted {
        let file = if item.file.is_empty() { fallback.to_string() } else { item.file.clone() };
        let severity = if item.code.starts_with("E-") { "error" } else { "warning" };
        lines.push(format!(
            "    {{\"code\": \"{}\", \"severity\": \"{}\", \"file\": \"{}\", \"line\": {}, \"col\": {}, \"message\": \"{}\"}}",
            item.code,
            severity,
            escape(&file),
            item.line,
            item.col,
            escape(&item.message)
        ));
    }
    let joined = if lines.is_empty() { String::new() } else { format!("\n{}\n", lines.join(",\n")) };
    format!(
        "{{\n  \"diagnostics\": [{joined}],\n  \"errors\": {},\n  \"warnings\": {}\n}}",
        error_count(diagnostics),
        warning_count(diagnostics)
    )
}

fn report(diagnostics: &[crate::check::Diagnostic]) -> i32 {
    println!("{} error(s), {} warning(s)", error_count(diagnostics), warning_count(diagnostics));
    if error_count(diagnostics) > 0 {
        1
    } else {
        0
    }
}

fn text_diagnostics(diagnostics: &[crate::check::Diagnostic], fallback: &str) -> i32 {
    let mut sorted = diagnostics.to_vec();
    sorted.sort_by_key(|item| (item.file.clone(), item.line, item.col, item.code));
    for item in &sorted {
        let file = if item.file.is_empty() { fallback.to_string() } else { item.file.clone() };
        println!("  {} {}:{} {}", item.code, file, item.line, item.message);
    }
    report(diagnostics)
}

fn collect_holes(decls: &[Decl]) -> Vec<(String, String)> {
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

fn cmd_check(
    path: &Path,
    json: bool,
    sugar: bool,
    goals_path: Option<&str>,
    use_cache: bool,
) -> i32 {
    let loaded = if sugar {
        let raw = match fs::read_to_string(path) {
            Ok(raw) => raw,
            Err(error) => return fail(format!("{}: {error}", path.display())),
        };
        let expanded = crate::sugar::expand(&raw);
        match syntax::parse(&expanded) {
            Ok(decls) => Loaded { decls, text: expanded, path: path.to_path_buf() },
            Err(error) => return fail(format!("{}: {error}", path.display())),
        }
    } else {
        match load(path) {
            Ok(loaded) => loaded,
            Err(error) => return fail(error),
        }
    };
    let cacheable = use_cache && !sugar && !path.is_dir();
    let cache_key = if cacheable { Some(crate::cache::key(&loaded.text)) } else { None };
    let mut diagnostics = match cache_key.as_ref().and_then(|key| crate::cache::load().get(key).cloned()) {
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
    if let Some(goals_file) = goals_path {
        let text = match fs::read_to_string(goals_file) {
            Ok(text) => text,
            Err(error) => return fail(format!("{goals_file}: {error}")),
        };
        let goals = crate::coverage::load_goals(&text);
        let report = crate::coverage::coverage(&Program::new(&loaded.decls), &goals);
        diagnostics.extend(crate::coverage::diagnostics(&report));
    }
    if json {
        println!("{}", diagnostics_json(&diagnostics, &path.display().to_string()));
        return if error_count(&diagnostics) > 0 { 1 } else { 0 };
    }
    println!("declarations: {}", loaded.decls.len());
    println!("diagnostics: {}", diagnostics.len());
    let mut sorted = diagnostics.clone();
    sorted.sort_by_key(|item| (item.file.clone(), item.line, item.col, item.code));
    for item in &sorted {
        let file = if item.file.is_empty() {
            path.display().to_string()
        } else {
            item.file.clone()
        };
        println!("  {} {}:{} {}", item.code, file, item.line, item.message);
    }
    if let Some(goals_file) = goals_path {
        if let Ok(text) = fs::read_to_string(goals_file) {
            let goals = crate::coverage::load_goals(&text);
            let report = crate::coverage::coverage(&Program::new(&loaded.decls), &goals);
            let covered = report.iter().filter(|goal| goal.covered()).count();
            println!("coverage: {covered}/{} goals addressed", report.len());
        }
    }
    let holes = collect_holes(&loaded.decls);
    if !holes.is_empty() {
        println!("holes: {} unknown value(s), listed in OPEN.md after a build", holes.len());
    }
    println!("{} error(s), {} warning(s)", error_count(&diagnostics), warning_count(&diagnostics));
    if error_count(&diagnostics) > 0 {
        1
    } else {
        0
    }
}

fn cmd_engine(path: &Path, json: bool) -> i32 {
    let loaded = match load(path) {
        Ok(loaded) => loaded,
        Err(error) => return fail(error),
    };
    let result = crate::closure::engine(&loaded.decls);
    if json {
        let quoted = |items: &[String]| {
            items.iter().map(|item| format!("\"{}\"", escape(item))).collect::<Vec<_>>().join(", ")
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
                format!("{{\"warrant\": \"{}\", \"missing\": [{}]}}", escape(warrant), quoted(missing))
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

fn cmd_suggest(path: &Path, json: bool) -> i32 {
    let loaded = match load(path) {
        Ok(loaded) => loaded,
        Err(error) => return fail(error),
    };
    let items = crate::fix::suggestions(&loaded.decls);
    if !json {
        if items.is_empty() {
            println!("no verified suggestion");
            return 0;
        }
        for item in &items {
            println!("{} {}", pad(&item.applicability, 18), item.message);
        }
        return 0;
    }
    let rendered: Vec<String> = items
        .iter()
        .map(|item| {
            format!(
                "{{\"applicability\": \"{}\", \"message\": \"{}\", \"file\": \"{}\", \"span\": {{\"line\": {}, \"col\": {}, \"end_line\": {}, \"end_col\": {}}}, \"replacement\": \"{}\"}}",
                item.applicability,
                escape(&item.message),
                escape(&item.file),
                item.line,
                item.col,
                item.end_line,
                item.end_col,
                escape(&item.replacement)
            )
        })
        .collect();
    println!("{{\"suggestions\": [{}]}}", rendered.join(", "));
    0
}

fn cmd_apply(path: &Path, dry: bool) -> i32 {
    let loaded = match load(path) {
        Ok(loaded) => loaded,
        Err(error) => return fail(error),
    };
    let items = crate::fix::suggestions(&loaded.decls);
    let (updated, applied) = crate::apply::apply_best(&loaded.text, &items);
    if !dry {
        fs::write(path, &updated).expect("write file");
    }
    let list: Vec<String> = applied.iter().map(|item| format!("\"{}\"", escape(item))).collect();
    println!("{{\"applied\": [{}]}}", list.join(", "));
    0
}

fn cmd_argue(path: &Path) -> i32 {
    let loaded = match load(path) {
        Ok(loaded) => loaded,
        Err(error) => return fail(error),
    };
    let program = Program::new(&loaded.decls);
    let diagnostics = crate::check::check(&loaded.decls);
    let rows = crate::argue::conclusions(&program);
    let width = rows.iter().map(|row| row.name.chars().count()).max().unwrap_or(10);
    for row in &rows {
        println!("{}  {} {}", pad(&row.name, width), pad(row.status, 10), row.why);
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

fn cmd_derive(path: &Path) -> i32 {
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

fn cmd_debt(path: &Path, all: bool) -> i32 {
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

fn cmd_impact(path: &Path) -> i32 {
    let loaded = match load(path) {
        Ok(loaded) => loaded,
        Err(error) => return fail(error),
    };
    let program = Program::new(&loaded.decls);
    let diagnostics = crate::check::check(&loaded.decls);
    for line in crate::impact::report(&program) {
        println!("{line}");
    }
    let errors = error_count(&diagnostics);
    if errors > 0 {
        println!("note: {errors} error(s); analyses ran on the resolved part");
    }
    0
}

fn cmd_next(path: &Path) -> i32 {
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
    let width = rows.iter().map(|row| row.label.chars().count()).max().unwrap_or(0);
    for row in &rows {
        println!("{}  touches {} claim(s)", pad(&row.label, width), row.count);
    }
    0
}

fn cmd_cost(path: &Path, top: usize) -> i32 {
    let loaded = match load(path) {
        Ok(loaded) => loaded,
        Err(error) => return fail(error),
    };
    let program = Program::new(&loaded.decls);
    let cost = crate::cost::report(&program);
    println!("decompression cost: {} external confirmation(s), depth {}", cost.total, cost.depth);
    if !cost.by_kind.is_empty() {
        let joined: Vec<String> =
            cost.by_kind.iter().map(|(kind, count)| format!("{kind}={count}")).collect();
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

fn cmd_weigh(path: &Path, threshold: f64) -> i32 {
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
            println!("  {} {}  {}", pad(&row.name, 16), format!("{:>5}", "--"), row.status);
            continue;
        };
        let mark = if value < threshold { "below" } else { "ok" };
        let detail: Vec<String> =
            row.detail.iter().map(|(rule, score)| format!("{rule}={score:.2}")).collect();
        println!(
            "  {} {}  {} {} {}",
            pad(&row.name, 16),
            format!("{value:>5.2}"),
            pad(row.status, 8),
            pad(mark, 5),
            detail.join("  ")
        );
    }
    0
}

fn cmd_cat(path: &Path) -> i32 {
    let loaded = match load(path) {
        Ok(loaded) => loaded,
        Err(error) => return fail(error),
    };
    let program = Program::new(&loaded.decls);
    let diagnostics = crate::check::check(&loaded.decls);
    let objects: Vec<String> =
        crate::cat::objects(&program).iter().map(|(kind, count)| format!("{kind}={count}")).collect();
    let arrows: Vec<String> =
        crate::cat::arrows(&program).iter().map(|(kind, count)| format!("{kind}={count}")).collect();
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
        println!("  {} {} {}", pad(stage.name, 24), pad(stage.label, 16), stage.why);
    }
    0
}

fn cmd_moves(path: &Path) -> i32 {
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
        println!("{} {} {} {}", row.code, pad(label, 26), pad(&row.target, 18), row.why);
    }
    0
}

fn cmd_assumptions(path: &Path, top: usize) -> i32 {
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
            "  {} {} touches {}  {}",
            pad(&row.name, 18),
            pad(&row.kind, 12),
            format!("{:>2}", row.count),
            row.why
        );
    }
    0
}

fn cmd_audit(path: &Path) -> i32 {
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
        let lost_text = if row.lost.is_empty() { "-".to_string() } else { row.lost.join(", ") };
        let added_text = if row.added.is_empty() { "-".to_string() } else { row.added.join(", ") };
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

fn cmd_egraph(path: &Path, write: Option<&str>) -> i32 {
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

fn cmd_whatif(path: &Path, name: &str) -> i32 {
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

fn cmd_graph(path: &Path, out: Option<&str>) -> i32 {
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

fn cmd_map(left: &Path, right: &Path, top: usize) -> i32 {
    let left_doc = match load(left) {
        Ok(loaded) => loaded,
        Err(error) => return fail(error),
    };
    let right_doc = match load(right) {
        Ok(loaded) => loaded,
        Err(error) => return fail(error),
    };
    let rows = crate::mapping::align(
        &Program::new(&left_doc.decls),
        &Program::new(&right_doc.decls),
        top,
    );
    if rows.is_empty() {
        println!("no shared structure");
        return 0;
    }
    for row in &rows {
        let shown: Vec<String> = row.shared.iter().take(4).map(|path| path.join(">")).collect();
        println!(
            "{:.2} w={} {} ~ {}  shared: {}",
            row.score,
            pad(&row.weight.to_string(), 2),
            pad(&row.name_a, 18),
            pad(&row.name_b, 18),
            shown.join(", ")
        );
    }
    0
}

/// Codes whose repair needs a value only the author can choose.  Naming them is the honest
/// alternative to printing nothing and letting the reader assume no repair was needed.
const NO_REPAIR: &[(&str, &str)] = &[
    ("W-EXAMPLE-SHAPE", "needs an attribute the examples share; only the author knows one"),
    ("E-CYCLE", "a cycle can be cut at any edge; which edge is a judgement"),
    ("W-UNCONFIRMED", "needs a person or a tool to confirm the evidence"),
    ("E-UNFALSIFIABLE", "needs a test or a rival, which is content"),
    ("E-VACUOUS", "needs premises or concepts, which is content"),
    ("W-PARTITION-UNKNOWN", "needs a dimension, which is the thing being asked for"),
    ("W-PARTITION-UNLEARNED", "needs an attribute that separates the examples"),
];

fn cmd_fix(path: &Path, apply: Option<&str>) -> i32 {
    let loaded = match load(path) {
        Ok(loaded) => loaded,
        Err(error) => return fail(error),
    };
    let (accepted, (before_e, before_w)) = crate::fix::verified(&loaded.decls);
    println!("before: {before_e} errors, {before_w} warnings (holes not counted)");
    if accepted.is_empty() {
        println!("no verified candidate reduces this");
    }
    for item in &accepted {
        println!(
            "  + {}  ({:+} errors, {:+} warnings)",
            item.description, item.delta.0, item.delta.1
        );
    }
    let diagnostics = crate::check::check(&loaded.decls);
    let accepted_names: std::collections::HashSet<&str> =
        accepted.iter().map(|item| item.description.as_str()).collect();
    let mut flat = 0;
    for (description, trial) in crate::fix::candidates(&loaded.decls) {
        if accepted_names.contains(description.as_str()) {
            continue;
        }
        if flat == 4 {
            break;
        }
        if flat == 0 {
            println!("candidates that do not pay:");
        }
        let (after_e, after_w) = crate::fix::score_without_holes(&trial);
        println!(
            "  - {description}  (no gain: {:+} errors, {:+} warnings)",
            after_e as isize - before_e as isize,
            after_w as isize - before_w as isize
        );
        flat += 1;
    }
    let blocked: Vec<&(&str, &str)> = NO_REPAIR
        .iter()
        .filter(|(code, _)| diagnostics.iter().any(|item| item.code == *code))
        .collect();
    if !blocked.is_empty() {
        println!("no machine repair for:");
        for (code, why) in blocked {
            println!("  {code}  {why}");
        }
    }
    if let Some(out) = apply {
        let mut current = loaded.decls.clone();
        let mut rounds = 0;
        while rounds < 6 {
            let (batch, _) = crate::fix::verified(&current);
            if batch.is_empty() {
                break;
            }
            current = batch.into_iter().next().map(|item| item.trial).unwrap_or(current);
            rounds += 1;
        }
        let (after_e, after_w) = crate::fix::score_without_holes(&current);
        fs::write(out, crate::fix::render(&current)).expect("write file");
        println!("applied {rounds} fix(es) -> {out} ({after_e} errors, {after_w} warnings)");
    }
    0
}

fn cmd_induce(path: &Path) -> i32 {
    let loaded = match load(path) {
        Ok(loaded) => loaded,
        Err(error) => return fail(error),
    };
    let program = Program::new(&loaded.decls);
    let diagnostics = crate::check::check(&loaded.decls);
    let errors = error_count(&diagnostics);
    if errors > 0 {
        println!("note: {errors} error(s) in {}; learning ran on what resolved", path.display());
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

fn cmd_pattern(path: &Path, name: &str, with_vars: bool) -> i32 {
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
    println!("  positives: {}", if positives.is_empty() { "-".to_string() } else { positives.join(", ") });
    println!("  negatives: {}", if negatives.is_empty() { "-".to_string() } else { negatives.join(", ") });
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
            let expression: Vec<String> =
                rule.iter().map(|feature| format!("\"{feature}\" in features")).collect();
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
        let side = if positives.contains(name) { "pos" } else { "neg" };
        let mut found: Vec<String> =
            crate::paths::features(&program, name, 2, &exclude).into_iter().collect();
        found.sort();
        println!("    {side} {name}: {}", if found.is_empty() { "-".to_string() } else { found.join(", ") });
    }
    0
}

fn cmd_build(path: &Path, out: &str, goals_path: Option<&str>) -> i32 {
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
        fs::write(out_dir.join("OBLIGATIONS.md"), crate::build::render_obligations(&program, &learning_map(&learnings))).expect("write");
        fs::write(out_dir.join("LEARNED.md"), crate::build::render_learned(&learnings, &[])).expect("write");
        println!("project {}: {} declarations, emitted {}", path.display(), program.decls.len(), out_dir.display());
        return 0;
    }

    let diagnostics = crate::check::check(&loaded.decls);
    text_diagnostics(&diagnostics, &path.display().to_string());
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
        println!("holes: {} unknown value(s), listed in OPEN.md after a build", holes.len());
    }
    if error_count(&all) > 0 {
        println!("not emitted: fix the errors first");
        return 1;
    }

    let learnings = crate::build::learnings_of(&program);
    let mut structurals: Vec<(String, Option<crate::patterns::Pattern>)> = Vec::new();
    for concept in program.of_kind("concept") {
        let rule = crate::patterns::learn(
            &program,
            &program::pos(concept),
            &program::neg(concept),
        );
        structurals.push((concept.name.clone(), rule));
    }
    let mapping = learning_map(&learnings);
    fs::write(out_dir.join("OBLIGATIONS.md"), crate::build::render_obligations(&program, &mapping)).expect("write");
    fs::write(out_dir.join("LEARNED.md"), crate::build::render_learned(&learnings, &structurals)).expect("write");
    fs::write(out_dir.join("OPEN.md"), crate::build::render_open(&program, &learnings, &holes)).expect("write");
    fs::write(out_dir.join("NOTES.md"), crate::build::render_notes(&program)).expect("write");
    let source = crate::build::predicates_source(&program, &mapping, &structurals, &path.file_name().map(|name| name.to_string_lossy().to_string()).unwrap_or_default());
    fs::write(out_dir.join("predicates.py"), &source).expect("write");
    for name in ["OBLIGATIONS.md", "LEARNED.md", "OPEN.md", "NOTES.md", "predicates.py"] {
        println!("emitted {}", out_dir.join(name).display());
    }
    let defined = learnings.iter().filter(|item| item.clause.is_some()).count();
    let structured = structurals.iter().filter(|(_, rule)| rule.is_some()).count();
    let problems = crate::build::verify_predicates(&program, &mapping, &structurals);
    if !problems.is_empty() {
        println!("predicates: FAILED on {} examples", problems.len());
        for problem in &problems {
            println!("  {problem}");
        }
        return 1;
    }
    println!("predicates: {defined} attribute + {structured} structural, verified on every example");
    if let Some(report) = &goals {
        fs::write(out_dir.join("COVERAGE.md"), crate::build::render_coverage(report)).expect("write");
        println!("emitted {}", out_dir.join("COVERAGE.md").display());
    }
    0
}

fn learning_map(learnings: &[crate::induce::Learning]) -> std::collections::HashMap<String, crate::induce::Learning> {
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

fn cmd_project(path: &Path) -> i32 {
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
    println!("{} error(s), {} warning(s)", error_count(&diagnostics), warning_count(&diagnostics));
    if error_count(&diagnostics) > 0 {
        1
    } else {
        0
    }
}

fn cmd_mem(recall: Option<&[String]>, k: usize) -> i32 {
    if let Some(codes) = recall {
        let entries = crate::cases::recall(codes, k);
        if entries.is_empty() {
            println!("no similar case");
            return 0;
        }
        for entry in entries {
            let outcome = if entry.improved { "improved" } else { "no better" };
            let codes = entry.entry.get("codes").map(|value| value.strings()).unwrap_or_default();
            let action = entry.entry.get("action").and_then(|value| value.as_str()).unwrap_or("");
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
    println!("expired:     {}  (recorded under a different checker)", report.expired);
    println!(
        "conflicting: {}  (same situation and action, opposite outcome)",
        report.conflicting
    );
    println!("no-op:       {}  (changed nothing either way)", report.noop);
    println!("unchanged:   {}", report.unchanged);
    println!("orphans:     {}  (document no longer exists)", report.orphans);
    0
}

fn cmd_propose(path: &Path, proposals_path: &str, apply: bool) -> i32 {
    let loaded = match load(path) {
        Ok(loaded) => loaded,
        Err(error) => return fail(error),
    };
    let text = match fs::read_to_string(proposals_path) {
        Ok(text) => text,
        Err(error) => return fail(format!("{proposals_path}: {error}")),
    };
    let payload = match crate::json::parse(&text) {
        Ok(payload) => payload,
        Err(error) => return fail(format!("{proposals_path}: {error}")),
    };
    let items: Vec<crate::json::Json> = match &payload {
        crate::json::Json::Arr(items) => items.clone(),
        other => other
            .get("proposals")
            .and_then(|value| value.as_arr())
            .map(|items| items.to_vec())
            .unwrap_or_default(),
    };
    if items.is_empty() {
        println!("no proposals in {proposals_path}");
        return 2;
    }

    let mut current = loaded.text.clone();
    let mut admitted = 0;
    let mut needs_review = 0;
    for item in &items {
        let decl = item.get("decl").and_then(|value| value.as_str()).unwrap_or("");
        let field = item.get("field").and_then(|value| value.as_str()).unwrap_or("");
        let value = item.get("value").and_then(|value| value.as_str()).unwrap_or("");
        let proposal = crate::propose::Proposal {
            decl: decl.to_string(),
            field: field.to_string(),
            value: value.to_string(),
        };
        let decls = match syntax::parse(&current) {
            Ok(decls) => decls,
            Err(error) => return fail(format!("the document stopped parsing: {error}")),
        };
        match crate::propose::verify(&current, &decls, &proposal) {
            crate::propose::Verdict::Admitted { errors, warnings, text } => {
                admitted += 1;
                println!(
                    "{} admitted  ({errors:+} errors, {warnings:+} warnings)",
                    pad(&format!("{decl}.{field}"), 34)
                );
                current = text;
            }
            crate::propose::Verdict::NeedsReview { codes, text } => {
                needs_review += 1;
                println!(
                    "{} review    introduced {}",
                    pad(&format!("{decl}.{field}"), 34),
                    codes.join(", ")
                );
                current = text;
            }
            crate::propose::Verdict::Rejected { reason } => {
                println!("{} rejected  {reason}", pad(&format!("{decl}.{field}"), 34));
            }
        }
    }
    if apply {
        fs::write(path, &current).expect("write file");
    }
    println!("{admitted} admitted, {needs_review} review, {} rejected", items.len() - admitted - needs_review);
    0
}

fn cmd_fca(path: &Path) -> i32 {
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
        println!("  intent        {}", crate::fca::render_pairs(&concept.intent));
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

fn cmd_kinds() -> i32 {
    println!("{} {} discharge", pad("kind", 12), pad("obligation", 52));
    for (kind, obligation, discharge) in program::TABLE {
        println!("{} {} {}", pad(kind, 12), pad(obligation, 52), discharge);
    }
    0
}

fn usage() {
    println!("engine-core — a checker for structured arguments");
    println!();
    println!("usage:");
    println!("  engine-core FILE|DIR [--json]        check, text or JSON");
    println!("  engine-core check FILE|DIR           same as above");
    println!("  engine-core engine FILE|DIR [--json] held, conditional, blocked, chains, gates");
    println!("  engine-core argue FILE               which conclusions survive attack");
    println!("  engine-core derive FILE              which conclusions are grounded in facts");
    println!("  engine-core debt FILE [--all]        which facts transitively gate a conclusion");
    println!("  engine-core kinds                    the obligation table");
    println!("  engine-core suggest FILE [--json]    verified repairs with an applicability");
    println!("  engine-core apply FILE [--dry-run]   apply machine repairs and recompile");
}

fn is_command(name: &str) -> bool {
    matches!(
        name,
        "check"
            | "engine"
            | "argue"
            | "derive"
            | "debt"
            | "kinds"
            | "suggest"
            | "apply"
            | "impact"
            | "next"
            | "cost"
            | "weigh"
            | "cat"
            | "moves"
            | "assumptions"
            | "audit"
            | "egraph"
            | "fix"
            | "induce"
            | "graph"
            | "map"
            | "mem"
            | "pattern"
            | "project"
            | "build"
            | "whatif"
            | "propose"
            | "fca"
    )
}

/// The arguments after the command, split into positional values and options.
struct Args {
    positional: Vec<String>,
    flags: Vec<String>,
    options: std::collections::HashMap<String, String>,
}

const VALUE_FLAGS: &[&str] = &["--top", "--threshold", "--out", "--goals", "--k", "--apply", "--proposals"];

impl Args {
    fn parse(raw: &[String]) -> Self {
        let mut positional = Vec::new();
        let mut flags = Vec::new();
        let mut options = std::collections::HashMap::new();
        let mut index = 0;
        while index < raw.len() {
            let item = &raw[index];
            if item.starts_with("--") {
                if let Some((name, value)) = item.split_once('=') {
                    options.insert(name.to_string(), value.to_string());
                    index += 1;
                    continue;
                }
                if VALUE_FLAGS.contains(&item.as_str()) && index + 1 < raw.len() {
                    options.insert(item.clone(), raw[index + 1].clone());
                    index += 2;
                    continue;
                }
                flags.push(item.clone());
                index += 1;
                continue;
            }
            positional.push(item.clone());
            index += 1;
        }
        Args { positional, flags, options }
    }

    fn has(&self, flag: &str) -> bool {
        self.flags.iter().any(|item| item == flag)
    }

    fn option(&self, flag: &str) -> Option<&str> {
        self.options.get(flag).map(|value| value.as_str())
    }

    fn number(&self, flag: &str, fallback: f64) -> f64 {
        self.option(flag).and_then(|value| value.parse().ok()).unwrap_or(fallback)
    }

    fn count(&self, flag: &str, fallback: usize) -> usize {
        self.option(flag).and_then(|value| value.parse().ok()).unwrap_or(fallback)
    }

    fn at(&self, index: usize) -> Option<&str> {
        self.positional.get(index).map(|item| item.as_str())
    }
}

pub fn run() -> i32 {
    let raw: Vec<String> = std::env::args().skip(1).collect();
    if raw.is_empty() || raw.iter().any(|arg| arg == "--help" || arg == "-h") {
        usage();
        return 0;
    }
    let (command, rest) = if is_command(&raw[0]) {
        (raw[0].clone(), raw[1..].to_vec())
    } else {
        ("check".to_string(), raw.clone())
    };
    let args = Args::parse(&rest);
    if command == "kinds" {
        return cmd_kinds();
    }
    if command == "mem" {
        let codes: Vec<String> = args.positional.clone();
        let recall = if args.has("--recall") { Some(codes.as_slice()) } else { None };
        return cmd_mem(recall, args.count("--k", 3));
    }
    let Some(target) = args.at(0) else {
        println!("usage: engine-core {command} FILE");
        return 2;
    };
    let path = PathBuf::from(target);
    match command.as_str() {
        "check" => cmd_check(
            &path,
            args.has("--json"),
            args.has("--sugar"),
            args.option("--goals"),
            args.has("--cache"),
        ),
        "engine" => cmd_engine(&path, args.has("--json")),
        "suggest" => cmd_suggest(&path, args.has("--json")),
        "apply" => cmd_apply(&path, args.has("--dry-run")),
        "argue" => cmd_argue(&path),
        "derive" => cmd_derive(&path),
        "debt" => cmd_debt(&path, args.has("--all")),
        "impact" => cmd_impact(&path),
        "next" => cmd_next(&path),
        "cost" => cmd_cost(&path, args.count("--top", 8)),
        "weigh" => cmd_weigh(&path, args.number("--threshold", 0.7)),
        "cat" => cmd_cat(&path),
        "moves" => cmd_moves(&path),
        "assumptions" => cmd_assumptions(&path, args.count("--top", 12)),
        "audit" => cmd_audit(&path),
        "egraph" => cmd_egraph(&path, args.option("--write")),
        "whatif" => {
            let Some(name) = args.at(1) else {
                println!("usage: engine-core whatif FILE NAME");
                return 2;
            };
            cmd_whatif(&path, name)
        }
        "graph" => cmd_graph(&path, args.option("--out")),
        "fix" => cmd_fix(&path, args.option("--apply")),
        "map" => {
            let Some(right) = args.at(1) else {
                println!("usage: engine-core map LEFT RIGHT");
                return 2;
            };
            cmd_map(&path, Path::new(right), args.count("--top", 8))
        }
        "induce" => cmd_induce(&path),
        "propose" => {
            let Some(proposals) = args.option("--proposals") else {
                println!("usage: engine-core propose FILE --proposals FILE.json [--apply]");
                return 2;
            };
            cmd_propose(&path, proposals, args.has("--apply"))
        }
        "fca" => cmd_fca(&path),        "pattern" => {
            let Some(name) = args.at(1) else {
                println!("usage: engine-core pattern FILE CONCEPT");
                return 2;
            };
            cmd_pattern(&path, name, args.has("--vars"))
        }
        "build" => cmd_build(&path, args.option("--out").unwrap_or("out"), args.option("--goals")),
        "project" => cmd_project(&path),
        _ => {
            println!("unknown command {command}");
            2
        }
    }
}
