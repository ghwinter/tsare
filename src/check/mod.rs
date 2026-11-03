//! Whether a document's declarations hold together: kinds and fields the language has, values of
//! the type their field declares, references that resolve to a declared name of the expected kind,
//! and the relations no single declaration can show -- a justification cycle, two concepts with
//! one extent, one metric constrained both ways without a ranking, a layer reached into without
//! exposing what was reached, a law in a layer that no law at another layer refines.
//!
//! `check` is the entry point.  Every pass reads the declarations, never the diagnostics an
//! earlier pass produced, so the result is the whole list of complaints and not the first.

use std::collections::{HashMap, HashSet};

pub(crate) mod collide;
mod confidence;
pub mod decide;
pub mod epistemic;
pub(crate) mod evidence;
pub mod fix;
pub mod kernel;
mod partition;
pub mod provenance;
pub mod status;

use crate::closure;
use crate::specs::spec;
use crate::syntax::{Decl, Val};

/// The field keys that hold references to other declarations.  They are the document's edges: only
/// these keys are walked for resolution, for the kind the field's type expects, and for the checks
/// that follow one declaration to another.
pub const REF_KEYS: &[&str] = &[
    "from",
    "because",
    "to",
    "source",
    "grounded_by",
    "against",
    "uses",
    "assumes",
    "answers",
    "separates",
    "rival",
    "witness",
    "basis",
    "attacks",
    "checked_by",
    "holds_in",
    "exposes",
    "closes_with",
    "parent",
    "questions",
    "dimension",
    "elements",
    "of",
    "over",
    "signature",
];

/// Values that name a procedure rather than a declaration.
pub const LITERALS: &[&str] = &[
    "tool",
    "doc",
    "code",
    "user",
    "model",
    "observation",
    "experiment",
];

/// One complaint: the code that classifies it, the message that names what is wrong, the span that
/// locates it, and the declaration it names when the complaint is about one.
#[derive(Debug, Clone, PartialEq)]
pub struct Diagnostic {
    pub code: &'static str,
    pub message: String,
    pub line: usize,
    pub col: usize,
    pub end_line: usize,
    pub end_col: usize,
    pub file: String,
    /// The declaration this diagnostic is about.  A code alone is not enough: two `W-COLLIDE`s in
    /// one document are two different complaints, and a repair that silences one elsewhere has not
    /// touched this one.  The name is stable across the edits a repair makes; line numbers are not.
    pub witness: Option<String>,
}

fn diag(code: &'static str, message: String, decl: &Decl) -> Diagnostic {
    Diagnostic {
        code,
        message,
        line: decl.line,
        col: decl.col,
        end_line: decl.end_line,
        end_col: decl.end_col,
        file: decl.file.clone(),
        witness: Some(decl.name.clone()),
    }
}

/// Codes live as `&'static str` so comparisons are cheap; a replayed code is interned here.
pub fn code_of(code: &str) -> &'static str {
    KNOWN_CODES
        .iter()
        .find(|known| **known == code)
        .copied()
        .unwrap_or_else(|| Box::leak(code.to_string().into_boxed_str()))
}

const KNOWN_CODES: &[&str] = &[
    "E-EMPTY-DOCUMENT",
    "E-DUP-FIELD",
    "E-DUP-ID",
    "E-KIND-UNKNOWN",
    "E-FIELD",
    "E-MISSING",
    "E-TYPE",
    "E-UNDEFINED",
    "E-NOTE-REF",
    "E-KIND",
    "E-CYCLE",
    "E-COLLAPSE",
    "E-TENSION",
    "E-LAYER-LEAK",
    "E-UNCOVERED",
    "E-QUOTE",
    "E-QUOTE-SOURCE",
    "W-HOLE",
    "W-CONDITIONAL",
];

/// Every name a value mentions, in the order it is written.  Literals such as `tool` and `user`
/// are names too and appear here; deciding that they are not declarations belongs to the caller,
/// because only the field says what kind of name it holds.
pub fn refs(value: &Val, out: &mut Vec<String>) {
    match value {
        Val::Id(name) => out.push(name.clone()),
        Val::List(items) => items.iter().for_each(|item| refs(item, out)),
        Val::Call(_, args) => args.iter().for_each(|item| refs(item, out)),
        Val::Rec(fields) => fields.iter().for_each(|(_, item)| refs(item, out)),
        _ => {}
    }
}

fn is_instrument(value: &Val) -> bool {
    match value {
        Val::Str(_) => true,
        Val::Call(name, args) => {
            (name == "tool" || name == "doc") && args.len() == 1 && matches!(args[0], Val::Str(_))
        }
        _ => false,
    }
}

fn is_evidence(value: &Val) -> bool {
    match value {
        Val::Id(name) => ["user", "model", "observation"].contains(&name.as_str()),
        Val::Call(name, args) => {
            (name == "tool" || name == "doc" || name == "code")
                && args.len() == 1
                && matches!(args[0], Val::Str(_))
        }
        _ => false,
    }
}

fn type_ok(ftype: &str, value: &Val) -> bool {
    if matches!(value, Val::Hole) {
        return true;
    }
    match ftype {
        "str" => matches!(value, Val::Str(_)),
        "num" => matches!(value, Val::Num(_)),
        "rec:any" => matches!(value, Val::Rec(_)),
        "attrs" => match value {
            Val::Rec(fields) => fields
                .iter()
                .all(|(_, item)| matches!(item, Val::Str(_) | Val::Num(_) | Val::Id(_))),
            _ => false,
        },
        "instrument" => is_instrument(value),
        "evidence" => is_evidence(value),
        _ if ftype.starts_with("enum:") => match value {
            Val::Id(name) => ftype[5..].split(',').any(|option| option == name),
            _ => false,
        },
        // an element that does not have the element type makes the list a different type
        _ if ftype.starts_with("list:") => match value {
            Val::List(items) => items.iter().all(|item| type_ok(&ftype[5..], item)),
            _ => false,
        },
        _ if ftype.starts_with("ref:") => matches!(value, Val::Id(_)),
        _ => true,
    }
}

fn check_record(decl: &Decl, out: &mut Vec<Diagnostic>) {
    let Some(fields) = spec(&decl.kind) else {
        out.push(diag(
            "E-KIND-UNKNOWN",
            format!("unknown kind {}", decl.kind),
            decl,
        ));
        return;
    };
    let mut seen_fields: HashSet<&str> = HashSet::new();
    for (index, (key, value)) in decl.fields.iter().enumerate() {
        let (line, col, end_line, end_col) = decl.field_span.get(index).copied().unwrap_or((
            decl.line,
            decl.col,
            decl.end_line,
            decl.end_col,
        ));
        if !seen_fields.insert(key.as_str()) {
            // The readers of a declaration take the first occurrence; a second one is a value no
            // command will ever see, so it must not compile as if it were the only one.
            out.push(Diagnostic {
                code: "E-DUP-FIELD",
                message: format!("{} carries {key} twice; only the first is read", decl.name),
                line,
                col,
                end_line,
                end_col,
                file: decl.file.clone(),
                witness: Some(decl.name.clone()),
            });
        }
        match fields.iter().find(|(name, _, _)| name == key) {
            Some((_, ftype, _)) => {
                if matches!(value, Val::Hole) {
                    out.push(Diagnostic {
                        code: "W-HOLE",
                        message: format!(
                            "{}.{key} is a hole: it compiles, but nothing can check it yet",
                            decl.name
                        ),
                        line,
                        col,
                        end_line,
                        end_col,
                        file: decl.file.clone(),
                        witness: Some(decl.name.clone()),
                    });
                }
                if !type_ok(ftype, value) {
                    out.push(Diagnostic {
                        code: "E-TYPE",
                        message: format!("{}.{key}: expected {ftype}", decl.name),
                        line,
                        col,
                        end_line,
                        end_col,
                        file: decl.file.clone(),
                        witness: Some(decl.name.clone()),
                    });
                }
            }
            None => out.push(Diagnostic {
                code: "E-FIELD",
                message: format!("{} has no field {key}", decl.kind),
                line,
                col,
                end_line,
                end_col,
                file: decl.file.clone(),
                witness: Some(decl.name.clone()),
            }),
        }
    }
    for &(name, _, required) in fields {
        if required && !decl.fields.iter().any(|(key, _)| key == name) {
            out.push(diag(
                "E-MISSING",
                format!("{} needs {name}", decl.name),
                decl,
            ));
        }
    }
}

/// Every complaint the document earns, from every pass: no pass stops at the first defect and no
/// pass reads another's findings, so the result is the whole list.  A document that declares
/// nothing earns a diagnostic of its own rather than an empty result.
pub fn check(decls: &[Decl]) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    if decls.is_empty() {
        // an empty document is not a quiet success: it says nothing and can be checked
        // for nothing. Whoever produced it (a person, a model, a truncated stream) the
        // verdict is the same.
        out.push(Diagnostic {
            code: "E-EMPTY-DOCUMENT",
            message: "the document declares nothing, so there is nothing to check".to_string(),
            line: 1,
            col: 1,
            end_line: 1,
            end_col: 1,
            file: String::new(),
            witness: None,
        });
    }
    let mut by_name: HashMap<&str, &str> = HashMap::new();
    for decl in decls {
        if by_name.contains_key(decl.name.as_str()) {
            out.push(diag(
                "E-DUP-ID",
                format!("{} is declared twice", decl.name),
                decl,
            ));
        } else {
            // the first declaration of a name owns it: the duplicate is reported above, and every
            // later reference resolves to the kind declared first
            by_name.insert(decl.name.as_str(), decl.kind.as_str());
        }
    }

    for decl in decls {
        check_record(decl, &mut out);
    }

    check_cycles(decls, &mut out);
    check_collapse(decls, &mut out);
    check_semantics(decls, &mut out);
    check_remaining(decls, &mut out);
    confidence::check_confidence_flow(decls, &mut out);
    collide::check_collide(decls, &mut out);
    evidence::check(decls, &mut out);
    check_tension(decls, &mut out);
    check_layer(decls, &mut out);
    check_refinements(decls, &mut out);
    let result = closure::closure(decls);
    for (name, reason) in &result.conditional {
        let decl = decls.iter().find(|item| &item.name == name);
        if let Some(decl) = decl {
            out.push(diag(
                "W-CONDITIONAL",
                format!("{name} is supported only through an ungrounded premise: {reason}"),
                decl,
            ));
        }
    }

    let mut seen: HashSet<(String, String)> = HashSet::new();
    for decl in decls {
        for (key, value) in &decl.fields {
            if !REF_KEYS.contains(&key.as_str()) {
                continue;
            }
            let want = expected_kind(decl, key);
            let mut found = Vec::new();
            refs(value, &mut found);
            for name in found {
                if LITERALS.contains(&name.as_str()) {
                    continue;
                }
                if let Some(target_kind) = by_name.get(name.as_str()) {
                    if *target_kind == "note" {
                        out.push(diag_ref(
                            "E-NOTE-REF",
                            format!(
                                "{}.{key} leans on the note {name}, which is not a commitment",
                                decl.name
                            ),
                            decl,
                            key,
                            &name,
                        ));
                        continue;
                    }
                    if let Some(expected) = want.as_deref() {
                        if expected != "any" && target_kind != &expected {
                            out.push(diag_ref(
                                "E-KIND",
                                format!(
                                    "{}.{key} expects {expected}, but {name} is {target_kind}",
                                    decl.name
                                ),
                                decl,
                                key,
                                &name,
                            ));
                        }
                    }
                    continue;
                }
                // one report per missing name per declaration: the same absent target named by
                // two fields is one mistake, and a second report would tell the reader nothing new
                if seen.insert((decl.name.clone(), name.clone())) {
                    out.push(diag(
                        "E-UNDEFINED",
                        format!("{}.{key} refers to {name}", decl.name),
                        decl,
                    ));
                }
            }
        }
    }
    check_quotes(decls, &by_name, &mut out);
    out
}

/// A quote is the anchor between a claim and its source: both fields or neither, and the quote
/// must be a line of the source's `text`.
fn check_quotes(decls: &[Decl], by_name: &HashMap<&str, &str>, out: &mut Vec<Diagnostic>) {
    for decl in decls {
        if decl.kind != "fact" && decl.kind != "proposition" {
            continue;
        }
        let source = crate::program::f_id(decl, "source");
        let quote = crate::program::f_str(decl, "quote");
        match (source, quote) {
            (None, None) => {}
            (None, Some(_)) => out.push(diag(
                "E-QUOTE-SOURCE",
                format!("{}.quote needs a source", decl.name),
                decl,
            )),
            (Some(_), None) => out.push(diag(
                "E-QUOTE",
                format!("{}.source needs a quote", decl.name),
                decl,
            )),
            (Some(source), Some(quote)) => {
                if quote.trim().is_empty() {
                    out.push(diag(
                        "E-QUOTE",
                        format!("{}.quote is empty", decl.name),
                        decl,
                    ));
                    continue;
                }
                let Some(kind) = by_name.get(source) else {
                    continue;
                };
                if *kind != "source" {
                    continue;
                }
                let Some(target) = decls.iter().find(|item| item.name == source) else {
                    continue;
                };
                let lines = text_lines(target);
                if lines.iter().any(|line| line.contains(quote)) {
                    continue;
                }
                let (line, col) = field_pos(decl, "quote");
                out.push(Diagnostic {
                    code: "E-QUOTE",
                    message: format!("{}.quote does not occur in {source}", decl.name),
                    line,
                    col,
                    end_line: line,
                    end_col: col + quote.chars().count(),
                    file: decl.file.clone(),
                    witness: Some(decl.name.clone()),
                });
            }
        }
    }
}

/// The lines of a `list:str` field; an absent or mistyped field is an empty list.
fn text_lines(decl: &Decl) -> Vec<String> {
    match decl
        .fields
        .iter()
        .find(|(name, _)| name == "text")
        .map(|(_, value)| value)
    {
        Some(Val::List(items)) => items
            .iter()
            .filter_map(|item| match item {
                Val::Str(text) => Some(text.clone()),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

pub(crate) fn ids(value: &Val) -> Vec<String> {
    let mut out = Vec::new();
    refs(value, &mut out);
    out
}

fn field_ids(decl: &Decl, key: &str) -> Vec<String> {
    decl.fields
        .iter()
        .filter(|(name, _)| name == key)
        .flat_map(|(_, value)| ids(value))
        .collect()
}

fn expected_kind(decl: &Decl, key: &str) -> Option<String> {
    let fields = spec(&decl.kind)?;
    let (_, ftype, _) = fields.iter().find(|(name, _, _)| *name == key)?;
    let kind = ftype
        .strip_prefix("list:ref:")
        .or_else(|| ftype.strip_prefix("ref:"))?;
    Some(kind.to_string())
}

fn diag_ref(code: &'static str, message: String, decl: &Decl, key: &str, name: &str) -> Diagnostic {
    let (line, col, end_col) = decl
        .ref_pos
        .iter()
        .find(|(item, _, _, _)| item == name)
        .map(|(_, line, col, end_col)| (*line, *col, *end_col))
        .unwrap_or_else(|| {
            let (line, col) = field_pos(decl, key);
            (line, col, col + name.chars().count())
        });
    Diagnostic {
        code,
        message,
        line,
        col,
        end_line: line,
        end_col,
        file: decl.file.clone(),
        witness: Some(decl.name.clone()),
    }
}

/// Whether the justification edges close on themselves.  A cycle is an error because every member
/// would be grounded in the next, which leaves the loop as a whole grounded in nothing.
fn check_cycles(decls: &[Decl], out: &mut Vec<Diagnostic>) {
    let names: Vec<String> = decls.iter().map(|decl| decl.name.clone()).collect();
    let mut by_name: std::collections::HashMap<&str, &Decl> = std::collections::HashMap::new();
    for decl in decls {
        by_name.insert(decl.name.as_str(), decl);
    }
    let mut edges: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    for decl in decls {
        let mut targets = Vec::new();
        for (key, value) in &decl.fields {
            // `rival` and `attacks` are opposition edges, not justification edges. Two hypotheses
            // that are each other's rival, or two rules that attack each other, are a live
            // disagreement the grounded extension is designed to leave undecided; calling that a
            // dependency cycle would refuse the normal shape of an argument.
            if REF_KEYS.contains(&key.as_str()) && key != "attacks" && key != "rival" {
                targets.extend(ids(value));
            }
        }
        // only a declared name can close a cycle: an unresolved target belongs to E-UNDEFINED, and
        // following it here would put a name in a cycle path that no declaration backs
        targets.retain(|target| names.contains(target));
        edges.insert(decl.name.clone(), targets);
    }

    fn visit(
        node: &str,
        edges: &std::collections::HashMap<String, Vec<String>>,
        by_name: &std::collections::HashMap<&str, &Decl>,
        color: &mut std::collections::HashMap<String, u8>,
        stack: &mut Vec<String>,
        out: &mut Vec<Diagnostic>,
    ) {
        color.insert(node.to_string(), 1);
        stack.push(node.to_string());
        if let Some(targets) = edges.get(node) {
            for target in targets.clone() {
                match color.get(&target).copied().unwrap_or(0) {
                    0 => visit(&target, edges, by_name, color, stack, out),
                    1 => {
                        let position = stack.iter().position(|item| item == &target).unwrap_or(0);
                        let mut cycle = stack[position..].to_vec();
                        cycle.push(target.clone());
                        if let Some(decl) = by_name.get(node) {
                            out.push(diag(
                                "E-CYCLE",
                                format!("dependency cycle: {}", cycle.join(" -> ")),
                                decl,
                            ));
                        }
                    }
                    _ => {}
                }
            }
        }
        stack.pop();
        color.insert(node.to_string(), 2);
    }

    let mut color = std::collections::HashMap::new();
    for decl in decls {
        if color.get(&decl.name).copied().unwrap_or(0) == 0 {
            let mut stack = Vec::new();
            visit(&decl.name, &edges, &by_name, &mut color, &mut stack, out);
        }
    }
}

/// Whether two concepts name the same extent.  A concept is its positives and negatives, compared
/// here as sets, so two with the same pair admit and exclude exactly the same things and differ
/// only in name: one concept with two spellings.
fn check_collapse(decls: &[Decl], out: &mut Vec<Diagnostic>) {
    let mut seen: std::collections::HashMap<(Vec<String>, Vec<String>), String> =
        std::collections::HashMap::new();
    for decl in decls.iter().filter(|decl| decl.kind == "concept") {
        let mut positives = field_ids(decl, "grounded_by");
        let mut negatives = field_ids(decl, "against");
        positives.sort();
        positives.dedup();
        negatives.sort();
        negatives.dedup();
        // a concept with no positive example asserts nothing, so it cannot collide
        if positives.is_empty() {
            continue;
        }
        match seen.get(&(positives.clone(), negatives.clone())) {
            Some(first) => out.push(diag(
                "E-COLLAPSE",
                format!(
                    "concepts {first} and {} have the same extent, so they are one concept with two names",
                    decl.name
                ),
                decl,
            )),
            None => {
                seen.insert((positives, negatives), decl.name.clone());
            }
        }
    }
}

mod semantic;

use self::semantic::{
    check_layer, check_refinements, check_remaining, check_semantics, check_tension, field_pos,
};

pub(crate) use self::semantic::{id_field, list_field, sem, string_field};
