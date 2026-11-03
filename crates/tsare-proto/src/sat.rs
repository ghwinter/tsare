//! A satisfiability check over what the document says, in plain DPLL.
//!
//! The encoding is deliberately literal: every fact is true, every rule says "if the premises
//! hold then the conclusion holds", and an attack says "these two cannot both hold".  That is a
//! consistency question the checker itself does not ask -- `closure` computes which rules survive,
//! not whether every stated rule can hold at once.  When the answer is "no", a deletion pass
//! shrinks the contradictory set to a core worth reading.
//!
//! Limits, stated so nobody trusts it further than it goes: no theory reasoning (no integers, no
//! ordering), no negation except an attack, and the encoding ignores the grounded semantics.

use tsare_lang::program::{applications, f_id, f_ids};
use tsare_lang::syntax::Decl;

pub struct Answer {
    pub satisfiable: bool,
    /// The variable names true in the first model DPLL found.
    pub model: Vec<String>,
    /// When unsatisfiable, a set of statements whose removal makes it satisfiable.
    pub core: Vec<String>,
}

type Lit = (usize, bool); // (variable, positive?)

fn variable(names: &mut Vec<String>, name: &str) -> usize {
    match names.iter().position(|known| known == name) {
        Some(index) => index,
        None => {
            names.push(name.to_string());
            names.len() - 1
        }
    }
}

/// One clause, with the sentence it came from, so an unsatisfiable core can be printed in words.
struct Clause {
    lits: Vec<Lit>,
    text: String,
}

fn clauses(decls: &[Decl]) -> (Vec<String>, Vec<Clause>) {
    let mut names = Vec::new();
    let mut out = Vec::new();
    let retired = tsare_lang::program::retired(decls);
    for decl in decls.iter().filter(|decl| decl.kind == "fact") {
        if retired.contains(&decl.name) {
            continue;
        }
        let index = variable(&mut names, &decl.name);
        out.push(Clause {
            lits: vec![(index, true)],
            text: format!("fact {}", decl.name),
        });
    }
    for rule in applications(decls) {
        if retired.contains(&rule.warrant) {
            continue;
        }
        // a rule is two implications: the warrant fires when its premises hold, and a firing
        // warrant concludes its target.  Without the first, an attack between warrants would
        // always be satisfiable by setting both warrants false.
        let mut fires: Vec<Lit> = rule
            .premises
            .iter()
            .map(|premise| (variable(&mut names, premise), false))
            .collect();
        fires.push((variable(&mut names, &rule.warrant), true));
        out.push(Clause {
            lits: fires,
            text: format!("{}: {} -> fires", rule.warrant, rule.premises.join(" & ")),
        });
        out.push(Clause {
            lits: vec![
                (variable(&mut names, &rule.warrant), false),
                (variable(&mut names, &rule.target), true),
            ],
            text: format!("{} concludes {}", rule.warrant, rule.target),
        });
    }
    for decl in decls.iter().filter(|decl| decl.kind == "warrant") {
        if retired.contains(&decl.name) {
            continue;
        }
        for target in f_ids(decl, "attacks") {
            let left = variable(&mut names, &decl.name);
            let right = variable(&mut names, &target);
            out.push(Clause {
                lits: vec![(left, false), (right, false)],
                text: format!("attack {} -> {target}", decl.name),
            });
        }
    }
    let _ = f_id; // kept for readability of the imports above
    (names, out)
}

fn unit_propagate(assignment: &mut [Option<bool>], clauses: &[&Clause]) -> bool {
    loop {
        let mut changed = false;
        for clause in clauses {
            let mut unit: Option<Lit> = None;
            let mut satisfied = false;
            let mut unassigned = 0;
            for &(var, positive) in &clause.lits {
                match assignment[var] {
                    Some(value) => {
                        if value == positive {
                            satisfied = true;
                            break;
                        }
                    }
                    None => {
                        unassigned += 1;
                        unit = Some((var, positive));
                    }
                }
            }
            if satisfied {
                continue;
            }
            if unassigned == 0 {
                return false;
            }
            if unassigned == 1 {
                let (var, positive) = unit.expect("one unassigned literal");
                assignment[var] = Some(positive);
                changed = true;
            }
        }
        if !changed {
            return true;
        }
    }
}

fn dpll(assignment: &mut [Option<bool>], clauses: &[&Clause]) -> bool {
    if !unit_propagate(assignment, clauses) {
        return false;
    }
    let Some((var, _)) = clauses
        .iter()
        .flat_map(|clause| clause.lits.iter().copied())
        .find(|(var, _)| assignment[*var].is_none())
    else {
        return true;
    };
    for value in [true, false] {
        let mut next = assignment.to_vec();
        next[var] = Some(value);
        if dpll(&mut next, clauses) {
            assignment.copy_from_slice(&next);
            return true;
        }
    }
    false
}

fn satisfiable(names: &[String], clauses: &[&Clause]) -> Option<Vec<bool>> {
    let mut assignment = vec![None; names.len()];
    if dpll(&mut assignment, clauses) {
        Some(
            assignment
                .into_iter()
                .map(|value| value.unwrap_or(false))
                .collect(),
        )
    } else {
        None
    }
}

pub fn solve(decls: &[Decl]) -> Answer {
    let (names, clauses) = clauses(decls);
    let all: Vec<&Clause> = clauses.iter().collect();
    match satisfiable(&names, &all) {
        Some(model) => Answer {
            satisfiable: true,
            model: names
                .iter()
                .zip(model.iter())
                .filter(|(_, value)| **value)
                .map(|(name, _)| name.clone())
                .collect(),
            core: Vec::new(),
        },
        None => {
            // deletion-based core: a clause is kept only when dropping it does not make the rest
            // satisfiable, which leaves a set that is contradictory on its own
            let mut kept: Vec<usize> = (0..clauses.len()).collect();
            for index in 0..clauses.len() {
                let without: Vec<&Clause> = kept
                    .iter()
                    .filter(|other| **other != index)
                    .map(|other| &clauses[*other])
                    .collect();
                if satisfiable(&names, &without).is_none() {
                    kept.retain(|other| *other != index);
                }
            }
            Answer {
                satisfiable: false,
                model: Vec::new(),
                core: kept
                    .iter()
                    .map(|index| clauses[*index].text.clone())
                    .collect(),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tsare_lang::syntax::parse;

    #[test]
    fn two_rules_that_cannot_both_hold_have_a_core() {
        let source = "\
fact F_A { claim: \"a\", evidence: user }
fact F_B { claim: \"b\", evidence: user }
proposition P_ONE { statement: \"one\", from: [F_A] }
proposition P_TWO { statement: \"two\", from: [F_B] }
warrant W_ONE { claim: \"r\", from: [F_A], to: P_ONE, basis: [F_A] }
warrant W_TWO { claim: \"r\", from: [F_B], to: P_TWO, basis: [F_B], attacks: [W_ONE] }
";
        let decls = parse(source).expect("parses");
        let answer = solve(&decls);
        assert!(!answer.satisfiable, "{:?}", answer.core);
        assert!(answer.core.iter().any(|line| line.contains("attack")));
    }

    #[test]
    fn a_rule_set_that_holds_together_has_a_model() {
        let source = "\
fact F_A { claim: \"a\", evidence: user }
proposition P_ONE { statement: \"one\", from: [F_A] }
warrant W_ONE { claim: \"r\", from: [F_A], to: P_ONE, basis: [F_A] }
";
        let decls = parse(source).expect("parses");
        let answer = solve(&decls);
        assert!(answer.satisfiable);
        assert!(answer.model.contains(&"P_ONE".to_string()));
    }
}
