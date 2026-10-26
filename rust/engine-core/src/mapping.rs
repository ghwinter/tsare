//! Structure mapping between two documents, cheaply.
//!
//! Gentner's claim is that analogy aligns relations, not objects, and prefers higher-order
//! relations.  This is the crude version: give each node a bag of edge-label paths up to
//! length two, score a pair by weighted Jaccard (longer paths count double), and only pair
//! nodes of the same kind.  It is not a maximum common subgraph; it is a signature match that
//! can be checked by eye.

use std::collections::{HashMap, HashSet};

use crate::impact::dependents;
use crate::program::Program;

pub type Paths = HashMap<Vec<String>, usize>;

pub struct Signature {
    pub kind: String,
    pub paths: Paths,
}

fn edges_of(
    forward: &HashMap<String, Vec<(String, String)>>,
    reverse: &HashMap<String, Vec<(String, String)>>,
    node: &str,
) -> Vec<(String, String)> {
    let mut out = forward.get(node).cloned().unwrap_or_default();
    out.extend(reverse.get(node).cloned().unwrap_or_default());
    out
}

pub fn signatures(prog: &Program, depth: usize) -> Vec<(String, Signature)> {
    let mut forward: HashMap<String, Vec<(String, String)>> = HashMap::new();
    for (source, targets) in dependents(prog).items() {
        forward.insert(
            source,
            targets
                .into_iter()
                .filter(|(dst, kind)| prog.has(dst) && kind != "positive" && kind != "negative")
                .map(|(dst, kind)| (kind, dst))
                .collect(),
        );
    }
    // incoming edges matter: a proposition has almost no outgoing ones, so without this
    // it could never match anything and every claim would be invisible to mapping
    let mut reverse: HashMap<String, Vec<(String, String)>> = HashMap::new();
    for (source, targets) in &forward {
        for (kind, dst) in targets {
            reverse
                .entry(dst.clone())
                .or_default()
                .push((format!("in:{kind}"), source.clone()));
        }
    }

    fn walk(
        forward: &HashMap<String, Vec<(String, String)>>,
        reverse: &HashMap<String, Vec<(String, String)>>,
        node: &str,
        labels: &mut Vec<String>,
        seen: &mut HashSet<String>,
        depth: usize,
        paths: &mut Paths,
    ) {
        if !labels.is_empty() {
            *paths.entry(labels.clone()).or_insert(0) += 1;
        }
        if labels.len() >= depth {
            return;
        }
        for (kind, dst) in edges_of(forward, reverse, node) {
            if seen.contains(&dst) {
                continue;
            }
            labels.push(kind);
            seen.insert(dst.clone());
            walk(forward, reverse, &dst, labels, seen, depth, paths);
            seen.remove(&dst);
            labels.pop();
        }
    }

    let mut out = Vec::new();
    for decl in prog.decls {
        if decl.kind == "layer" {
            continue;
        }
        let mut paths: Paths = HashMap::new();
        let mut labels: Vec<String> = Vec::new();
        let mut seen: HashSet<String> = HashSet::new();
        seen.insert(decl.name.clone());
        walk(&forward, &reverse, &decl.name, &mut labels, &mut seen, depth, &mut paths);
        out.push((decl.name.clone(), Signature { kind: decl.kind.clone(), paths }));
    }
    out
}

pub struct Pair {
    pub score: f64,
    pub weight: usize,
    pub name_a: String,
    pub name_b: String,
    pub shared: Vec<Vec<String>>,
}

fn compare(left: &Signature, right: &Signature) -> (f64, usize, Vec<Vec<String>>) {
    if left.kind != right.kind {
        return (0.0, 0, Vec::new());
    }
    let common: Vec<Vec<String>> =
        left.paths.keys().filter(|path| right.paths.contains_key(*path)).cloned().collect();
    let mut union: HashSet<Vec<String>> = left.paths.keys().cloned().collect();
    union.extend(right.paths.keys().cloned());
    if union.is_empty() {
        return (0.0, 0, Vec::new());
    }
    let shared_weight: usize = common.iter().map(|path| path.len()).sum();
    let total: usize = union.iter().map(|path| path.len()).sum();
    let mut sorted = common;
    sorted.sort_by(|left, right| {
        right.len().cmp(&left.len()).then_with(|| left.cmp(right))
    });
    (shared_weight as f64 / total as f64, shared_weight, sorted)
}

pub fn align(prog_a: &Program, prog_b: &Program, top: usize) -> Vec<Pair> {
    let sig_a = signatures(prog_a, 2);
    let sig_b = signatures(prog_b, 2);
    let mut rows = Vec::new();
    for (name_a, left) in &sig_a {
        for (name_b, right) in &sig_b {
            if left.kind != right.kind {
                continue;
            }
            let (score, weight, shared) = compare(left, right);
            if score > 0.0 {
                rows.push(Pair {
                    score,
                    weight,
                    name_a: name_a.clone(),
                    name_b: name_b.clone(),
                    shared,
                });
            }
        }
    }
    // a 1.0 on one shared edge is weaker evidence than a 1.0 on four
    rows.sort_by(|left, right| {
        right
            .score
            .partial_cmp(&left.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| right.weight.cmp(&left.weight))
            .then_with(|| left.name_a.cmp(&right.name_a))
            .then_with(|| left.name_b.cmp(&right.name_b))
    });
    rows.truncate(top);
    rows
}
