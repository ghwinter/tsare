//! Transitive debt: which facts gate a conclusion, and how they were obtained.
//!
//! `cost` counts what a declaration owes directly.  This walks the support graph instead, so a
//! claim is charged for the evidence at the end of its chain even when a proposition sits in
//! between.  The output is the first sentence the document does not contain.

use crate::program::{support_edges, walk, Program};

pub struct Debt {
    pub name: String,
    pub worst: &'static str,
    pub weak: usize,
    pub total: usize,
    pub lines: Vec<String>,
}

pub fn report(prog: &Program, only_gated: bool) -> Vec<Debt> {
    let edges = support_edges(prog);
    let mut rows = Vec::new();
    for decl in prog.decls {
        if decl.kind != "proposition" && decl.kind != "hypothesis" {
            continue;
        }
        let facts = walk(prog, &decl.name, &edges);
        let total = facts.len();
        let weak: Vec<&crate::program::Step> =
            facts.iter().filter(|step| step.status != "measured").collect();
        if only_gated && weak.is_empty() {
            continue;
        }
        let worst = if weak.iter().any(|step| step.status == "unconfirmed") {
            "unconfirmed"
        } else if !weak.is_empty() {
            "asserted"
        } else {
            "clear"
        };
        let lines = weak
            .iter()
            .map(|step| {
                format!("{} ({}) via {}", step.fact, step.status, step.path.join(" <- "))
            })
            .collect();
        rows.push(Debt {
            name: decl.name.clone(),
            worst,
            weak: weak.len(),
            total,
            lines,
        });
    }
    rows.sort_by(|left, right| {
        right.weak.cmp(&left.weak).then_with(|| left.name.cmp(&right.name))
    });
    rows
}
