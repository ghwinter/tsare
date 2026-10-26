//! Equivalence merging in the style of an e-graph: find nodes that mean the same, union
//! them, saturate, then rewrite every reference to the canonical name.
//!
//! The checker rejects duplicates (`E-COLLAPSE`, `E-DUP-ID`).  A generator, especially an LLM,
//! produces them constantly, and rejecting is the least useful response.  Merging keeps the
//! information and removes the redundancy: the document gets shorter, and nothing else changes.

use std::collections::HashMap;

use crate::closure::{field_ids, id_field};
use crate::fix::render_value;
use crate::program::{f_num, f_str, neg, pos, Program};
use crate::syntax::{Decl, Val};

const KINDS: &[&str] = &[
    "fact",
    "concept",
    "proposition",
    "warrant",
    "value",
    "hypothesis",
    "question",
    "blindspot",
    "law",
];

pub struct UnionFind {
    parent: HashMap<String, String>,
}

impl UnionFind {
    pub fn new(names: &[String]) -> Self {
        UnionFind {
            parent: names
                .iter()
                .map(|name| (name.clone(), name.clone()))
                .collect(),
        }
    }

    pub fn find(&mut self, name: &str) -> String {
        let mut current = name.to_string();
        while self
            .parent
            .get(&current)
            .cloned()
            .unwrap_or_else(|| current.clone())
            != current
        {
            let parent = self
                .parent
                .get(&current)
                .cloned()
                .unwrap_or_else(|| current.clone());
            let grand = self
                .parent
                .get(&parent)
                .cloned()
                .unwrap_or_else(|| parent.clone());
            self.parent.insert(current.clone(), grand);
            current = parent;
        }
        current
    }

    pub fn union(&mut self, a: &str, b: &str) -> bool {
        let left = self.find(a);
        let right = self.find(b);
        if left == right {
            return false;
        }
        // canonical = lexicographically smaller, so the result is deterministic
        let (keep, drop) = if left < right {
            (left, right)
        } else {
            (right, left)
        };
        self.parent.insert(drop, keep);
        true
    }

    pub fn names(&self) -> Vec<String> {
        self.parent.keys().cloned().collect()
    }

    pub fn canonical(&mut self) -> Vec<(String, String)> {
        let names = self.names();
        names
            .into_iter()
            .map(|name| {
                let root = self.find(&name);
                (name, root)
            })
            .collect()
    }
}

fn canon(uf: &mut UnionFind, names: Vec<String>) -> String {
    let mut roots: Vec<String> = names.iter().map(|name| uf.find(name)).collect();
    roots.sort();
    roots.join(",")
}

/// Keys are computed under the current union-find, which is what makes saturation work.
fn key_of(uf: &mut UnionFind, decl: &Decl) -> Option<Vec<String>> {
    match decl.kind.as_str() {
        "fact" => {
            let attrs = match decl
                .fields
                .iter()
                .find(|(key, _)| key == "attrs")
                .map(|(_, value)| value)
            {
                Some(Val::Rec(fields)) => fields
                    .iter()
                    .map(|(key, value)| format!("{key}={}", render_value(value)))
                    .collect::<Vec<_>>()
                    .join("\u{1}"),
                _ => String::new(),
            };
            let evidence = decl
                .fields
                .iter()
                .find(|(key, _)| key == "evidence")
                .map(|(_, value)| render_value(value))
                .unwrap_or_default();
            Some(vec![
                "fact".to_string(),
                f_str(decl, "claim").unwrap_or("").to_string(),
                evidence,
                attrs,
                f_str(decl, "scope").unwrap_or("").to_string(),
            ])
        }
        "concept" => Some(vec![
            "concept".to_string(),
            canon(uf, pos(decl)),
            canon(uf, neg(decl)),
        ]),
        "proposition" => Some(vec![
            "proposition".to_string(),
            f_str(decl, "statement").unwrap_or("").to_string(),
            canon(uf, field_ids(decl, "uses")),
            canon(uf, field_ids(decl, "from")),
        ]),
        "warrant" => {
            let target = id_field(decl, "to")
                .map(|name| uf.find(name))
                .unwrap_or_default();
            Some(vec![
                "warrant".to_string(),
                canon(uf, field_ids(decl, "from")),
                target,
                canon(uf, field_ids(decl, "basis")),
                canon(uf, field_ids(decl, "attacks")),
            ])
        }
        "value" => Some(vec![
            "value".to_string(),
            f_str(decl, "metric").unwrap_or("").to_string(),
            id_field(decl, "direction").unwrap_or("").to_string(),
            f_str(decl, "unit").unwrap_or("").to_string(),
            f_num(decl, "threshold")
                .map(crate::program::repr_float)
                .unwrap_or_default(),
        ]),
        _ => None,
    }
}

pub fn merge_groups(prog: &Program) -> (UnionFind, Vec<Vec<String>>) {
    let names: Vec<String> = prog
        .decls
        .iter()
        .filter(|decl| KINDS.contains(&decl.kind.as_str()))
        .map(|decl| decl.name.clone())
        .collect();
    let mut uf = UnionFind::new(&names);
    loop {
        let mut buckets: Vec<(Vec<String>, Vec<String>)> = Vec::new();
        let mut index: HashMap<Vec<String>, usize> = HashMap::new();
        for decl in prog.decls {
            let Some(key) = key_of(&mut uf, decl) else {
                continue;
            };
            match index.get(&key) {
                Some(position) => buckets[*position].1.push(decl.name.clone()),
                None => {
                    index.insert(key.clone(), buckets.len());
                    buckets.push((key, vec![decl.name.clone()]));
                }
            }
        }
        let mut changed = false;
        for (_, group) in &buckets {
            for other in group.iter().skip(1) {
                changed = uf.union(&group[0], other) || changed;
            }
        }
        if !changed {
            break;
        }
    }
    let mut classes: Vec<(String, Vec<String>)> = Vec::new();
    let mut index: HashMap<String, usize> = HashMap::new();
    for name in &names {
        let root = uf.find(name);
        match index.get(&root) {
            Some(position) => classes[*position].1.push(name.clone()),
            None => {
                index.insert(root.clone(), classes.len());
                classes.push((root, vec![name.clone()]));
            }
        }
    }
    let groups = classes
        .into_iter()
        .map(|(_, mut group)| {
            group.sort();
            group
        })
        .collect();
    (uf, groups)
}

fn rename(value: &Val, mapping: &HashMap<String, String>) -> Val {
    match value {
        Val::Id(name) => Val::Id(mapping.get(name).cloned().unwrap_or_else(|| name.clone())),
        Val::List(items) => Val::List(items.iter().map(|item| rename(item, mapping)).collect()),
        Val::Rec(fields) => Val::Rec(
            fields
                .iter()
                .map(|(key, item)| (key.clone(), rename(item, mapping)))
                .collect(),
        ),
        Val::Call(name, args) => Val::Call(
            name.clone(),
            args.iter().map(|item| rename(item, mapping)).collect(),
        ),
        other => other.clone(),
    }
}

pub fn rewrite(decls: &[Decl], mapping: &HashMap<String, String>) -> Vec<Decl> {
    let mut keep: Vec<Decl> = decls
        .iter()
        .filter(|decl| {
            mapping
                .get(&decl.name)
                .cloned()
                .unwrap_or_else(|| decl.name.clone())
                == decl.name
        })
        .cloned()
        .collect();
    for decl in &mut keep {
        for (_, value) in decl.fields.iter_mut() {
            *value = rename(value, mapping);
        }
    }
    keep
}

pub fn canonical_map(uf: &mut UnionFind) -> HashMap<String, String> {
    uf.canonical().into_iter().collect()
}
