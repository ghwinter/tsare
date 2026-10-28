//! Evidence and binding: what a claim rests on, and what it ranges over.
//!
//! Six checks that share one question.  A claim can be well formed and still rest on nothing:
//! a citation whose law requires a field the claim does not carry, a verdict from a model, a
//! bound no rule reads, a hash with nothing behind it, a universal with an unenumerable domain,
//! a reference to something already withdrawn.  None of these is a syntax error and none is a
//! disagreement about facts; they are all failures of the accounting.

use super::{id_field, ids, list_field, sem, string_field, Diagnostic, REF_KEYS};
use crate::closure::field_ids;
use crate::syntax::{Decl, Val};

/// The six checks, in the order a reader would ask the questions.
pub(crate) fn check(decls: &[Decl], out: &mut Vec<Diagnostic>) {
    check_requirements(decls, out);
    check_verdicts(decls, out);
    check_dead_values(decls, out);
    check_artifacts(decls, out);
    check_bindings(decls, out);
    check_bare_numbers(decls, out);
    check_retirement(decls, out);
}
/// A law may name the fields a declaration must carry in order to cite it (`requires`).  Without
/// that, citing a principle is free: a reference is resolved for existence only, so a claim
/// about button colour can stand on a law about instruments and nothing notices.  A requirement
/// makes the citation cost the field the law is about.  It still cannot tell whether the value
/// is the right one -- only that the claim paid for the name it used.
fn check_requirements(decls: &[Decl], out: &mut Vec<Diagnostic>) {
    let laws: std::collections::HashMap<&str, &Decl> = decls
        .iter()
        .filter(|decl| decl.kind == "law")
        .map(|decl| (decl.name.as_str(), decl))
        .collect();
    for decl in decls.iter().filter(|decl| decl.kind != "law") {
        for key in ["from", "basis", "checked_by"] {
            for cited in field_ids(decl, key) {
                let Some(law) = laws.get(cited.as_str()) else {
                    continue;
                };
                for needed in string_items(law, "requires") {
                    if carries(decl, &needed) {
                        continue;
                    }
                    sem(
                        decl,
                        "W-UNSATISFIED",
                        format!(
                            "{} cites law {cited}, which requires {needed}, and declares no {needed}",
                            decl.name
                        ),
                        Some(key),
                        out,
                    );
                }
            }
        }
    }
}

fn string_items(decl: &Decl, key: &str) -> Vec<String> {
    let Some(value) = decl
        .fields
        .iter()
        .find(|(name, _)| name == key)
        .map(|(_, value)| value)
    else {
        return Vec::new();
    };
    match value {
        Val::List(items) => items
            .iter()
            .filter_map(|item| match item {
                Val::Str(text) => Some(text.clone()),
                Val::Id(name) => Some(name.clone()),
                _ => None,
            })
            .collect(),
        Val::Str(text) => vec![text.clone()],
        _ => Vec::new(),
    }
}

/// A field is carried when it is present, is not a hole, and is not the empty list.
pub(crate) fn carries(decl: &Decl, key: &str) -> bool {
    let Some(value) = decl
        .fields
        .iter()
        .find(|(name, _)| name == key)
        .map(|(_, value)| value)
    else {
        return false;
    };
    !matches!(value, Val::Hole) && !matches!(value, Val::List(items) if items.is_empty())
}

/// A verdict is a claim about how strong the evidence is.  When the only witness is the model that
/// made the claim, the verdict asserts a grade nothing reached: a model assessing its own guess is
/// not an observation.  The fact stays legal; what it cannot do is settle a hypothesis.
fn check_verdicts(decls: &[Decl], out: &mut Vec<Diagnostic>) {
    let by_name: std::collections::HashMap<&str, &Decl> = decls
        .iter()
        .map(|decl| (decl.name.as_str(), decl))
        .collect();
    for decl in decls.iter().filter(|decl| decl.kind == "hypothesis") {
        let Some(verdict) = id_field(decl, "verdict") else {
            continue;
        };
        if verdict != "supported" && verdict != "refuted" {
            continue;
        }
        let Some(witness) =
            id_field(decl, "verdict_by").and_then(|name| by_name.get(name).copied())
        else {
            continue;
        };
        if crate::program::grade(witness) >= crate::program::Grade::Confirmed {
            continue;
        }
        sem(
            decl,
            "E-SELF-VERDICT",
            format!(
                "hypothesis {} is marked {verdict}, but {} is {}: a witness graded below confirmed cannot settle it",
                decl.name,
                witness.name,
                crate::program::grade(witness).label()
            ),
            Some("verdict"),
            out,
        );
    }
}

/// A bound that no rule reads cannot change a decision.  `impact` reports facts no conclusion
/// depends on; this is the same defect for the numbers a decision would have had to use.
fn check_dead_values(decls: &[Decl], out: &mut Vec<Diagnostic>) {
    let referenced: std::collections::HashSet<String> = decls
        .iter()
        .flat_map(|decl| decl.fields.iter())
        .filter(|(key, _)| REF_KEYS.contains(&key.as_str()))
        .flat_map(|(_, value)| ids(value))
        .collect();
    for decl in decls.iter().filter(|decl| decl.kind == "value") {
        let decides = decl
            .fields
            .iter()
            .any(|(key, _)| key == "threshold" || key == "priority");
        if !decides || referenced.contains(&decl.name) {
            continue;
        }
        sem(
            decl,
            "W-UNREAD",
            format!(
                "value {} declares a bound that no rule reads, so it cannot change a decision",
                decl.name
            ),
            Some("threshold"),
            out,
        );
    }
}

/// `out` is the hash of what the instrument produced.  Without it, `evidence: tool("bench.sh")`
/// records that something ran, not what came out -- and a run that exits 0 while leaving a
/// placeholder behind passes every other check in this file.  A path is read from the working
/// directory; an unreadable artifact is unverified, never assumed good.
fn check_artifacts(decls: &[Decl], out: &mut Vec<Diagnostic>) {
    for decl in decls.iter().filter(|decl| decl.kind == "fact") {
        let Some(expected) = string_field(decl, "out") else {
            continue;
        };
        let Some(path) = string_field(decl, "artifact") else {
            sem(
                decl,
                "W-OUT-UNVERIFIED",
                format!(
                    "fact {} declares out but names no artifact, so the hash is compared with nothing",
                    decl.name
                ),
                Some("out"),
                out,
            );
            continue;
        };
        let Ok(bytes) = std::fs::read(path) else {
            sem(
                decl,
                "W-OUT-UNVERIFIED",
                format!(
                    "fact {} names artifact {path}, which cannot be read from here",
                    decl.name
                ),
                Some("artifact"),
                out,
            );
            continue;
        };
        let actual = crate::hash::hex(&bytes);
        if !actual.eq_ignore_ascii_case(expected.trim()) {
            sem(
                decl,
                "E-OUT-MISMATCH",
                format!(
                    "fact {} records {expected}, but {path} hashes to {actual}: the artifact is not what the document says it is",
                    decl.name
                ),
                Some("out"),
                out,
            );
        }
    }
}

/// A universal claim is bound only when its domain is something the engine can enumerate: a
/// concept's extent is its positives, a closed partition is its elements, a layer is the
/// declarations it holds.  Anything else is a free variable wearing a name.
fn check_bindings(decls: &[Decl], out: &mut Vec<Diagnostic>) {
    let by_name: std::collections::HashMap<&str, &Decl> = decls
        .iter()
        .map(|decl| (decl.name.as_str(), decl))
        .collect();
    for decl in decls
        .iter()
        .filter(|decl| decl.kind == "fact" || decl.kind == "proposition")
    {
        for domain in list_field(decl, "over") {
            let Some(target) = by_name.get(domain.as_str()) else {
                continue;
            };
            let unbounded = match target.kind.as_str() {
                "concept" if list_field(target, "grounded_by").is_empty() => {
                    Some("the concept has no positive example")
                }
                "partition" if id_field(target, "closure") != Some("closed") => {
                    Some("the partition is not closed")
                }
                "layer"
                    if !decls
                        .iter()
                        .any(|other| id_field(other, "layer") == Some(target.name.as_str())) =>
                {
                    Some("the layer holds no declaration")
                }
                "concept" | "partition" | "layer" => None,
                _ => Some("only a concept, a closed partition or a layer can be a domain"),
            };
            let Some(reason) = unbounded else {
                continue;
            };
            sem(
                decl,
                "E-SCOPE-UNBOUNDED",
                format!(
                    "{} ranges over {domain}, whose extent cannot be enumerated: {reason}",
                    decl.name
                ),
                Some("over"),
                out,
            );
        }
    }
}

/// The units a quantity is written with.  A digit alone is not a quantity: `A1`, `0.4.3` and
/// `F_1` are names, and a rule that fires on them is a rule about typing, not about measuring.
const UNITS: &[&str] = &[
    "%", "×", "倍", "ms", "ns", "s", "x", "fps", "Hz", "kHz", "MB", "GB", "times", "秒", "毫秒",
];

/// Whether the text states a quantity: a digit run followed by a unit, on a word boundary.
fn states_a_quantity(text: &str) -> bool {
    let chars: Vec<char> = text.chars().collect();
    let mut index = 0;
    while index < chars.len() {
        if !chars[index].is_ascii_digit() {
            index += 1;
            continue;
        }
        let mut after = index;
        while after < chars.len() && (chars[after].is_ascii_digit() || chars[after] == '.') {
            after += 1;
        }
        while after < chars.len() && chars[after] == ' ' {
            after += 1;
        }
        for unit in UNITS {
            let width = unit.chars().count();
            if after + width > chars.len() {
                continue;
            }
            let candidate: String = chars[after..after + width].iter().collect();
            if candidate != *unit {
                continue;
            }
            let next = chars.get(after + width);
            let boundary = next
                .map(|ch| !(ch.is_alphanumeric() || *ch == '_'))
                .unwrap_or(true);
            if boundary {
                return true;
            }
        }
        index = after.max(index + 1);
    }
    false
}

/// A number in a conclusion needs an instrument, or it is an opinion wearing a number's clothes.
///
/// The criterion is structural -- does this sentence cite a `value` that names the instrument that
/// produced it -- and only the question "what counts as a number" is lexical.  Measured on 76
/// documents: firing on any digit gave 6 warnings of which 4 were identifiers (`A1`, `B1`); firing
/// on a digit with a unit gave 2, both real.
///
/// shortcut: only unit-anchored quantities are caught, so `97 tests` and `第 3 条` are not; upgrade
/// when there is a criterion for counting and for ordinals.
fn check_bare_numbers(decls: &[Decl], out: &mut Vec<Diagnostic>) {
    let measured: std::collections::HashSet<&str> = decls
        .iter()
        .filter(|decl| decl.kind == "value" && carries(decl, "measured_by"))
        .map(|decl| decl.name.as_str())
        .collect();
    for decl in decls
        .iter()
        .filter(|decl| decl.kind == "proposition" || decl.kind == "warrant")
    {
        let key = if decl.kind == "proposition" {
            "statement"
        } else {
            "claim"
        };
        let Some(text) = string_field(decl, key) else {
            continue;
        };
        if !states_a_quantity(text) {
            continue;
        }
        let cited: std::collections::HashSet<String> = field_ids(decl, "from")
            .into_iter()
            .chain(field_ids(decl, "basis"))
            .collect();
        if cited.iter().any(|name| measured.contains(name.as_str())) {
            continue;
        }
        sem(
            decl,
            "W-BARE-NUMBER",
            format!(
                "{} states a number and cites no measured value: nothing here says what produced it",
                decl.name
            ),
            Some(key),
            out,
        );
    }
}

/// A reference to a retired declaration resolves -- nothing was deleted -- but the ground behind it
/// was withdrawn.  Saying nothing would let a document keep leaning on what it already replaced.
fn check_retirement(decls: &[Decl], out: &mut Vec<Diagnostic>) {
    // a withdrawal names something; a withdrawal that names nothing withdraws nothing, and a typo
    // would otherwise be silent because `supersedes` is an edge, not a dependency
    let declared: std::collections::HashSet<&str> =
        decls.iter().map(|decl| decl.name.as_str()).collect();
    for decl in decls {
        for name in list_field(decl, "supersedes") {
            if name == decl.name {
                sem(
                    decl,
                    "E-SUPERSEDE-UNDEFINED",
                    format!(
                        "{} supersedes itself: a withdrawal has to leave something standing",
                        decl.name
                    ),
                    Some("supersedes"),
                    out,
                );
                continue;
            }
            if declared.contains(name.as_str()) {
                continue;
            }
            sem(
                decl,
                "E-SUPERSEDE-UNDEFINED",
                format!(
                    "{} supersedes {name}, which is not declared: the withdrawal names nothing",
                    decl.name
                ),
                Some("supersedes"),
                out,
            );
        }
    }
    // A retirement is a decision, and every other decision in this language has to say why.  A
    // warning rather than an error: an old document that withdrew something is still readable, but
    // a reader cannot tell a considered withdrawal from a quiet one.
    for decl in decls {
        if list_field(decl, "supersedes").is_empty() || !list_field(decl, "because").is_empty() {
            continue;
        }
        sem(
            decl,
            "W-RETIRED-UNBASED",
            format!(
                "{} withdraws a declaration and gives no reason: `because:` names what makes the withdrawal right",
                decl.name
            ),
            Some("supersedes"),
            out,
        );
    }
    let gone = crate::closure::retired(decls);
    if gone.is_empty() {
        return;
    }
    for decl in decls {
        // one warning per name, at the first field that cites it: a declaration that cites the
        // same withdrawn ground twice is still one mistake
        let mut reported: std::collections::HashSet<(String, String)> =
            std::collections::HashSet::new();
        for (key, value) in &decl.fields {
            if !REF_KEYS.contains(&key.as_str()) {
                continue;
            }
            for name in ids(value) {
                if !gone.contains(&name) || name == decl.name {
                    continue;
                }
                if !reported.insert((decl.name.clone(), name.clone())) {
                    continue;
                }
                sem(
                    decl,
                    "W-RETIRED",
                    format!(
                        "{} cites {name}, which is retired: the reference resolves, the ground does not",
                        decl.name
                    ),
                    Some(key),
                    out,
                );
            }
        }
    }
}
