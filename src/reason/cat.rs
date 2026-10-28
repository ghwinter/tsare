//! The pipeline read as a category, and the checks that decide whether it is one.
//!
//! Objects: the signature (declarations).  Arrows: the typed references between them.
//! Two candidate functors are testable right now:
//!
//! ```text
//!     Ext     concept -> its extent (a set of facts);  parent arrow must map to inclusion
//!     Expose  declaration -> the layer that may be seen from outside
//! ```
//!
//! `Ext` is a functor exactly when every child's extent sits inside its parent's: `E-HIERARCHY`.
//! `Expose` is a functor exactly when every cross-layer reference lands on an exposed name:
//! `E-LAYER-LEAK`.
//!
//! Everything else in the pipeline is reported as what it is: partial, non-deterministic, or
//! not compositional at all.

use std::collections::{HashMap, HashSet};

use crate::impact::dependents;
use crate::program::{f_id, f_ids, pos, Program};
use crate::specs::spec;
use crate::syntax::Val;

pub fn objects(prog: &Program) -> Vec<(String, usize)> {
    let mut counts: HashMap<String, usize> = HashMap::new();
    for decl in prog.decls {
        *counts.entry(decl.kind.clone()).or_insert(0) += 1;
    }
    let mut pairs: Vec<(String, usize)> = counts.into_iter().collect();
    pairs.sort();
    pairs
}

pub fn arrows(prog: &Program) -> Vec<(String, usize)> {
    let mut counts: HashMap<String, usize> = HashMap::new();
    for (_, targets) in dependents(prog).items() {
        for (_, kind) in targets {
            *counts.entry(kind).or_insert(0) += 1;
        }
    }
    let mut pairs: Vec<(String, usize)> = counts.into_iter().collect();
    pairs.sort();
    pairs
}

pub fn extent_functor(prog: &Program) -> (usize, usize, Vec<String>) {
    let mut parents: Vec<(&crate::syntax::Decl, String)> = Vec::new();
    for concept in prog.of_kind("concept") {
        if let Some(parent) = f_id(concept, "parent") {
            parents.push((concept, parent.to_string()));
        }
    }
    let mut preserved = 0;
    let mut failures = Vec::new();
    for (child, parent_name) in &parents {
        let Some(parent) = prog.get(parent_name) else {
            continue;
        };
        if parent.kind != "concept" {
            continue;
        }
        let parent_positives: HashSet<String> = pos(parent).into_iter().collect();
        if pos(child)
            .iter()
            .all(|fact| parent_positives.contains(fact))
        {
            preserved += 1;
        } else {
            failures.push(format!(
                "{} -> {parent_name}: extent not included",
                child.name
            ));
        }
    }
    (preserved, parents.len(), failures)
}

fn iter_ids(value: &Val, out: &mut Vec<String>) {
    match value {
        Val::Id(name) => out.push(name.clone()),
        Val::List(items) => items.iter().for_each(|item| iter_ids(item, out)),
        Val::Rec(fields) => fields.iter().for_each(|(_, item)| iter_ids(item, out)),
        Val::Call(_, args) => args.iter().for_each(|item| iter_ids(item, out)),
        _ => {}
    }
}

fn expected_kind(ftype: &str) -> Option<&str> {
    ftype
        .strip_prefix("list:ref:")
        .or_else(|| ftype.strip_prefix("ref:"))
}

pub fn expose_functor(prog: &Program) -> (usize, usize, Vec<String>) {
    let mut exposed: HashMap<String, HashSet<String>> = HashMap::new();
    for layer in prog.of_kind("layer") {
        exposed.insert(
            layer.name.clone(),
            f_ids(layer, "exposes").into_iter().collect(),
        );
    }
    let mut total = 0;
    let mut visible = 0;
    let mut leaks = Vec::new();
    for decl in prog.decls {
        let Some(source) = f_id(decl, "layer") else {
            continue;
        };
        if decl.kind == "layer" {
            continue;
        }
        let Some(fields) = spec(&decl.kind) else {
            continue;
        };
        for (key, ftype, _) in fields {
            if *key == "layer" || expected_kind(ftype).is_none() {
                continue;
            }
            let Some(value) = decl
                .fields
                .iter()
                .find(|(name, _)| name == key)
                .map(|(_, value)| value)
            else {
                continue;
            };
            let mut names = Vec::new();
            iter_ids(value, &mut names);
            for name in names {
                let Some(target) = prog.get(&name) else {
                    continue;
                };
                if target.kind == "layer" {
                    continue;
                }
                let Some(target_layer) = f_id(target, "layer") else {
                    continue;
                };
                if target_layer == source {
                    continue;
                }
                total += 1;
                if exposed
                    .get(target_layer)
                    .map(|names| names.contains(&name))
                    .unwrap_or(false)
                {
                    visible += 1;
                } else {
                    leaks.push(format!(
                        "{} in {source} -> {name} in {target_layer}",
                        decl.name
                    ));
                }
            }
        }
    }
    (visible, total, leaks)
}

/// What one structural mapping did when asked whether it preserves its law.
pub struct Outcome {
    pub kept: usize,
    pub total: usize,
    pub failures: Vec<String>,
}

/// One structural mapping whose preservation is checkable.  This table is the list of laws the
/// document format claims: adding a law is adding an entry, with the diagnostic that already
/// reports it -- not a new branch in the command.
pub struct Functor {
    pub name: &'static str,
    /// What it maps, in one line.
    pub maps: &'static str,
    /// The condition it preserves, in one line.
    pub law: &'static str,
    /// The code reported when the law fails; it must be a code the checker really emits.
    pub code: &'static str,
    pub check: fn(&Program) -> Outcome,
}

fn check_extent(program: &Program) -> Outcome {
    let (kept, total, failures) = extent_functor(program);
    Outcome {
        kept,
        total,
        failures,
    }
}

fn check_expose(program: &Program) -> Outcome {
    let (visible, crossing, leaks) = expose_functor(program);
    Outcome {
        kept: visible,
        total: crossing,
        failures: leaks,
    }
}

pub const FUNCTORS: &[Functor] = &[
    Functor {
        name: "Ext",
        maps: "concept -> its extent, a set of facts",
        law: "a child's extent sits inside its parent's",
        code: "E-HIERARCHY",
        check: check_extent,
    },
    Functor {
        name: "Expose",
        maps: "declaration -> the layer that may be seen from outside",
        law: "a cross-layer reference lands on an exposed name",
        code: "E-LAYER-LEAK",
        check: check_expose,
    },
];
pub struct Stage {
    pub name: &'static str,
    pub label: &'static str,
    pub why: &'static str,
}

/// The pipeline as arrows with an honest label.
pub fn stages() -> Vec<Stage> {
    vec![
        Stage {
            name: "source -> AST",
            label: "total",
            why: "parse() either produces a tree or an E-SYNTAX",
        },
        Stage {
            name: "AST -> signature",
            label: "total",
            why: "check() is a function of the declaration list",
        },
        Stage {
            name: "signature -> diagnostics",
            label: "total",
            why: "same input, same codes",
        },
        Stage {
            name: "signature -> clauses",
            label: "partial",
            why: "a concept without examples yields no clause",
        },
        Stage {
            name: "clauses -> predicates",
            label: "total",
            why: "renderer + exec verification",
        },
        Stage {
            name: "diagnostics -> repair",
            label: "non-deterministic",
            why: "the model may no-op or change meaning",
        },
        Stage {
            name: "signature -> weights",
            label: "total",
            why: "min/max over a finite set",
        },
    ]
}
