//! The document read as prose: every field as it was written, its ground in the margin.
//!
//! A projection is lossy, and the loss has a shape.  What a sentence stands on -- a reference,
//! an evidence kind, a closure -- is not part of the sentence.  Rendering it as one would
//! produce a paragraph that sounds supported and shows nothing, which is the failure this
//! engine exists to refuse.  So the sentence is quoted and the ground goes beside it, where a
//! reader can see that they are two different things.
//!
//! The words are never rewritten.  A paraphrase of a claim is a new claim.

use std::collections::HashMap;

use crate::build::{self, Obligation};
use crate::check::Diagnostic;
use crate::induce::Learning;
use crate::program::Program;
use crate::syntax::{Decl, Val};

/// The field each kind says out loud.  Everything else -- a ground, a check, a count -- is
/// marginal by nature and cannot be read off the sentence.
fn spoken(kind: &str) -> &'static [&'static str] {
    match kind {
        "fact" => &["claim"],
        "concept" => &["name"],
        "proposition" => &["statement"],
        "hypothesis" => &["predicts"],
        "question" => &["asks", "answers"],
        "blindspot" => &["cannot_express"],
        "warrant" => &["claim"],
        "law" => &["statement"],
        "value" => &["metric"],
        "partition" => &["elements"],
        "note" => &["text"],
        "layer" => &["name"],
        _ => &[],
    }
}

/// `nested` asks for the source spelling: a string inside a call or a list is a literal, and a
/// projection that drops its quotes turns `code("src/specs.rs")` into an identifier.
fn value_text(value: &Val, nested: bool) -> String {
    match value {
        Val::Id(name) => name.clone(),
        Val::Str(text) => {
            if nested {
                format!("\"{text}\"")
            } else {
                text.clone()
            }
        }
        Val::Num(number) => number.clone(),
        Val::List(items) => items
            .iter()
            .map(|item| value_text(item, nested))
            .collect::<Vec<_>>()
            .join(", "),
        Val::Call(name, arguments) => format!(
            "{name}({})",
            arguments
                .iter()
                .map(|item| value_text(item, true))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Val::Rec(pairs) => pairs
            .iter()
            .map(|(key, item)| format!("{key}: {}", value_text(item, true)))
            .collect::<Vec<_>>()
            .join("; "),
        Val::Hole => "_".to_string(),
    }
}

fn field<'a>(decl: &'a Decl, key: &str) -> Option<&'a Val> {
    decl.fields
        .iter()
        .find(|(name, _)| name == key)
        .map(|(_, value)| value)
}

fn block(decl: &Decl, obligation: &Obligation, diagnostics: &[Diagnostic]) -> String {
    let mut out = format!("### {} -- {}\n\n", decl.name, decl.kind);
    let said = spoken(&decl.kind);
    for key in said {
        let Some(value) = field(decl, key) else {
            continue;
        };
        if *key == "answers" {
            if let Val::List(items) = value {
                out.push('\n');
                for (index, item) in items.iter().enumerate() {
                    out.push_str(&format!("{}. {}\n", index + 1, value_text(item, false)));
                }
                continue;
            }
        }
        out.push_str(&format!("> {}\n", value_text(value, false)));
    }
    out.push('\n');

    let mut note: Vec<String> = Vec::new();
    for (key, value) in &decl.fields {
        // the group heading already names the layer, and a repeated field is not a margin
        if said.contains(&key.as_str()) || key == "layer" {
            continue;
        }
        note.push(format!("{key}: {}", value_text(value, true)));
    }
    // a ready obligation owes nothing, so its note is not an obligation
    if obligation.state == "ready" {
        note.push(format!(
            "obligation: {} ({})",
            obligation.action, obligation.state
        ));
    } else {
        note.push(format!(
            "obligation: {} ({}), {}",
            obligation.action, obligation.state, obligation.note
        ));
    }
    out.push_str(&format!("  {}\n", note.join(" | ")));

    for item in diagnostics {
        if item.line >= decl.line && item.line <= decl.end_line {
            out.push_str(&format!("  ! {}: {}\n", item.code, item.message));
        }
    }
    out
}

/// The projection.  Declarations are grouped by the layer that admits them; a document with no
/// layers is projected in the order it was written.  A diagnostic appears in the margin of the
/// declaration it points at, because that is where a reader is already looking.
pub fn markdown(program: &Program, diagnostics: &[Diagnostic], file: &str) -> String {
    let learnings: HashMap<String, Learning> = build::learnings_of(program)
        .into_iter()
        .map(|item| (item.concept.clone(), item))
        .collect();

    let mut out = String::new();
    out.push_str(&format!("# Projection\n\n`{file}`\n\n"));
    out.push_str(
        "Quoted text is a field, copied verbatim: a paraphrase of a claim is a new claim.\n",
    );
    out.push_str(
        "The indented note under each quote is the projection's own -- what the sentence\n",
    );
    out.push_str("stands on, who checks it, and what is still owed.\n\n");

    let layers: Vec<&str> = program
        .decls
        .iter()
        .filter(|decl| decl.kind == "layer")
        .map(|decl| decl.name.as_str())
        .collect();
    let mut groups: Vec<(String, Vec<&Decl>)> = Vec::new();
    if layers.is_empty() {
        groups.push((String::new(), program.decls.iter().collect()));
    } else {
        let declared: Vec<&Decl> = program
            .decls
            .iter()
            .filter(|decl| decl.kind == "layer")
            .collect();
        groups.push(("layers".to_string(), declared));
        for layer in &layers {
            let members: Vec<&Decl> = program
                .decls
                .iter()
                .filter(|decl| {
                    decl.kind != "layer"
                        && matches!(field(decl, "layer"), Some(Val::Id(id)) if id.as_str() == *layer)
                })
                .collect();
            if !members.is_empty() {
                groups.push((layer.to_string(), members));
            }
        }
        let unplaced: Vec<&Decl> = program
            .decls
            .iter()
            .filter(|decl| decl.kind != "layer" && field(decl, "layer").is_none())
            .collect();
        if !unplaced.is_empty() {
            groups.push(("unplaced".to_string(), unplaced));
        }
    }

    let (mut total, mut ready) = (0, 0);
    for (heading, members) in &groups {
        if !heading.is_empty() {
            out.push_str(&format!("## {heading}\n\n"));
        }
        for decl in members {
            let obligation = build::obligation(decl, program, &learnings);
            total += 1;
            if obligation.state == "ready" {
                ready += 1;
            }
            out.push_str(&block(decl, &obligation, diagnostics));
            out.push('\n');
        }
    }
    out.push_str(&format!("---\n\n{total} obligation(s), {ready} ready.\n"));
    out
}
