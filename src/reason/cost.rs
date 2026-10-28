//! Decompression cost: how much external work a document demands before it can be checked.
//!
//! Compression is nearly free; the bottleneck is expanding a claim back into something another
//! party can verify.  So the unit here is not characters saved, it is **external confirmations**
//! (a person, a tool, an instrument) plus **verification depth** (how many links a claim sits
//! from its evidence).

use crate::impact::support_adjacency;
use crate::program::{obligations, Program};
use std::collections::{HashMap, HashSet};

pub struct Cost {
    pub total: usize,
    pub by_kind: Vec<(String, usize)>,
    pub rows: Vec<(String, Vec<String>)>,
    pub depth: usize,
    pub chain: Vec<String>,
}

/// Longest support chain, and the path that realises it.
pub fn chain(prog: &Program) -> (usize, Vec<String>) {
    let adjacency: HashMap<String, Vec<String>> = support_adjacency(prog).into_iter().collect();
    let mut best: (usize, Vec<String>) = (0, Vec::new());
    for decl in prog.decls {
        let mut stack: Vec<(String, Vec<String>, HashSet<String>)> = Vec::new();
        let mut seen = HashSet::new();
        seen.insert(decl.name.clone());
        stack.push((decl.name.clone(), vec![decl.name.clone()], seen));
        while let Some((node, path, visited)) = stack.pop() {
            if path.len() > best.0 {
                best = (path.len() - 1, path.clone());
            }
            for next in adjacency.get(&node).cloned().unwrap_or_default() {
                if visited.contains(&next) {
                    continue;
                }
                let mut step = path.clone();
                step.push(next.clone());
                let mut seen = visited.clone();
                seen.insert(next.clone());
                stack.push((next, step, seen));
            }
        }
    }
    best
}

pub fn report(prog: &Program) -> Cost {
    let mut rows: Vec<(String, Vec<String>)> = Vec::new();
    let mut by_kind: HashMap<String, usize> = HashMap::new();
    for decl in prog.decls {
        let found = obligations(prog, decl);
        if found.is_empty() {
            continue;
        }
        *by_kind.entry(decl.kind.clone()).or_insert(0) += found.len();
        rows.push((decl.name.clone(), found));
    }
    rows.sort_by_key(|row| std::cmp::Reverse(row.1.len()));
    let mut pairs: Vec<(String, usize)> = by_kind.into_iter().collect();
    pairs.sort();
    let total = pairs.iter().map(|(_, count)| count).sum();
    let (depth, path) = chain(prog);
    Cost {
        total,
        by_kind: pairs,
        rows,
        depth,
        chain: path,
    }
}
