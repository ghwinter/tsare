//! The command table: the help and the dispatch read the same list.
//!
//! One row per command: its group, the usage line, whether it takes a target file, and the
//! function that runs it.  Adding a command is one row, not a row and a match arm.

use super::*;

pub struct Command {
    pub group: &'static str,
    pub name: &'static str,
    pub usage: &'static str,
    pub needs_target: bool,
    pub run: fn(&Path, &Args) -> Outcome,
}

pub const GROUPS: &[&str] = &["checking", "reasoning", "repairing", "emitting"];

/// Commands that exist for a protocol rather than a person: they dispatch but are not listed.
pub const HIDDEN: &[&str] = &["mcp"];

pub const COMMANDS: &[Command] = &[
    Command { group: "checking", name: "check", usage: "check PATH [--json] [--goals F] [--sugar] [--cache] [--against OTHER] [--strict] [--budget N]   parse and check (the default)", needs_target: true, run: |path: &Path, args: &Args| { let _ = (path, args); cmd_check(
            path,
            CheckOptions {
                json: args.has("--json"),
                sugar: args.has("--sugar"),
                goals: args.option("--goals").map(str::to_string),
                cache: args.has("--cache"),
                against: args.option("--against").map(str::to_string),
                strict: args.has("--strict"),
                budget: args.option("--budget").and_then(|value| value.parse().ok()),
            },
        ) } },
    Command { group: "checking", name: "status", usage: "status PATH [--json] [--trace FILE] [--budget N]   the decision, the counts and the next question", needs_target: true, run: |path: &Path, args: &Args| { let _ = (path, args); cmd_status(path, args.has("--json"), args.option("--trace"), args.option("--budget").and_then(|value| value.parse().ok())) } },
    Command { group: "checking", name: "contracts", usage: "contracts [--json]   the operator contracts: input, output and effect", needs_target: false, run: |_path: &Path, args: &Args| cmd_contracts(args.has("--json")) },
    Command { group: "checking", name: "provenance", usage: "provenance PATH [--json]    where each quoted claim sits in its source", needs_target: true, run: |path: &Path, args: &Args| { let _ = (path, args); cmd_provenance(path, args.has("--json")) } },
    Command { group: "checking", name: "extract", usage: "extract PATH [--json] [--terms A,B] [--terms-from FILE] [--write F]   the definitions a page states", needs_target: true, run: |path: &Path, args: &Args| { let _ = (path, args); cmd_extract(path, args.has("--json"), args.option("--terms"), args.option("--terms-from"), args.option("--write")) } },
    Command { group: "checking", name: "project", usage: "project DIR                                          the same, grouped by file", needs_target: true, run: |path: &Path, args: &Args| { let _ = (path, args); cmd_project(path) } },
    Command { group: "checking", name: "trace", usage: "trace PATH [--out FILE | --append FILE]   one run, or append it to a session", needs_target: true, run: |path: &Path, args: &Args| { let _ = (path, args); cmd_trace(path, args.option("--out"), args.option("--append")) } },
    Command { group: "checking", name: "replay", usage: "replay TRACE [--diff]        re-run a recorded session, show the write-backs, compare", needs_target: true, run: |path: &Path, args: &Args| { let _ = (path, args); cmd_replay(path, args.has("--diff")) } },
    Command { group: "checking", name: "grow", usage: "grow LIBRARY SOURCE NAME --out FILE   add one checked declaration to a library", needs_target: true, run: |path: &Path, args: &Args| { let _ = (path, args); {
            let (Some(library), Some(source), Some(name)) = (args.at(0), args.at(1), args.at(2))
            else {
                println!("usage: tsare grow LIBRARY SOURCE NAME --out FILE");
                return Outcome::Usage;
            };
            let Some(out) = args.option("--out") else {
                println!("usage: tsare grow LIBRARY SOURCE NAME --out FILE");
                return Outcome::Usage;
            };
            cmd_grow(Path::new(library), Path::new(source), name, out)
        } } },
    Command { group: "checking", name: "library", usage: "library LIBRARY [--cases STORE] [--json]  what the grown abstracts cover, and how often they are used", needs_target: true, run: |path: &Path, args: &Args| { let _ = (path, args); cmd_library(path, args.option("--cases"), args.has("--json")) } },
    Command { group: "checking", name: "save", usage: "save FILE [--store DIR]      store a ledger under its content hash", needs_target: true, run: |path: &Path, args: &Args| { let _ = (path, args); cmd_save(path, args.option("--store").unwrap_or(".tsare/versions")) } },
    Command { group: "checking", name: "log", usage: "log STORE                    the content hashes the store holds", needs_target: true, run: |path: &Path, args: &Args| { let _ = (path, args); cmd_log(path) } },
    Command { group: "checking", name: "restore", usage: "restore HASH [--store DIR] --out FILE   write a stored ledger back", needs_target: true, run: |path: &Path, args: &Args| { let _ = (path, args); {
            let Some(out) = args.option("--out") else {
                println!("usage: tsare restore HASH [--store DIR] --out FILE");
                return Outcome::Usage;
            };
            cmd_restore(
                args.at(0).unwrap_or(""),
                args.option("--store").unwrap_or(".tsare/versions"),
                out,
            )
        } } },
    Command { group: "reasoning", name: "engine", usage: "engine PATH [--json]         held, conditional, blocked, chains, gates", needs_target: true, run: |path: &Path, args: &Args| { let _ = (path, args); cmd_engine(path, args.has("--json")) } },
    Command { group: "reasoning", name: "arrows", usage: "arrows PATH [--json]         the relations the document states, with kinds and spans", needs_target: true, run: |path: &Path, args: &Args| { let _ = (path, args); cmd_arrows(path, args.has("--json")) } },
    Command { group: "reasoning", name: "argue", usage: "argue PATH [--against OTHER] [--json]   which conclusions survive attack", needs_target: true, run: |path: &Path, args: &Args| { let _ = (path, args); cmd_argue(
            path,
            args.option("--against").map(Path::new),
            args.has("--json"),
        ) } },
    Command { group: "reasoning", name: "confront", usage: "confront LEFT RIGHT          which rules attack which, and which attacks nobody answered", needs_target: true, run: |path: &Path, args: &Args| { let _ = (path, args); {
            let Some(other) = args.at(1) else {
                println!("usage: tsare confront LEFT RIGHT");
                return Outcome::Usage;
            };
            cmd_confront(path, Path::new(other))
        } } },
    Command { group: "reasoning", name: "derive", usage: "derive PATH                  which conclusions are grounded in facts", needs_target: true, run: |path: &Path, args: &Args| { let _ = (path, args); cmd_derive(path) } },
    Command { group: "reasoning", name: "debt", usage: "debt PATH [--all]            which facts transitively gate a conclusion", needs_target: true, run: |path: &Path, args: &Args| { let _ = (path, args); cmd_debt(path, args.has("--all")) } },
    Command { group: "reasoning", name: "impact", usage: "impact PATH                  dead facts, single support, load-bearing nodes", needs_target: true, run: |path: &Path, args: &Args| { let _ = (path, args); cmd_impact(path) } },
    Command { group: "reasoning", name: "next", usage: "next PATH [--json]            open decisions, ranked by what they touch", needs_target: true, run: |path: &Path, args: &Args| { let _ = (path, args); cmd_next(path, args.has("--json")) } },
    Command { group: "reasoning", name: "ask", usage: "ask PATH [--json] [--one]   the next question, ranked by what one answer settles", needs_target: true, run: |path: &Path, args: &Args| { let _ = (path, args); cmd_ask(path, args) } },
    Command { group: "reasoning", name: "novel", usage: "novel PATH --attribute N [--holds IDS]\n                               is a proposed attribute new, redundant, or a recombination", needs_target: true, run: |path: &Path, args: &Args| { let _ = (path, args); cmd_novel(path, args) } },
    Command { group: "reasoning", name: "meet", usage: "meet PATH LEFT RIGHT        the most general form two declarations share, and the holes it leaves", needs_target: true, run: |path: &Path, args: &Args| { let _ = (path, args); cmd_meet(path, args) } },
    Command { group: "reasoning", name: "hint", usage: "hint PATH [--json]         the concept a form implies and the document does not declare", needs_target: true, run: |path: &Path, args: &Args| { let _ = (path, args); cmd_hint(path, args.has("--json")) } },
    Command { group: "reasoning", name: "shape", usage: "shape PATH [--top N] [--depth D] [--layers] [--against OTHER [--near N]] [--json]  the forms declarations share", needs_target: true, run: |path: &Path, args: &Args| { let _ = (path, args); cmd_shape(
            path,
            args.count("--top", 8),
            args.count("--depth", 0),
            args.has("--layers"),
            args.option("--against"),
            args.count("--near", 0),
            args.has("--json"),
        ) } },
    Command { group: "reasoning", name: "imply", usage: "imply PATH [--json] [--max-antecedent N] [--min-support N]", needs_target: true, run: |path: &Path, args: &Args| { let _ = (path, args); cmd_imply(path, args) } },
    Command { group: "reasoning", name: "abstract", usage: "abstract PATH [--min-support N] [--max-intent N]\n                               concepts the attributes already hold but nobody named", needs_target: true, run: |path: &Path, args: &Args| { let _ = (path, args); cmd_abstract(path, args) } },
    Command { group: "reasoning", name: "cost", usage: "cost PATH [--top N] [--json]  external confirmations the document demands", needs_target: true, run: |path: &Path, args: &Args| { let _ = (path, args); cmd_cost(path, args.count("--top", 8), args.has("--json")) } },
    Command { group: "reasoning", name: "decide", usage: "decide PATH [--json]         closed, closable or open: what the document still owes", needs_target: true, run: |path: &Path, args: &Args| { let _ = (path, args); cmd_decide(path, args.has("--json")) } },
    Command { group: "reasoning", name: "epistemic", usage: "epistemic PATH [--json]      verified, inferred or unverified: how each conclusion is known", needs_target: true, run: |path: &Path, args: &Args| { let _ = (path, args); cmd_epistemic(path, args.has("--json")) } },
    Command { group: "reasoning", name: "weigh", usage: "weigh PATH [--threshold X]   confidence propagated by min/max", needs_target: true, run: |path: &Path, args: &Args| { let _ = (path, args); cmd_weigh(path, args.number("--threshold", 0.7)) } },
    Command { group: "reasoning", name: "fca", usage: "fca PATH                     what a concept's examples share, and what they imply", needs_target: true, run: |path: &Path, args: &Args| { let _ = (path, args); cmd_fca(path) } },
    Command { group: "reasoning", name: "cat", usage: "cat PATH                     the pipeline read as a category", needs_target: true, run: |path: &Path, args: &Args| { let _ = (path, args); cmd_cat(path) } },
    Command { group: "reasoning", name: "moves", usage: "moves PATH                   six representation moves as candidates", needs_target: true, run: |path: &Path, args: &Args| { let _ = (path, args); cmd_moves(path) } },
    Command { group: "reasoning", name: "assumptions", usage: "assumptions PATH [--top N]   load-bearing declarations without support", needs_target: true, run: |path: &Path, args: &Args| { let _ = (path, args); cmd_assumptions(path, args.count("--top", 12)) } },
    Command { group: "reasoning", name: "delta", usage: "delta BEFORE AFTER [--json] [--record STORE --doc DOC [--action A]]  what changed between two `check --json` reports", needs_target: true, run: |path: &Path, args: &Args| { let _ = (path, args); {
            let Some(after) = args.at(1) else {
                println!("usage: tsare delta BEFORE.json AFTER.json");
                return Outcome::Usage;
            };
            cmd_delta(
                path,
                Path::new(after),
                args.has("--json"),
                args.option("--record"),
                args.option("--doc"),
                args.option("--action"),
            )
        } } },
    Command { group: "reasoning", name: "diff", usage: "diff BEFORE AFTER [--json]   what changed between two ledgers", needs_target: true, run: |path: &Path, args: &Args| { let _ = (path, args); {
            let Some(after) = args.at(1) else {
                println!("usage: tsare diff BEFORE AFTER");
                return Outcome::Usage;
            };
            cmd_diff(path, Path::new(after), args.has("--json"))
        } } },
    Command { group: "reasoning", name: "merge", usage: "merge BASE OURS THEIRS --out FILE [--json]   structural three-way merge", needs_target: true, run: |path: &Path, args: &Args| { let _ = (path, args); {
            let (Some(ours), Some(theirs)) = (args.at(1), args.at(2)) else {
                println!("usage: tsare merge BASE OURS THEIRS --out FILE [--json]");
                return Outcome::Usage;
            };
            cmd_merge(
                path,
                Path::new(ours),
                Path::new(theirs),
                args.option("--out"),
                args.has("--json"),
            )
        } } },
    Command { group: "reasoning", name: "blame", usage: "blame STORE NAME [--json]   which version last changed each field", needs_target: true, run: |path: &Path, args: &Args| { let _ = (path, args); {
            let Some(name) = args.at(1) else {
                println!("usage: tsare blame STORE NAME [--json]");
                return Outcome::Usage;
            };
            cmd_blame(path, name, args.has("--json"))
        } } },
    Command { group: "reasoning", name: "chain", usage: "chain FILE NAME [--json]   the support path from a claim to its evidence", needs_target: true, run: |path: &Path, args: &Args| { let _ = (path, args); {
            let Some(name) = args.at(1) else {
                println!("usage: tsare chain FILE NAME [--json]");
                return Outcome::Usage;
            };
            cmd_chain(path, name, args.has("--json"))
        } } },
    Command { group: "reasoning", name: "chains", usage: "chains FILE [--json]   every open support path", needs_target: true, run: |path: &Path, args: &Args| { let _ = (path, args); cmd_chains(path, args.has("--json")) } },
    Command { group: "reasoning", name: "whatif", usage: "whatif PATH NAME             what falls when one assumption is removed", needs_target: true, run: |path: &Path, args: &Args| { let _ = (path, args); {
            let Some(name) = args.at(1) else {
                println!("usage: tsare whatif FILE NAME");
                return Outcome::Usage;
            };
            cmd_whatif(path, name)
        } } },
    Command { group: "reasoning", name: "retract", usage: "retract FILE NAME [--out FILE] [--json]   remove one assumption and enumerate the conclusions whose status changes", needs_target: true, run: |path: &Path, args: &Args| { let _ = (path, args); {
            let Some(name) = args.at(1) else {
                println!("usage: tsare retract FILE NAME [--out FILE] [--json]");
                return Outcome::Usage;
            };
            cmd_retract(path, name, args.option("--out"), args.has("--json"))
        } } },
    Command { group: "reasoning", name: "kinds", usage: "kinds                        the obligation table: what each kind owes", needs_target: false, run: |path: &Path, args: &Args| { let _ = (path, args); cmd_kinds() } },
    Command { group: "repairing", name: "fix", usage: "fix PATH [--apply OUT]          verified repair candidates, and applying them", needs_target: true, run: |path: &Path, args: &Args| { let _ = (path, args); cmd_fix(path, args.option("--apply")) } },
    Command { group: "repairing", name: "suggest", usage: "suggest PATH [--json]           repairs with an applicability", needs_target: true, run: |path: &Path, args: &Args| { let _ = (path, args); cmd_suggest(path, args.has("--json")) } },
    Command { group: "repairing", name: "apply", usage: "apply PATH [--dry-run] [--record STORE] [--action A] [--library FILE]  splice machine repairs and recompile, recording the process", needs_target: true, run: |path: &Path, args: &Args| { let _ = (path, args); cmd_apply(path, args.has("--dry-run"), args.option("--record"), args.option("--action").unwrap_or("apply"), args.option("--library")) } },
    Command { group: "repairing", name: "propose", usage: "propose PATH --proposals F [--policy P] [--apply]\n                                  gate values an outside model wants to fill in\n                                  policies: harmless (default), strict, improve", needs_target: true, run: |path: &Path, args: &Args| { let _ = (path, args); {
            let Some(proposals) = args.option("--proposals") else {
                println!("usage: tsare propose FILE --proposals FILE.json [--apply]");
                return Outcome::Usage;
            };
            cmd_propose(
                path,
                proposals,
                args.has("--apply"),
                args.option("--policy").unwrap_or("harmless"),
            )
        } } },
    Command { group: "emitting", name: "fmt", usage: "fmt PATH                             the document in one spelling, for a stable diff and hash", needs_target: true, run: |path: &Path, args: &Args| { let _ = (path, args); cmd_fmt(path) } },
    Command { group: "emitting", name: "outline", usage: "outline PATH                        the document read as prose, grounds in the margin", needs_target: true, run: |path: &Path, args: &Args| { let _ = (path, args); cmd_outline(path) } },
    Command { group: "emitting", name: "build", usage: "build PATH [--out DIR] [--goals F]   obligations, clauses, open items, predicates", needs_target: true, run: |path: &Path, args: &Args| { let _ = (path, args); cmd_build(
            path,
            args.option("--out").unwrap_or("out"),
            args.option("--goals"),
        ) } },
    Command { group: "emitting", name: "graph", usage: "graph PATH [--out FILE]             the relation net as Mermaid", needs_target: true, run: |path: &Path, args: &Args| { let _ = (path, args); cmd_graph(path, args.option("--out")) } },
    Command { group: "emitting", name: "views", usage: "views PATH [--as VIEW] [--check] [--json]   five projections of the same document", needs_target: true, run: |path: &Path, args: &Args| { let _ = (path, args); cmd_views(
            path,
            args.has("--json"),
            args.option("--as"),
            args.has("--check"),
        ) } },
    Command { group: "emitting", name: "map", usage: "map LEFT RIGHT [--top N]            structure mapping between two documents", needs_target: true, run: |path: &Path, args: &Args| { let _ = (path, args); {
            let Some(right) = args.at(1) else {
                println!("usage: tsare map LEFT RIGHT");
                return Outcome::Usage;
            };
            cmd_map(path, Path::new(right), args.count("--top", 8))
        } } },
    Command { group: "emitting", name: "induce", usage: "induce PATH                         the clause each concept's examples support", needs_target: true, run: |path: &Path, args: &Args| { let _ = (path, args); cmd_induce(path) } },
    Command { group: "emitting", name: "pattern", usage: "pattern PATH CONCEPT [--vars]       the structure that separates the examples", needs_target: true, run: |path: &Path, args: &Args| { let _ = (path, args); {
            let Some(name) = args.at(1) else {
                println!("usage: tsare pattern FILE CONCEPT");
                return Outcome::Usage;
            };
            cmd_pattern(path, name, args.has("--vars"))
        } } },
    Command { group: "emitting", name: "audit", usage: "audit PATH                          can a kind be written with a smaller vocabulary", needs_target: true, run: |path: &Path, args: &Args| { let _ = (path, args); cmd_audit(path) } },
    Command { group: "emitting", name: "egraph", usage: "egraph PATH [--write OUT]           merge equivalent declarations, rewrite references", needs_target: true, run: |path: &Path, args: &Args| { let _ = (path, args); cmd_egraph(path, args.option("--write")) } },
    Command { group: "emitting", name: "mem", usage: "mem [--store FILE] [--recall CODE...] [--process SIG] [--k N] [--json]  the case store: audit it, or recall by situation or process", needs_target: false, run: |path: &Path, args: &Args| { let _ = (path, args); { let store = args.option("--store").map(PathBuf::from).unwrap_or_else(crate::cases::store_path); let codes: Vec<String> = args.positional.clone(); let recall = if args.has("--recall") { Some(codes.as_slice()) } else { None }; cmd_mem(&store, recall, args.option("--process"), args.count("--k", 3), args.has("--json")) } } },
];

pub fn find(name: &str) -> Option<&'static Command> {
    COMMANDS.iter().find(|command| command.name == name)
}

pub(crate) fn dispatch(path: &Path, command: &str, args: &Args) -> Outcome {
    match find(command) {
        Some(entry) => (entry.run)(path, args),
        None => {
            println!("unknown command {command}");
            Outcome::Usage
        }
    }
}
