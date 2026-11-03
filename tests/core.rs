//! The core document and the tree agree.
//!
//! `docs/CORE.md` assigns every diagnostic code to one of the six questions the checker asks.
//! A prose table drifts: a code gets added, renamed or dropped, and the table keeps describing a
//! tree that no longer exists. This reads the fenced block in the document and holds it to
//! `explain::CODES`, so the two move in the same change.

use std::collections::BTreeSet;
use std::path::Path;

use tsare::explain::CODES;

fn core() -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("docs")
        .join("CORE.md");
    std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{} is part of the repository: {error}", path.display()))
}

/// The codes in the fenced block that starts with `R1 shape`.
fn assigned() -> Vec<String> {
    let text = core();
    let start = text
        .find("R1 shape")
        .expect("docs/CORE.md has a rule table that starts with R1 shape");
    let open = text[..start]
        .rfind("```")
        .expect("the rule table is inside a fenced block");
    let close = text[start..]
        .find("```")
        .expect("the rule table's fenced block is closed");
    text[open + 3..start + close]
        .split_whitespace()
        .filter(|token| token.starts_with("E-") || token.starts_with("W-"))
        .map(str::to_string)
        .collect()
}

#[test]
fn every_code_is_assigned_to_exactly_one_rule() {
    let assigned = assigned();
    let mut seen = BTreeSet::new();
    for code in &assigned {
        assert!(
            seen.insert(code.clone()),
            "{code} is assigned to a rule twice"
        );
    }

    let known: BTreeSet<String> = CODES.iter().map(|code| code.to_string()).collect();
    let missing: Vec<&String> = known.difference(&seen).collect();
    let extra: Vec<&String> = seen.difference(&known).collect();
    assert!(
        missing.is_empty(),
        "docs/CORE.md does not assign these codes: {missing:?}"
    );
    assert!(
        extra.is_empty(),
        "docs/CORE.md assigns these, which are not codes: {extra:?}"
    );
}
