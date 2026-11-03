//! The intervention: what follows when a value is *set*, rather than merely observed.
//!
//! `whatif` deletes a declaration, which removes the node and every edge into and out of it.  An
//! intervention does something else: it removes only the edges *into* the named value and keeps
//! the value itself exogenous, so the downstream consequences still fire.  That distinction --
//! delete versus do -- is the whole reason a causal engine is not a what-if.
//!
//! Limits: the graph is the one the document declares (premises to conclusions), there are no
//! confounders to adjust for, and the answer is reachability rather than a probability.

use std::collections::{HashMap, HashSet};

use tsare_lang::program::applications;
use tsare_lang::syntax::Decl;

pub struct Effect {
    /// The names the observational reading reaches from its facts.
    pub observed: Vec<String>,
    /// The names the interventional reading reaches, with the edges into `do` removed.
    pub intervened: Vec<String>,
    /// Names that only the intervention reaches, and names it loses.
    pub gained: Vec<String>,
    pub lost: Vec<String>,
}

fn edges(decls: &[Decl]) -> HashMap<String, Vec<String>> {
    let mut out: HashMap<String, Vec<String>> = HashMap::new();
    for rule in applications(decls) {
        for premise in rule.premises {
            out.entry(premise).or_default().push(rule.target.clone());
        }
    }
    out
}

fn roots(decls: &[Decl]) -> Vec<String> {
    let retired = tsare_lang::program::retired(decls);
    decls
        .iter()
        .filter(|decl| decl.kind == "fact" && !retired.contains(&decl.name))
        .map(|decl| decl.name.clone())
        .collect()
}

fn reach(
    edges: &HashMap<String, Vec<String>>,
    roots: &[String],
    ignore_incoming: Option<&str>,
) -> Vec<String> {
    let mut seen: HashSet<String> = roots.iter().cloned().collect();
    let mut queue: Vec<String> = roots.to_vec();
    while let Some(current) = queue.pop() {
        for next in edges.get(&current).cloned().unwrap_or_default() {
            if ignore_incoming == Some(next.as_str()) && current != next {
                continue;
            }
            if seen.insert(next.clone()) {
                queue.push(next);
            }
        }
    }
    let mut out: Vec<String> = seen.into_iter().collect();
    out.sort();
    out
}

pub fn intervene(decls: &[Decl], do_name: &str) -> Effect {
    let graph = edges(decls);
    let facts = roots(decls);
    let observed = reach(&graph, &facts, None);
    // `do(X)` cuts every edge into X and makes X a source: its consequences still fire
    let mut intervened_roots = facts.clone();
    if !intervened_roots.iter().any(|name| name == do_name) {
        intervened_roots.push(do_name.to_string());
    }
    let intervened = reach(&graph, &intervened_roots, Some(do_name));
    let before: HashSet<&String> = observed.iter().collect();
    let after: HashSet<&String> = intervened.iter().collect();
    Effect {
        gained: intervened
            .iter()
            .filter(|name| !before.contains(name))
            .cloned()
            .collect(),
        lost: observed
            .iter()
            .filter(|name| !after.contains(name))
            .cloned()
            .collect(),
        observed,
        intervened,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tsare_lang::syntax::parse;

    #[test]
    fn an_intervention_keeps_the_downstream_and_cuts_the_upstream() {
        let source = "\
fact F_A { claim: \"a\", evidence: user }
proposition P_MID { statement: \"mid\", from: [F_A] }
proposition P_END { statement: \"end\", from: [P_MID] }
warrant W_ONE { claim: \"r\", from: [F_A], to: P_MID, basis: [F_A] }
warrant W_TWO { claim: \"r\", from: [P_MID], to: P_END, basis: [P_MID] }
";
        let decls = parse(source).expect("parses");
        let effect = intervene(&decls, "P_MID");
        assert!(
            effect.intervened.contains(&"P_END".to_string()),
            "{:?}",
            effect.intervened
        );
        // the value set by intervention is itself in the reachable set
        assert!(effect.intervened.contains(&"P_MID".to_string()));
        assert!(effect.observed.contains(&"P_END".to_string()));
    }
}
