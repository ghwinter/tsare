//! Relational features: what a node looks like as a position in the net, not as a row.
//!
//! An attribute says "strength = 1".  A position says "premise of a warrant that is under
//! attack".  The second is what the analogy literature means by structure, and it is what the
//! attribute learner cannot see.  A feature here is a directed path of edge labels up to
//! length two, e.g. `premise>attacked by`.

use std::collections::HashSet;

use crate::impact::dependents;
use crate::program::Program;

fn adjacency(prog: &Program) -> std::collections::HashMap<String, Vec<(String, String)>> {
    let mut out: std::collections::HashMap<String, Vec<(String, String)>> =
        std::collections::HashMap::new();
    for (source, targets) in dependents(prog).items() {
        let list = out.entry(source).or_default();
        for (dst, kind) in targets {
            if prog.has(&dst)
                && !list
                    .iter()
                    .any(|(name, edge)| name == &dst && edge == &kind)
            {
                list.push((dst, kind));
            }
        }
    }
    out
}

/// Edge-label paths from a node. Membership edges are ignored: they encode the label.
pub fn features(
    prog: &Program,
    node: &str,
    depth: usize,
    exclude: &HashSet<String>,
) -> HashSet<String> {
    let adj = adjacency(prog);
    let mut found: HashSet<String> = HashSet::new();
    let mut frontier: Vec<(String, String)> = vec![(node.to_string(), String::new())];
    for _ in 0..depth {
        let mut next: Vec<(String, String)> = Vec::new();
        for (current, prefix) in &frontier {
            for (target, kind) in adj.get(current).cloned().unwrap_or_default() {
                if kind == "positive" || kind == "negative" || exclude.contains(&target) {
                    continue;
                }
                let label = if prefix.is_empty() {
                    kind.clone()
                } else {
                    format!("{prefix}>{kind}")
                };
                found.insert(label.clone());
                next.push((target, label));
            }
        }
        frontier = next;
    }
    found
}

pub fn learn(
    prog: &Program,
    positives: &[String],
    negatives: &[String],
    max_size: usize,
    exclude: &HashSet<String>,
) -> Option<Vec<String>> {
    if positives.is_empty() || negatives.is_empty() {
        return None;
    }
    let mut feats: std::collections::HashMap<String, HashSet<String>> =
        std::collections::HashMap::new();
    for name in positives.iter().chain(negatives.iter()) {
        feats.insert(name.clone(), features(prog, name, 2, exclude));
    }
    let mut shared: HashSet<String> = feats[&positives[0]].clone();
    for name in positives.iter().skip(1) {
        shared = shared.intersection(&feats[name]).cloned().collect();
    }
    let mut candidates: Vec<String> = shared
        .into_iter()
        .filter(|feature| negatives.iter().any(|name| !feats[name].contains(feature)))
        .collect();
    candidates.sort();

    for size in 1..=max_size {
        if size > candidates.len() {
            continue;
        }
        for indices in crate::induce::combinations(candidates.len(), size) {
            let combo: Vec<String> = indices
                .iter()
                .map(|index| candidates[*index].clone())
                .collect();
            if positives
                .iter()
                .all(|name| combo.iter().all(|feature| feats[name].contains(feature)))
                && negative_ok(&feats, negatives, &combo)
            {
                return Some(combo);
            }
        }
    }
    None
}

fn negative_ok(
    feats: &std::collections::HashMap<String, HashSet<String>>,
    negatives: &[String],
    combo: &[String],
) -> bool {
    !negatives
        .iter()
        .any(|name| combo.iter().all(|feature| feats[name].contains(feature)))
}
