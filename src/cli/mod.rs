//! Command-line front end: one argument model, one dispatch path, one help table.
//!
//! The commands live in four files that follow the groups on the help screen -- checking,
//! reasoning, repairing, emitting.  Nothing here decides anything about a document; every command
//! loads, calls one library function and prints.  When a command starts to look like it has an
//! opinion, the opinion belongs in the library, not here.

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use crate::program::{self, Program};
use crate::syntax::{self, Decl, Val};
mod checking;
mod emitting;
mod reasoning;
mod repairing;

use checking::*;
use emitting::*;
use reasoning::*;
use repairing::*;

pub struct Loaded {
    pub decls: Vec<Decl>,
    pub text: String,
    pub path: PathBuf,
    /// A document that does not parse has no declarations, but it does have a diagnostic.
    pub syntax: Vec<crate::check::Diagnostic>,
    /// One entry per file when the path is a directory, so each diagnostic can be rendered
    /// against its own source.
    pub sources: Vec<(String, String)>,
}

fn syntax_diagnostic(file: &str, error: &syntax::ParseError) -> crate::check::Diagnostic {
    crate::check::Diagnostic {
        code: error.code,
        message: error.message.clone(),
        line: error.line,
        col: error.col,
        end_line: error.line,
        end_col: error.col + 1,
        file: file.to_string(),
        witness: None,
    }
}

/// Load and parse, keeping syntax errors as diagnostics.  Callers that cannot analyse a
/// broken document use `load`, which turns the first one into a printed message.
pub fn load_for_check(path: &Path) -> Result<Loaded, String> {
    if path.as_os_str() == "-" {
        // a document from a pipe: editors and CI both want this
        let mut text = String::new();
        std::io::stdin()
            .read_to_string(&mut text)
            .map_err(|error| format!("standard input: {error}"))?;
        let (decls, errors) = syntax::parse_recovering(&text);
        let syntax = errors
            .iter()
            .map(|error| syntax_diagnostic("-", error))
            .collect();
        return Ok(Loaded {
            decls,
            text,
            path: path.to_path_buf(),
            syntax,
            sources: Vec::new(),
        });
    }
    if path.is_dir() {
        let mut files: Vec<PathBuf> = fs::read_dir(path)
            .map_err(|error| format!("{}: {error}", path.display()))?
            .filter_map(|entry| entry.ok().map(|item| item.path()))
            .filter(|item| item.extension().map(|ext| ext == "tsare").unwrap_or(false))
            .collect();
        files.sort();
        let mut decls = Vec::new();
        let mut syntax_errors = Vec::new();
        let mut sources: Vec<(String, String)> = Vec::new();
        for file in &files {
            let source =
                fs::read_to_string(file).map_err(|error| format!("{}: {error}", file.display()))?;
            let name = file
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            sources.push((name.clone(), source.clone()));
            let (mut parsed, errors) = syntax::parse_recovering(&source);
            for decl in &mut parsed {
                decl.file = name.clone();
            }
            decls.extend(parsed);
            for error in &errors {
                syntax_errors.push(syntax_diagnostic(&name, error));
            }
        }
        return Ok(Loaded {
            decls,
            text: String::new(),
            path: path.to_path_buf(),
            syntax: syntax_errors,
            sources,
        });
    }
    let text = fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let (decls, errors) = syntax::parse_recovering(&text);
    let syntax_errors: Vec<crate::check::Diagnostic> = errors
        .iter()
        .map(|error| syntax_diagnostic("", error))
        .collect();
    Ok(Loaded {
        decls,
        text,
        path: path.to_path_buf(),
        syntax: syntax_errors,
        sources: Vec::new(),
    })
}

pub fn load(path: &Path) -> Result<Loaded, String> {
    let loaded = load_for_check(path)?;
    if let Some(first) = loaded.syntax.first() {
        return Err(format!(
            "{}:{}:{}: error[E-SYNTAX]: {}",
            path.display(),
            first.line,
            first.col,
            first.message
        ));
    }
    Ok(loaded)
}

fn fail(error: String) -> i32 {
    println!("{error}");
    1
}

pub fn escape(text: &str) -> String {
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
    diagnostics
        .iter()
        .filter(|item| item.code.starts_with("E-"))
        .count()
}

fn warning_count(diagnostics: &[crate::check::Diagnostic]) -> usize {
    diagnostics.len() - error_count(diagnostics)
}

/// Diagnostics as JSON records, the contract in PROTOCOL.md.
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

/// `check FILE --against KERNEL`: the kernel is a prelude. Its names may be used, not
/// redefined, and a claim inside one of its layers must stand on one of that layer's laws.
/// One document, with every declaration labelled by the file it came from.  A merge is only
/// honest if a reader can tell which side a name is on, and the diagnostics already carry that.
fn tagged(path: &Path, loaded: &Loaded) -> Vec<crate::syntax::Decl> {
    let name = path.display().to_string();
    let mut decls = loaded.decls.clone();
    for decl in &mut decls {
        decl.file = name.clone();
    }
    decls
}

fn learning_map(
    learnings: &[crate::induce::Learning],
) -> std::collections::HashMap<String, crate::induce::Learning> {
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

/// One line of the help screen and one name the dispatcher must accept.  They were two lists and
/// they had already drifted -- `novel`'s description sat under `abstract` -- which is what two
/// lists do.  The usage text keeps its shape: two spaces, the name, the arguments, the sentence.
struct Command(&'static str, &'static str, &'static str);

const GROUPS: &[&str] = &["checking", "reasoning", "repairing", "emitting"];

/// Commands that exist for a protocol rather than a person: they dispatch but are not listed.
const HIDDEN: &[&str] = &["mcp"];

const COMMANDS: &[Command] = &[
        Command("checking", "check", "check PATH [--json] [--goals F] [--sugar] [--cache] [--against OTHER] [--strict]   parse and check (the default)"),
        Command("checking", "project", "project DIR                                          the same, grouped by file"),
        Command("reasoning", "engine", "engine PATH [--json]         held, conditional, blocked, chains, gates"),
        Command("reasoning", "argue", "argue PATH [--against OTHER] which conclusions survive attack"),
        Command("reasoning", "confront", "confront LEFT RIGHT          which rules attack which, and which attacks nobody answered"),
        Command("reasoning", "derive", "derive PATH                  which conclusions are grounded in facts"),
        Command("reasoning", "debt", "debt PATH [--all]            which facts transitively gate a conclusion"),
        Command("reasoning", "impact", "impact PATH                  dead facts, single support, load-bearing nodes"),
        Command("reasoning", "next", "next PATH [--json]            open decisions, ranked by what they touch"),
        Command("reasoning", "ask", "ask PATH [--json] [--one]   the next question, ranked by what one answer settles"),
        Command("reasoning", "novel", "novel PATH --attribute N [--holds IDS]\n                               is a proposed attribute new, redundant, or a recombination"),
        Command("reasoning", "meet", "meet PATH LEFT RIGHT        the most general form two declarations share, and the holes it leaves"),
        Command("reasoning", "shape", "shape PATH [--top N] [--depth D] [--layers] [--against OTHER [--near N]] [--json]  the forms declarations share"),
        Command("reasoning", "imply", "imply PATH [--json] [--max-antecedent N] [--min-support N]"),
        Command("reasoning", "abstract", "abstract PATH [--min-support N] [--max-intent N]\n                               concepts the attributes already hold but nobody named"),
        Command("reasoning", "cost", "cost PATH [--top N] [--json]  external confirmations the document demands"),
        Command("reasoning", "weigh", "weigh PATH [--threshold X]   confidence propagated by min/max"),
        Command("reasoning", "fca", "fca PATH                     what a concept's examples share, and what they imply"),
        Command("reasoning", "cat", "cat PATH                     the pipeline read as a category"),
        Command("reasoning", "moves", "moves PATH                   six representation moves as candidates"),
        Command("reasoning", "assumptions", "assumptions PATH [--top N]   load-bearing declarations without support"),
        Command("reasoning", "delta", "delta BEFORE AFTER [--json] [--record STORE --doc DOC [--action A]]  what changed between two `check --json` reports"),
        Command("reasoning", "whatif", "whatif PATH NAME             what falls when one assumption is removed"),
        Command("reasoning", "kinds", "kinds                        the obligation table: what each kind owes"),
        Command("repairing", "fix", "fix PATH [--apply OUT]          verified repair candidates, and applying them"),
        Command("repairing", "suggest", "suggest PATH [--json]           repairs with an applicability"),
        Command("repairing", "apply", "apply PATH [--dry-run]          splice machine repairs and recompile"),
        Command("repairing", "propose", "propose PATH --proposals F [--policy P] [--apply]\n                                  gate values an outside model wants to fill in\n                                  policies: harmless (default), strict, improve"),
        Command("emitting", "outline", "outline PATH                        the document read as prose, grounds in the margin"),
        Command("emitting", "build", "build PATH [--out DIR] [--goals F]   obligations, clauses, open items, predicates"),
        Command("emitting", "graph", "graph PATH [--out FILE]             the relation net as Mermaid"),
        Command("emitting", "map", "map LEFT RIGHT [--top N]            structure mapping between two documents"),
        Command("emitting", "induce", "induce PATH                         the clause each concept's examples support"),
        Command("emitting", "pattern", "pattern PATH CONCEPT [--vars]       the structure that separates the examples"),
        Command("emitting", "audit", "audit PATH                          can a kind be written with a smaller vocabulary"),
        Command("emitting", "egraph", "egraph PATH [--write OUT]           merge equivalent declarations, rewrite references"),
        Command("emitting", "mem", "mem [--store FILE] [--recall CODE...] [--k N]  the case store: audit it, or recall situations"),
];

fn usage() {
    println!(
        "tsare {} — thought structure-assisted reasoning engine (document format {})",
        env!("CARGO_PKG_VERSION"),
        crate::syntax::FORMAT_VERSION
    );
    println!();
    println!("usage: tsare [COMMAND] PATH [options]");
    println!("       tsare kinds | mem | --explain CODE | --version | --help");
    println!();
    println!("PATH is a .tsare document, a directory of them, or - for standard input.");
    for group in GROUPS {
        println!();
        println!("{group}");
        for command in COMMANDS.iter().filter(|command| command.0 == *group) {
            println!("  {}", command.2);
        }
    }
    println!();
    println!("exit codes");
    println!("  0  no errors (warnings are allowed)");
    println!("  1  errors found, or a proposal refused");
    println!("  2  usage problem: no document named, or a command that does not exist");
}

fn is_command(name: &str) -> bool {
    COMMANDS.iter().any(|command| command.1 == name) || HIDDEN.contains(&name)
}

/// A bare path is shorthand for `check PATH`.  A misspelled command is not a path: guessing turns
/// `tsare shap doc.tsare` into "shap: the system cannot find the file", which reads as the tool
/// being broken instead of the invocation being wrong, and would silently check a file that
/// happens to carry the typo's name.
fn looks_like_a_document(word: &str) -> bool {
    word == "-" || word.ends_with(".tsare") || Path::new(word).exists()
}

/// The arguments after the command, split into positional values and options.
struct Args {
    positional: Vec<String>,
    flags: Vec<String>,
    options: std::collections::HashMap<String, String>,
}

const VALUE_FLAGS: &[&str] = &[
    "--top",
    "--depth",
    "--threshold",
    "--out",
    "--goals",
    "--k",
    "--apply",
    "--proposals",
    "--policy",
    "--attribute",
    "--holds",
    "--against",
    "--max-antecedent",
    "--min-support",
    "--max-intent",
    "--record",
    "--doc",
    "--action",
    "--store",
    "--near",
    "--write",
];

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
        Args {
            positional,
            flags,
            options,
        }
    }

    fn has(&self, flag: &str) -> bool {
        self.flags.iter().any(|item| item == flag)
    }

    fn option(&self, flag: &str) -> Option<&str> {
        self.options.get(flag).map(|value| value.as_str())
    }

    fn number(&self, flag: &str, fallback: f64) -> f64 {
        self.option(flag)
            .and_then(|value| value.parse().ok())
            .unwrap_or(fallback)
    }

    fn count(&self, flag: &str, fallback: usize) -> usize {
        self.option(flag)
            .and_then(|value| value.parse().ok())
            .unwrap_or(fallback)
    }

    fn at(&self, index: usize) -> Option<&str> {
        self.positional.get(index).map(|item| item.as_str())
    }
}

pub fn run() -> i32 {
    let raw: Vec<String> = std::env::args().skip(1).collect();
    if let Some(position) = raw.iter().position(|arg| arg == "--explain") {
        let Some(code) = raw.get(position + 1) else {
            println!("usage: tsare --explain CODE");
            return 2;
        };
        return match crate::explain::explain(code) {
            Some(text) => {
                println!("{code}\n");
                println!("{text}");
                0
            }
            None => {
                println!("no explanation for {code}");
                1
            }
        };
    }
    if raw.iter().any(|arg| arg == "--version" || arg == "-V") {
        println!(
            "tsare {} (document format {})",
            env!("CARGO_PKG_VERSION"),
            crate::syntax::FORMAT_VERSION
        );
        return 0;
    }
    if raw.is_empty() || raw.iter().any(|arg| arg == "--help" || arg == "-h") {
        usage();
        return 0;
    }
    let (command, rest) = if is_command(&raw[0]) {
        (raw[0].clone(), raw[1..].to_vec())
    } else if looks_like_a_document(&raw[0]) {
        ("check".to_string(), raw.clone())
    } else {
        println!("unknown command {}; `tsare help` lists them", raw[0]);
        return 2;
    };
    let args = Args::parse(&rest);
    if command == "mcp" {
        return crate::mcp::serve();
    }
    if command == "kinds" {
        return cmd_kinds();
    }
    if command == "mem" {
        let store = args
            .option("--store")
            .map(PathBuf::from)
            .unwrap_or_else(crate::cases::store_path);
        let codes: Vec<String> = args.positional.clone();
        let recall = if args.has("--recall") {
            Some(codes.as_slice())
        } else {
            None
        };
        return cmd_mem(&store, recall, args.count("--k", 3));
    }
    let Some(target) = args.at(0) else {
        println!("usage: tsare {command} FILE");
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
            args.option("--against"),
            args.has("--strict"),
        ),
        "engine" => cmd_engine(&path, args.has("--json")),
        "suggest" => cmd_suggest(&path, args.has("--json")),
        "apply" => cmd_apply(&path, args.has("--dry-run")),
        "argue" => cmd_argue(&path, args.option("--against").map(Path::new)),
        "confront" => {
            let Some(other) = args.at(1) else {
                println!("usage: tsare confront LEFT RIGHT");
                return 2;
            };
            cmd_confront(&path, Path::new(other))
        }
        "derive" => cmd_derive(&path),
        "debt" => cmd_debt(&path, args.has("--all")),
        "impact" => cmd_impact(&path),
        "next" => cmd_next(&path, args.has("--json")),
        "ask" => cmd_ask(&path, &args),
        "novel" => cmd_novel(&path, &args),
        "meet" => cmd_meet(&path, &args),
        "shape" => cmd_shape(
            &path,
            args.count("--top", 8),
            args.count("--depth", 0),
            args.has("--layers"),
            args.option("--against"),
            args.count("--near", 0),
            args.has("--json"),
        ),
        "imply" => cmd_imply(&path, &args),
        "cost" => cmd_cost(&path, args.count("--top", 8), args.has("--json")),
        "weigh" => cmd_weigh(&path, args.number("--threshold", 0.7)),
        "cat" => cmd_cat(&path),
        "moves" => cmd_moves(&path),
        "assumptions" => cmd_assumptions(&path, args.count("--top", 12)),
        "audit" => cmd_audit(&path),
        "egraph" => cmd_egraph(&path, args.option("--write")),
        "whatif" => {
            let Some(name) = args.at(1) else {
                println!("usage: tsare whatif FILE NAME");
                return 2;
            };
            cmd_whatif(&path, name)
        }
        "graph" => cmd_graph(&path, args.option("--out")),
        "fix" => cmd_fix(&path, args.option("--apply")),
        "map" => {
            let Some(right) = args.at(1) else {
                println!("usage: tsare map LEFT RIGHT");
                return 2;
            };
            cmd_map(&path, Path::new(right), args.count("--top", 8))
        }
        "delta" => {
            let Some(after) = args.at(1) else {
                println!("usage: tsare delta BEFORE.json AFTER.json");
                return 2;
            };
            cmd_delta(
                &path,
                Path::new(after),
                args.has("--json"),
                args.option("--record"),
                args.option("--doc"),
                args.option("--action"),
            )
        }
        "induce" => cmd_induce(&path),
        "propose" => {
            let Some(proposals) = args.option("--proposals") else {
                println!("usage: tsare propose FILE --proposals FILE.json [--apply]");
                return 2;
            };
            cmd_propose(
                &path,
                proposals,
                args.has("--apply"),
                args.option("--policy").unwrap_or("harmless"),
            )
        }
        "abstract" => cmd_abstract(&path, &args),
        "fca" => cmd_fca(&path),
        "pattern" => {
            let Some(name) = args.at(1) else {
                println!("usage: tsare pattern FILE CONCEPT");
                return 2;
            };
            cmd_pattern(&path, name, args.has("--vars"))
        }
        "build" => cmd_build(
            &path,
            args.option("--out").unwrap_or("out"),
            args.option("--goals"),
        ),
        "project" => cmd_project(&path),
        "outline" => cmd_outline(&path),
        _ => {
            println!("unknown command {command}");
            2
        }
    }
}
