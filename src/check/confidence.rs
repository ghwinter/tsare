//! A declared number is a claim about support, so it can exceed what the document supports.

use super::*;

pub(crate) fn check_confidence_flow(decls: &[Decl], out: &mut Vec<Diagnostic>) {
    let program = crate::program::Program::new(decls);
    let weights = crate::weigh::weights(&program);
    for decl in decls.iter().filter(|decl| decl.kind == "proposition") {
        let Some(declared) = crate::program::f_num(decl, "confidence") else {
            continue;
        };
        if let Some(row) = weights.iter().find(|row| row.name == decl.name) {
            if row.status == "derived" {
                if let Some(value) = row.value {
                    if declared > value + 1e-9 {
                        sem(
                            decl,
                            "W-CONFIDENCE-INFLATED",
                            format!(
                                "proposition {} declares confidence {declared}, but its weakest link supports {value}",
                                decl.name
                            ),
                            Some("confidence"),
                            out,
                        );
                    }
                }
                continue;
            }
        }
        // no rule concludes it, so the number has no derivation and must not exceed the
        // weakest premise it cites
        let mut floor: Option<f64> = None;
        for name in crate::program::f_ids(decl, "from") {
            let Some(fact) = program.get(&name) else {
                continue;
            };
            let Some(value) = crate::program::f_num(fact, "confidence") else {
                continue;
            };
            floor = Some(floor.map_or(value, |low: f64| low.min(value)));
        }
        if let Some(floor) = floor {
            if declared > floor + 1e-9 {
                sem(
                    decl,
                    "W-CONFIDENCE-INFLATED",
                    format!(
                        "proposition {} declares confidence {declared} with no rule, above its weakest premise {floor}",
                        decl.name
                    ),
                    Some("confidence"),
                    out,
                );
            }
        }
    }
    // a rule's number is bounded by the numbers it consumes: weigh already takes the minimum,
    // so a higher declared number is a claim its own premises cannot carry
    for decl in decls.iter().filter(|decl| decl.kind == "warrant") {
        let Some(declared) = crate::program::f_num(decl, "confidence") else {
            continue;
        };
        let mut floor: Option<f64> = None;
        for name in crate::program::f_ids(decl, "from") {
            let Some(premise) = program.get(&name) else {
                continue;
            };
            let value = crate::program::f_num(premise, "confidence").unwrap_or(1.0);
            floor = Some(floor.map_or(value, |low: f64| low.min(value)));
        }
        if let Some(floor) = floor {
            if declared > floor + 1e-9 {
                sem(
                    decl,
                    "W-CONFIDENCE-INFLATED",
                    format!(
                        "warrant {} declares confidence {declared}, above its weakest premise {floor}",
                        decl.name
                    ),
                    Some("confidence"),
                    out,
                );
            }
        }
    }
}
