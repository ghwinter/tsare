//! What the compiler decides, in the smallest form that can be checked by eye.
//!
//! These assertions came from the reference suite and were kept because they name a property,
//! not an implementation: a rule that attacks another defeats it, an ungrounded premise makes
//! a conclusion conditional, a duplicate is repaired only when the repair compiles better.

pub use std::collections::HashSet;

pub use tsare::argue::conclusions;
pub use tsare::build::{learnings_of, verify_predicates};
pub use tsare::check::check;
pub use tsare::closure::{engine, grounded_sets};
pub use tsare::cost::report as cost_report;
pub use tsare::derive::analyze;
pub use tsare::egraph::merge_groups;
pub use tsare::fix::suggestions;
pub use tsare::induce::induce_concept;
pub use tsare::program::Program;
pub use tsare::sugar::expand;
pub use tsare::syntax::{parse, Decl};

fn decls(source: &str) -> Vec<Decl> {
    parse(source).expect("parse")
}

fn codes(source: &str) -> Vec<&'static str> {
    check(&decls(source)).iter().map(|item| item.code).collect()
}

fn statuses(source: &str) -> std::collections::HashMap<String, &'static str> {
    let decls = decls(source);
    conclusions(&Program::new(&decls))
        .into_iter()
        .map(|row| (row.name, row.status))
        .collect()
}

const ATTACK: &str = r#"
fact F1 { claim: "x", evidence: user }
proposition P1 { statement: "a", from: [F1] }
warrant W1 { claim: "r1", from: [F1], to: P1 }
warrant W2 { claim: "r2", from: [F1], to: P1, attacks: [W1] }
"#;

mod checks;
mod concepts;
mod documents;
mod partitions;
mod repairs;
mod rules;
mod structures;
mod syntax;
