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

fn report(diagnostics: &[crate::check::Diagnostic], fallback: &str) -> i32 {
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
    report(diagnostics, fallback)
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

fn cmd_check(path: &Path, json: bool) -> i32 {
    let loaded = match load(path) {
        Ok(loaded) => loaded,
        Err(error) => return fail(error),
    };
    let diagnostics = crate::check::check(&loaded.decls);
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
    )
}

/// The arguments after the command, split into positional values and options.
struct Args {
    positional: Vec<String>,
    flags: Vec<String>,
    options: std::collections::HashMap<String, String>,
}

const VALUE_FLAGS: &[&str] = &["--top", "--threshold", "--out", "--goals", "--k", "--apply"];

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
    let Some(target) = args.at(0) else {
        println!("usage: engine-core {command} FILE");
        return 2;
    };
    let path = PathBuf::from(target);
    match command.as_str() {
        "check" => cmd_check(&path, args.has("--json")),
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
        _ => {
            println!("unknown command {command}");
            2
        }
    }
}
