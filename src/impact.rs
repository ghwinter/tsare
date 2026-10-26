//! Global analyses: the part of a compiler that is not a per-declaration rule.
//!
//! A rule can only look at one declaration.  Intelligence is what you get when you look at
//! the whole net: what is dead, what everything hangs on, what would still stand if a node
//! were wrong.  These are the cheap analogues of liveness, dominators and dead-code
//! elimination -- on a graph of commitments instead of a control-flow graph.

use std::collections::{HashMap, HashSet};

use crate::program::{f_id, f_ids, neg, pos, Program};
use crate::syntax::Decl;

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
        self.map.entry(from.to_string()).or_default().push((to.to_string(), kind.to_string()));
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

/// conclusion -> [warrants that conclude it], in declaration order.
pub fn supporting_warrants(prog: &Program) -> Vec<(String, Vec<String>)> {
    let mut keys: Vec<String> = Vec::new();
    let mut map: HashMap<String, Vec<String>> = HashMap::new();
    for warrant in prog.of_kind("warrant") {
        if let Some(conclusion) = f_id(warrant, "to") {
            if !map.contains_key(conclusion) {
                keys.push(conclusion.to_string());
            }
            map.entry(conclusion.to_string()).or_default().push(warrant.name.clone());
        }
    }
    keys.into_iter().map(|key| (key.clone(), map.remove(&key).unwrap_or_default())).collect()
}

pub fn adjacency(prog: &Program) -> HashMap<String, HashSet<String>> {
    let mut adj: HashMap<String, HashSet<String>> = HashMap::new();
    for (source, targets) in dependents(prog).items() {
        for (dst, _) in targets {
            adj.entry(source.clone()).or_default().insert(dst.clone());
            adj.entry(dst).or_default().insert(source.clone());
        }
    }
    adj
}

/// How many claims sit within two hops of this decision.
pub fn unlocks(prog: &Program, node: &str, depth: usize) -> usize {
    let adj = adjacency(prog);
    let claims = ["proposition", "hypothesis", "question"];
    let mut seen: HashSet<String> = HashSet::new();
    seen.insert(node.to_string());
    let mut frontier: HashSet<String> = seen.clone();
    for _ in 0..depth {
        let mut next: HashSet<String> = HashSet::new();
        for name in &frontier {
            for other in adj.get(name).cloned().unwrap_or_default() {
                if seen.insert(other.clone()) {
                    next.insert(other);
                }
            }
        }
        frontier = next;
    }
    seen.iter()
        .filter(|name| {
            prog.get(name).map(|decl| claims.contains(&decl.kind.as_str())).unwrap_or(false)
        })
        .count()
}

pub struct Decision {
    pub label: String,
    pub node: String,
    pub count: usize,
}

/// Open decisions: holes, questions, blind spots. Ranked by what they touch.
pub fn decisions(prog: &Program) -> Vec<Decision> {
    let mut rows: Vec<Decision> = Vec::new();
    for decl in prog.decls {
        for (key, value) in &decl.fields {
            if matches!(value, crate::syntax::Val::Hole) {
                rows.push(Decision {
                    label: format!("{}.{key} (hole)", decl.name),
                    node: decl.name.clone(),
                    count: unlocks(prog, &decl.name, 2),
                });
            }
        }
    }
    for question in prog.of_kind("question") {
        rows.push(Decision {
            label: format!("{} (question)", question.name),
            node: question.name.clone(),
            count: unlocks(prog, &question.name, 2),
        });
    }
    for blindspot in prog.of_kind("blindspot") {
        rows.push(Decision {
            label: format!("{} (blind spot)", blindspot.name),
            node: blindspot.name.clone(),
            count: unlocks(prog, &blindspot.name, 2),
        });
    }
    rows.sort_by(|left, right| right.count.cmp(&left.count).then_with(|| left.label.cmp(&right.label)));
    rows
}

pub fn report(prog: &Program) -> Vec<String> {
    let deps = dependents(prog);
    let supports = supporting_warrants(prog);
    let mut lines: Vec<String> = Vec::new();

    let dead_facts: Vec<String> = prog
        .of_kind("fact")
        .iter()
        .filter(|decl| deps.get(&decl.name).map(|list| list.is_empty()).unwrap_or(true))
        .map(|decl| decl.name.clone())
        .collect();
    if !dead_facts.is_empty() {
        lines.push(format!("dead facts (nothing depends on them): {}", dead_facts.join(", ")));
    }

    let single: Vec<(String, String)> = supports
        .iter()
        .filter(|(_, warrants)| warrants.len() == 1)
        .map(|(proposition, warrants)| (proposition.clone(), warrants[0].clone()))
        .collect();
    for (proposition, warrant) in &single {
        lines.push(format!("single support: {proposition} rests on {warrant} alone"));
    }

    let orphan_props: Vec<String> = prog
        .of_kind("proposition")
        .iter()
        .filter(|decl| {
            let supported = supports.iter().any(|(name, _)| name == &decl.name);
            let assumed = deps
                .get(&decl.name)
                .map(|list| list.iter().any(|(name, kind)| kind == "assumes" && !name.is_empty()))
                .unwrap_or(false);
            !supported && !assumed
        })
        .map(|decl| decl.name.clone())
        .collect();
    if !orphan_props.is_empty() {
        lines.push(format!(
            "propositions with no rule and no dependent: {}",
            orphan_props.join(", ")
        ));
    }

    // load-bearing: facts whose removal would leave a single-support proposition alone
    for (proposition, warrant) in &single {
        let Some(decl) = prog.get(warrant) else { continue };
        for premise in f_ids(decl, "from") {
            let users: Vec<String> = deps
                .get(&premise)
                .map(|list| {
                    list.iter()
                        .filter(|(_, kind)| kind == "premise")
                        .map(|(name, _)| name.clone())
                        .collect()
                })
                .unwrap_or_default();
            if users.len() == 1 && &users[0] == warrant {
                lines.push(format!(
                    "load-bearing: {premise} is the only premise of the only rule for {proposition}"
                ));
            }
        }
    }

    let mut counts: HashMap<&str, usize> = HashMap::new();
    for decl in prog.decls {
        *counts.entry(decl.kind.as_str()).or_insert(0) += 1;
    }
    let mut pairs: Vec<(&str, usize)> = counts.into_iter().collect();
    pairs.sort();
    let joined: Vec<String> = pairs.iter().map(|(kind, count)| format!("{kind}={count}")).collect();
    lines.push(format!("graph: {}", joined.join(", ")));
    lines
}

/// The support graph as costs see it: only edges a conclusion is built from.
pub fn support_adjacency(prog: &Program) -> Vec<(String, Vec<String>)> {
    let support = ["premise", "basis", "concludes", "assumes", "separates", "from", "closes with"];
    let mut out = Vec::new();
    for (source, targets) in dependents(prog).items() {
        let filtered: Vec<String> = targets
            .iter()
            .filter(|(dst, kind)| prog.has(dst) && support.contains(&kind.as_str()))
            .map(|(dst, _)| dst.clone())
            .collect();
        out.push((source, filtered));
    }
    out
}

/// The declaration a report line is about, when the line names one.
pub fn referenced<'a>(prog: &Program<'a>, name: &str) -> Option<&'a Decl> {
    prog.get(name)
}
