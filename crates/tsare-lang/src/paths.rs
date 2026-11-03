//! Relational features: what a node looks like as a position in the net, not as a row.
//!
//! An attribute says "strength = 1".  A position says "premise of a warrant that is under
//! attack".  The second is what the analogy literature means by structure, and it is what the
//! attribute learner cannot see.  A feature here is a directed path of edge labels up to
//! length two, e.g. `premise>attacked by`.

use std::collections::{HashMap, HashSet};

use crate::program::{f_id, f_ids, neg, pos, Program};

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
        for indices in crate::util::combinations(candidates.len(), size) {
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

/// Insertion-ordered adjacency: the order is part of the output when a walk is printed.
#[derive(Default)]
pub struct Deps {
    pub keys: Vec<String>,
    pub map: HashMap<String, Vec<(String, String)>>,
}

impl Deps {
    pub fn add(&mut self, from: &str, to: &str, kind: &str) {
        if !self.map.contains_key(from) {
            self.keys.push(from.to_string());
        }
        self.map
            .entry(from.to_string())
            .or_default()
            .push((to.to_string(), kind.to_string()));
    }

    pub fn get(&self, key: &str) -> Option<&Vec<(String, String)>> {
        self.map.get(key)
    }

    pub fn items(&self) -> Vec<(String, Vec<(String, String)>)> {
        self.keys
            .iter()
            .map(|key| (key.clone(), self.map.get(key).cloned().unwrap_or_default()))
            .collect()
    }
}

/// node -> [(dependent, edge kind)]
pub fn dependents(prog: &Program) -> Deps {
    let mut out = Deps::default();
    for concept in prog.of_kind("concept") {
        for fact in pos(concept) {
            out.add(&fact, &concept.name, "positive");
        }
        for fact in neg(concept) {
            out.add(&fact, &concept.name, "negative");
        }
    }
    for warrant in prog.of_kind("warrant") {
        for premise in f_ids(warrant, "from") {
            out.add(&premise, &warrant.name, "premise");
        }
        for basis in f_ids(warrant, "basis") {
            out.add(&basis, &warrant.name, "basis");
        }
        for target in f_ids(warrant, "attacks") {
            out.add(&target, &warrant.name, "attacked by");
        }
        if let Some(conclusion) = f_id(warrant, "to") {
            out.add(&warrant.name, conclusion, "concludes");
        }
    }
    for hypothesis in prog.of_kind("hypothesis") {
        for premise in f_ids(hypothesis, "assumes") {
            out.add(&premise, &hypothesis.name, "assumes");
        }
    }
    for proposition in prog.of_kind("proposition") {
        for premise in f_ids(proposition, "from") {
            out.add(&premise, &proposition.name, "from");
        }
    }
    for question in prog.of_kind("question") {
        for hypothesis in f_ids(question, "separates") {
            out.add(&hypothesis, &question.name, "separates");
        }
    }
    for blindspot in prog.of_kind("blindspot") {
        for question in f_ids(blindspot, "closes_with") {
            out.add(&blindspot.name, &question, "closes with");
        }
    }
    out
}
