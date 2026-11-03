//! Six representation moves, as concrete candidates for a given document.
//!
//! R1 swap an invariant, R2 drop a constraint, R3 change the representation, R4 invent a unit,
//! R5 build a feedback loop, R6 import from elsewhere.  Each candidate names a target and the
//! signal that produced it; nothing here decides whether the move is a good idea.

use crate::assumptions::report as assumption_report;
use crate::program::{f_id, f_ids, f_str, instrument_of, Program};
use crate::syntax::Val;

pub fn moves() -> Vec<(&'static str, &'static str)> {
    vec![
        ("R1", "swap an invariant"),
        ("R2", "drop a constraint"),
        ("R3", "change the representation"),
        ("R4", "invent a unit"),
        ("R5", "build a feedback loop"),
        ("R6", "import from elsewhere"),
    ]
}

pub struct Move {
    pub code: &'static str,
    pub target: String,
    pub why: String,
}

pub fn suggest(prog: &Program) -> Vec<Move> {
    let mut rows: Vec<Move> = Vec::new();

    // R5: no signal exists that could settle this claim
    for decl in prog.of_kind("hypothesis") {
        if f_str(decl, "test").is_none() && f_id(decl, "rival").is_none() {
            rows.push(Move {
                code: "R5",
                target: decl.name.clone(),
                why: "no test and no rival: nothing would settle it".to_string(),
            });
        }
    }
    for decl in prog.of_kind("fact") {
        if matches!(decl.fields.iter().find(|(key, _)| key == "evidence").map(|(_, value)| value), Some(Val::Id(name)) if name == "model")
            && f_str(decl, "confirmed_by").is_none()
        {
            rows.push(Move {
                code: "R5",
                target: decl.name.clone(),
                why: "model-supplied and unconfirmed: no feedback loop".to_string(),
            });
        }
    }

    // R4: a quantity with no instrument
    for decl in prog.of_kind("value") {
        if instrument_of(decl, "measured_by").is_none() {
            rows.push(Move {
                code: "R4",
                target: decl.name.clone(),
                why: "no measured_by: the number has no instrument".to_string(),
            });
        }
    }

    // R2: load-bearing and never checked
    let (assumptions, _) = assumption_report(prog, 20);
    for row in assumptions {
        rows.push(Move {
            code: "R2",
            target: row.name,
            why: format!("{} (touches {})", row.why, row.count),
        });
    }

    // R3: a document that lists facts but forms no concept
    let facts = prog.of_kind("fact").len();
    let concepts = prog.of_kind("concept").len();
    if facts >= 3 && concepts == 0 {
        rows.push(Move {
            code: "R3",
            target: "(document)".to_string(),
            why: format!("{facts} facts and no concept: compress them"),
        });
    }

    // R1: a conclusion hanging on one premise
    for proposition in prog.of_kind("proposition") {
        let warrants: Vec<&crate::syntax::Decl> = prog
            .of_kind("warrant")
            .into_iter()
            .filter(|warrant| f_id(warrant, "to") == Some(proposition.name.as_str()))
            .collect();
        if warrants.len() == 1 {
            let premises = f_ids(warrants[0], "from");
            if premises.len() == 1 {
                rows.push(Move {
                    code: "R1",
                    target: proposition.name.clone(),
                    why: format!("rests on {} alone via {}", premises[0], warrants[0].name),
                });
            }
        }
    }

    // R6: the material to import from is another document
    if !prog.of_kind("question").is_empty() {
        rows.push(Move {
            code: "R6",
            target: "(document)".to_string(),
            why: "has open questions: map another document for structure".to_string(),
        });
    }

    rows
}
