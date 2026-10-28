//! Patterns with variables: a concept can require two branches to land on the same node.
//!
//! `paths` compared label sequences, so `premise` + `basis` looked identical whether or not
//! both edges pointed at the same warrant.  A variable can say "the same ?w", which is the
//! smallest thing a conjunctive query adds over a bag of paths.

use std::collections::HashMap;

use crate::impact::dependents;
use crate::program::Program;

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Target {
    Var(String),
    Const(String),
}

pub type Branch = (Vec<String>, Target);
pub type Pattern = Vec<Branch>;

fn edges_of(prog: &Program) -> HashMap<String, Vec<(String, String)>> {
    let mut out = HashMap::new();
    for (source, targets) in dependents(prog).items() {
        out.insert(
            source,
            targets
                .into_iter()
                .filter(|(dst, kind)| prog.has(dst) && kind != "positive" && kind != "negative")
                .map(|(dst, kind)| (kind, dst))
                .collect(),
        );
    }
    out
}

pub fn branches(prog: &Program, root: &str, depth: usize) -> Vec<(Vec<String>, String)> {
    let edges = edges_of(prog);
    let mut found = Vec::new();
    fn walk(
        edges: &HashMap<String, Vec<(String, String)>>,
        node: &str,
        labels: &mut Vec<String>,
        seen: &mut std::collections::HashSet<String>,
        depth: usize,
        found: &mut Vec<(Vec<String>, String)>,
    ) {
        if !labels.is_empty() {
            found.push((labels.clone(), node.to_string()));
        }
        if labels.len() >= depth {
            return;
        }
        for (kind, dst) in edges.get(node).cloned().unwrap_or_default() {
            if seen.contains(&dst) {
                continue;
            }
            labels.push(kind);
            seen.insert(dst.clone());
            walk(edges, &dst, labels, seen, depth, found);
            seen.remove(&dst);
            labels.pop();
        }
    }
    let mut seen = std::collections::HashSet::new();
    seen.insert(root.to_string());
    walk(&edges, root, &mut Vec::new(), &mut seen, depth, &mut found);
    found
}

pub fn walk_labels(prog: &Program, root: &str, labels: &[String]) -> Vec<String> {
    let edges = edges_of(prog);
    let mut current: Vec<String> = vec![root.to_string()];
    for label in labels {
        let mut next: Vec<String> = Vec::new();
        for node in &current {
            for (kind, dst) in edges.get(node).cloned().unwrap_or_default() {
                if &kind == label && !next.contains(&dst) {
                    next.push(dst);
                }
            }
        }
        current = next;
        if current.is_empty() {
            break;
        }
    }
    let mut sorted = current;
    sorted.sort();
    sorted
}

pub fn render(pattern: &Pattern) -> String {
    let parts: Vec<String> = pattern
        .iter()
        .map(|(labels, target)| {
            let path = format!(
                " -{}-> {}",
                labels.join("-> -"),
                match target {
                    Target::Var(name) | Target::Const(name) => name.clone(),
                }
            );
            format!("?r{path}")
        })
        .collect();
    parts.join("  and  ")
}

pub fn matches(prog: &Program, root: &str, pattern: &Pattern) -> bool {
    let end_sets: Vec<Vec<String>> = pattern
        .iter()
        .map(|(labels, _)| walk_labels(prog, root, labels))
        .collect();
    if end_sets.iter().any(|ends| ends.is_empty()) {
        return false;
    }
    // enumerate assignments for the (at most two) branches
    fn product(
        sets: &[Vec<String>],
        index: usize,
        choice: &mut Vec<String>,
        pattern: &Pattern,
        out: &mut bool,
    ) {
        if *out {
            return;
        }
        if index == sets.len() {
            let mut binding: HashMap<String, String> = HashMap::new();
            for ((_, target), node) in pattern.iter().zip(choice.iter()) {
                match target {
                    Target::Const(fixed) => {
                        if node != fixed {
                            return;
                        }
                    }
                    Target::Var(name) => match binding.get(name) {
                        Some(bound) if bound != node => return,
                        _ => {
                            binding.insert(name.clone(), node.clone());
                        }
                    },
                }
            }
            *out = true;
            return;
        }
        for node in &sets[index] {
            choice.push(node.clone());
            product(sets, index + 1, choice, pattern, out);
            choice.pop();
        }
    }
    let mut found = false;
    product(&end_sets, 0, &mut Vec::new(), pattern, &mut found);
    found
}

pub fn candidates(prog: &Program, positive: &str) -> Vec<Pattern> {
    let observed = branches(prog, positive, 2);
    let mut patterns: Vec<(String, Pattern)> = Vec::new();
    let mut index: HashMap<String, usize> = HashMap::new();
    let add = |pattern: Pattern,
               patterns: &mut Vec<(String, Pattern)>,
               index: &mut HashMap<String, usize>| {
        let key = render(&pattern);
        match index.get(&key) {
            Some(position) => patterns[*position].1 = pattern,
            None => {
                index.insert(key.clone(), patterns.len());
                patterns.push((key, pattern));
            }
        }
    };

    for (labels, node) in &observed {
        add(
            vec![(labels.clone(), Target::Var("?v".to_string()))],
            &mut patterns,
            &mut index,
        );
        add(
            vec![(labels.clone(), Target::Const(node.clone()))],
            &mut patterns,
            &mut index,
        );
    }
    for (i, (labels_a, node_a)) in observed.iter().enumerate() {
        for (labels_b, node_b) in observed.iter().skip(i + 1) {
            if labels_a == labels_b {
                continue;
            }
            let shared = node_a == node_b;
            let variants: [(Target, Target); 4] = [
                (
                    Target::Var("?w".to_string()),
                    Target::Var(if shared {
                        "?w".to_string()
                    } else {
                        "?u".to_string()
                    }),
                ),
                (Target::Const(node_a.clone()), Target::Const(node_b.clone())),
                (Target::Var("?w".to_string()), Target::Const(node_b.clone())),
                (Target::Const(node_a.clone()), Target::Var("?w".to_string())),
            ];
            for (spec_a, spec_b) in variants {
                add(
                    vec![(labels_a.clone(), spec_a), (labels_b.clone(), spec_b)],
                    &mut patterns,
                    &mut index,
                );
            }
        }
    }
    patterns.into_iter().map(|(_, pattern)| pattern).collect()
}

pub fn learn(prog: &Program, positives: &[String], negatives: &[String]) -> Option<Pattern> {
    if positives.is_empty() || negatives.is_empty() {
        return None;
    }
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut ranked: Vec<(usize, usize, String, Pattern)> = Vec::new();
    for positive in positives {
        for pattern in candidates(prog, positive) {
            let key = render(&pattern);
            if !seen.insert(key.clone()) {
                continue;
            }
            if !positives.iter().all(|name| matches(prog, name, &pattern)) {
                continue;
            }
            if negatives.iter().any(|name| matches(prog, name, &pattern)) {
                continue;
            }
            let consts = pattern
                .iter()
                .filter(|(_, target)| matches!(target, Target::Const(_)))
                .count();
            ranked.push((pattern.len(), consts, key, pattern));
        }
    }
    ranked.sort_by(|left, right| (left.0, left.1, &left.2).cmp(&(right.0, right.1, &right.2)));
    ranked.into_iter().next().map(|(_, _, _, pattern)| pattern)
}
