//! The assumption ledger: what the document leans on without checking.
//!
//! A declaration is *load-bearing* when something depends on it, and *unsupported* when its
//! own justification is an assertion rather than an instrument, a rule, or a checker.  The
//! intersection is the set of shared assumptions nobody has questioned.

use std::collections::{HashMap, HashSet};

use crate::impact::dependents;
use crate::program::{obligations, Program};

pub fn unsupported(prog: &Program, name: &str) -> Option<String> {
    let decl = prog.get(name)?;
    obligations(prog, decl).into_iter().next()
}

/// Everything reachable from a declaration, restricted to the claims it can affect.
pub fn reachable(prog: &Program) -> HashMap<String, HashSet<String>> {
    let mut adj: HashMap<String, Vec<String>> = HashMap::new();
    for (source, targets) in dependents(prog).items() {
        adj.insert(
            source,
            targets
                .iter()
                .filter(|(dst, _)| prog.has(dst))
                .map(|(dst, _)| dst.clone())
                .collect(),
        );
    }
    let claims = ["proposition", "hypothesis", "question"];
    let mut out = HashMap::new();
    for decl in prog.decls {
        let mut seen: HashSet<String> = HashSet::new();
        let mut frontier: Vec<String> = vec![decl.name.clone()];
        while let Some(node) = frontier.pop() {
            for next in adj.get(&node).cloned().unwrap_or_default() {
                if seen.insert(next.clone()) {
                    frontier.push(next);
                }
            }
        }
        let claims_only: HashSet<String> = seen
            .into_iter()
            .filter(|name| {
                prog.get(name)
                    .map(|decl| claims.contains(&decl.kind.as_str()))
                    .unwrap_or(false)
            })
            .collect();
        out.insert(decl.name.clone(), claims_only);
    }
    out
}

pub struct Assumption {
    pub name: String,
    pub kind: String,
    pub why: String,
    pub count: usize,
}

/// The rows to show and how many there are in all: a caller that is handed only `top` rows cannot
/// say whether it is looking at the whole list, and a total that counts the cut is a false total.
pub fn report(prog: &Program, top: usize) -> (Vec<Assumption>, usize) {
    let impact = reachable(prog);
    let mut rows = Vec::new();
    for decl in prog.decls {
        let Some(why) = unsupported(prog, &decl.name) else {
            continue;
        };
        let count = impact.get(&decl.name).map(|names| names.len()).unwrap_or(0);
        if count == 0 {
            continue;
        }
        rows.push(Assumption {
            name: decl.name.clone(),
            kind: decl.kind.clone(),
            why,
            count,
        });
    }
    rows.sort_by(|left, right| {
        right
            .count
            .cmp(&left.count)
            .then_with(|| left.name.cmp(&right.name))
    });
    let total = rows.len();
    rows.truncate(top);
    (rows, total)
}
