//! What a form implies and the document has not declared.
//!
//! A class of declarations that share every position except a few is an equation with those
//! positions left unfilled: `fact NAME { claim: ?, evidence: ? }`. A reader can say which concept
//! would fill them; the checker can say the narrower thing, and it is worth saying: no concept,
//! partition or question in this document names that dimension, so the difference between these
//! declarations exists in the data and not in the vocabulary.
//!
//! This is advice, never a diagnostic: a class of facts that differ only in evidence may be
//! exactly what the author meant. The code names the shape of the gap and leaves the judgement.

use std::collections::BTreeSet;

use crate::program::{f_ids, Program};
use crate::shape::shapes_at;
use crate::syntax::Val;

/// A class that shares a form but leaves positions free.
pub struct Hint {
    pub kind: String,
    pub members: Vec<String>,
    /// The free positions that look like a dimension, with how many values each takes.
    pub free: Vec<(String, usize)>,
    /// The layers the members declare, when they differ: the class crosses a boundary.
    pub layers: Vec<String>,
    /// True when nothing in the document separates the members already.
    pub unnamed: bool,
    /// The equation with its positions unfilled.
    pub template: String,
}

/// The positions a concept can name.  Everything else in a declaration is either prose (it varies
/// in every class by construction) or a reference to a declaration, which is identity rather than
/// a dimension.  An allow-list, because a new field should be added here deliberately: the
/// analysis is only useful if it stays quiet about the fields that cannot mean anything.
const DIMENSIONS: &[&str] = &[
    "evidence",
    "confidence",
    "layer",
    "tag",
    "closure",
    "status",
    "by",
    "unit",
    "direction",
];

/// A position with two to eight distinct values is a dimension a person could name; more than that
/// is a free parameter, and this analysis would only be guessing.
const MIN_VALUES: usize = 2;
const MAX_VALUES: usize = 8;

pub fn hints(program: &Program) -> Vec<Hint> {
    let mut out: Vec<Hint> = Vec::new();
    for group in shapes_at(program, 0) {
        if group.members.len() < 2 {
            continue;
        }
        let free: Vec<(String, usize)> = group
            .variables
            .iter()
            .filter(|(field, values)| {
                DIMENSIONS.contains(&field.as_str())
                    && values.len() >= MIN_VALUES
                    && values.len() <= MAX_VALUES
            })
            .map(|(field, values)| (field.clone(), values.len()))
            .collect();
        if free.is_empty() {
            continue;
        }
        let members: BTreeSet<&str> = group.members.iter().map(String::as_str).collect();
        let mut layers: Vec<String> = Vec::new();
        for name in &group.members {
            let Some(decl) = program.get(name) else {
                continue;
            };
            let Some((_, Val::Id(layer))) = decl.fields.iter().find(|(key, _)| key == "layer")
            else {
                continue;
            };
            if !layers.contains(layer) {
                layers.push(layer.clone());
            }
        }
        let equation = template(&group.kind, &free);
        out.push(Hint {
            kind: group.kind.clone(),
            members: group.members.clone(),
            free,
            layers,
            unnamed: !separated(program, &members),
            template: equation,
        });
    }
    out.sort_by(|left, right| {
        right
            .members
            .len()
            .cmp(&left.members.len())
            .then_with(|| left.kind.cmp(&right.kind))
    });
    out
}

/// Whether any declared concept, partition or question already speaks about these members. One
/// member is enough: the vocabulary has noticed the class.
fn separated(program: &Program, members: &BTreeSet<&str>) -> bool {
    let speaks =
        |names: Vec<String>| -> bool { names.iter().any(|name| members.contains(name.as_str())) };
    for concept in program.of_kind("concept") {
        let mut names = f_ids(concept, "grounded_by");
        names.extend(f_ids(concept, "against"));
        if speaks(names) {
            return true;
        }
    }
    for partition in program.of_kind("partition") {
        if speaks(f_ids(partition, "elements")) {
            return true;
        }
    }
    for question in program.of_kind("question") {
        if speaks(f_ids(question, "separates")) {
            return true;
        }
    }
    false
}

/// `value NAME { direction: ?, unit: ?, ... }`: the equation with its free positions open and
/// everything else left implicit.
fn template(kind: &str, free: &[(String, usize)]) -> String {
    let parts: Vec<String> = free
        .iter()
        .map(|(field, _)| format!("{field}: ?"))
        .collect();
    format!("{kind} NAME {{ {}, ... }}", parts.join(", "))
}

pub fn text(hints: &[Hint]) -> String {
    if hints.is_empty() {
        return "no form leaves a position a concept could name\n".to_string();
    }
    let mut out = format!(
        "{} form(s) whose free positions the vocabulary does not name\n\n",
        hints.len()
    );
    for hint in hints {
        let free: Vec<String> = hint
            .free
            .iter()
            .map(|(field, count)| format!("{field} ({count})"))
            .collect();
        out.push_str(&format!(
            "{} x{}  free: {}\n",
            hint.kind,
            hint.members.len(),
            free.join(", ")
        ));
        if hint.layers.len() > 1 {
            out.push_str(&format!("  crosses layers: {}\n", hint.layers.join(" -> ")));
        }
        out.push_str(&format!("  members: {}\n", hint.members.join(", ")));
        if hint.unnamed {
            out.push_str("  no concept, partition or question separates these members\n");
            out.push_str(&format!("  fill: {}\n", hint.template));
        } else {
            out.push_str("  already named by a concept, partition or question\n");
        }
        out.push('\n');
    }
    out
}
