use super::*;

/// One line of the help screen and one name the dispatcher must accept.  They were two lists and
/// they had already drifted -- `novel`'s description sat under `abstract` -- which is what two
/// lists do.  The usage text keeps its shape: two spaces, the name, the arguments, the sentence.
struct Command(&'static str, &'static str, &'static str);

const GROUPS: &[&str] = &["checking", "reasoning", "repairing", "emitting"];

/// Commands that exist for a protocol rather than a person: they dispatch but are not listed.
const HIDDEN: &[&str] = &["mcp"];

const COMMANDS: &[Command] = &[
        Command("checking", "check", "check PATH [--json] [--goals F] [--sugar] [--cache] [--against OTHER] [--strict]   parse and check (the default)"),
        Command("checking", "status", "status PATH [--json] [--trace FILE]   the decision, the counts and the next question"),
        Command("checking", "provenance", "provenance PATH [--json]    where each quoted claim sits in its source"),
        Command("checking", "project", "project DIR                                          the same, grouped by file"),
        Command("checking", "trace", "trace PATH [--out FILE | --append FILE]   one run, or append it to a session"),
        Command("checking", "replay", "replay TRACE [--diff]        re-run a recorded session, show the write-backs, compare"),
        Command("checking", "grow", "grow LIBRARY SOURCE NAME --out FILE   add one checked declaration to a library"),
        Command("checking", "save", "save FILE [--store DIR]      store a ledger under its content hash"),
        Command("checking", "log", "log STORE                    the content hashes the store holds"),
        Command("checking", "restore", "restore HASH [--store DIR] --out FILE   write a stored ledger back"),
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
        Command("reasoning", "hint", "hint PATH [--json]         the concept a form implies and the document does not declare"),
        Command("reasoning", "shape", "shape PATH [--top N] [--depth D] [--layers] [--against OTHER [--near N]] [--json]  the forms declarations share"),
        Command("reasoning", "imply", "imply PATH [--json] [--max-antecedent N] [--min-support N]"),
        Command("reasoning", "abstract", "abstract PATH [--min-support N] [--max-intent N]\n                               concepts the attributes already hold but nobody named"),
        Command("reasoning", "cost", "cost PATH [--top N] [--json]  external confirmations the document demands"),
        Command("reasoning", "decide", "decide PATH [--json]         closed, closable or open: what the document still owes"),
        Command("reasoning", "epistemic", "epistemic PATH [--json]      verified, inferred or unverified: how each conclusion is known"),
        Command("reasoning", "weigh", "weigh PATH [--threshold X]   confidence propagated by min/max"),
        Command("reasoning", "fca", "fca PATH                     what a concept's examples share, and what they imply"),
        Command("reasoning", "cat", "cat PATH                     the pipeline read as a category"),
        Command("reasoning", "moves", "moves PATH                   six representation moves as candidates"),
        Command("reasoning", "assumptions", "assumptions PATH [--top N]   load-bearing declarations without support"),
        Command("reasoning", "delta", "delta BEFORE AFTER [--json] [--record STORE --doc DOC [--action A]]  what changed between two `check --json` reports"),
        Command("reasoning", "diff", "diff BEFORE AFTER [--json]   what changed between two ledgers"),
        Command("reasoning", "merge", "merge BASE OURS THEIRS --out FILE [--json]   structural three-way merge"),
        Command("reasoning", "blame", "blame STORE NAME [--json]   which version last changed each field"),
        Command("reasoning", "chain", "chain FILE NAME [--json]   the support path from a claim to its evidence"),
        Command("reasoning", "chains", "chains FILE [--json]   every open support path"),
        Command("reasoning", "whatif", "whatif PATH NAME             what falls when one assumption is removed"),
        Command("reasoning", "kinds", "kinds                        the obligation table: what each kind owes"),
        Command("repairing", "fix", "fix PATH [--apply OUT]          verified repair candidates, and applying them"),
        Command("repairing", "suggest", "suggest PATH [--json]           repairs with an applicability"),
        Command("repairing", "apply", "apply PATH [--dry-run]          splice machine repairs and recompile"),
        Command("repairing", "propose", "propose PATH --proposals F [--policy P] [--apply]\n                                  gate values an outside model wants to fill in\n                                  policies: harmless (default), strict, improve"),
        Command("emitting", "fmt", "fmt PATH                             the document in one spelling, for a stable diff and hash"),
        Command("emitting", "outline", "outline PATH                        the document read as prose, grounds in the margin"),
        Command("emitting", "build", "build PATH [--out DIR] [--goals F]   obligations, clauses, open items, predicates"),
        Command("emitting", "graph", "graph PATH [--out FILE]             the relation net as Mermaid"),
        Command("emitting", "views", "views PATH [--as VIEW] [--check] [--json]   five projections of the same document"),
        Command("emitting", "map", "map LEFT RIGHT [--top N]            structure mapping between two documents"),
        Command("emitting", "induce", "induce PATH                         the clause each concept's examples support"),
        Command("emitting", "pattern", "pattern PATH CONCEPT [--vars]       the structure that separates the examples"),
        Command("emitting", "audit", "audit PATH                          can a kind be written with a smaller vocabulary"),
        Command("emitting", "egraph", "egraph PATH [--write OUT]           merge equivalent declarations, rewrite references"),
        Command("emitting", "mem", "mem [--store FILE] [--recall CODE...] [--k N]  the case store: audit it, or recall situations"),
];

pub(crate) fn usage() {
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
    println!("  2  usage problem");
}

pub(crate) fn is_command(name: &str) -> bool {
    COMMANDS.iter().any(|command| command.1 == name) || HIDDEN.contains(&name)
}

/// A bare path is shorthand for `check PATH`.  A misspelled command is not a path: guessing turns
/// `tsare shap doc.tsare` into "shap: the system cannot find the file", which reads as the tool
/// being broken instead of the invocation being wrong, and would silently check a file that
/// happens to carry the typo's name.
pub(crate) fn looks_like_a_document(word: &str) -> bool {
    word == "-" || word.ends_with(".tsare") || Path::new(word).exists()
}
