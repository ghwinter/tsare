//! The document's relations as objects: each arrow has a kind, two ends, and the place in the
//! file that states it.
//!
//! The kinds are the theory's morphism classes, not the field names, so two documents that spell
//! the same relation differently can be compared without reading their syntax.  Nothing here
//! judges; it reads what the document already says.  The mapping is deliberately short -- a field
//! becomes an arrow only when its meaning is a relation between two named things:
//!
//! | kind       | the document says                         | ends            |
//! | ---------- | ----------------------------------------- | --------------- |
//! | `derives`  | a warrant concludes a proposition          | premises -> target |
//! | `attacks`  | a warrant attacks one, a hypothesis is another's rival | -> the attacked |
//! | `retires`  | `supersedes` withdraws what it is appended after | -> the withdrawn |
//! | `sub`      | a concept names its parent                 | child -> parent |
//! | `asks`     | a question names what it asks about        | question -> target |
//! | `reports`  | a claim quotes a source                    | claim -> source |
//!
//! `grounded_by:` and `against:` are not arrows: they are the extent a concept is computed from,
//! which is what the lattice (`sub`) orders, and giving them an arrow would say a fact "supports"
//! a concept in the same sense a premise supports a claim.

use crate::provenance;
use tsare_lang::program::{applications, f_id, f_ids, f_str};
use tsare_lang::syntax::Decl;

/// One relation the document states, with the place it is written.
#[derive(Debug, Clone, PartialEq)]
pub struct Arrow {
    pub kind: &'static str,
    pub from: Vec<String>,
    pub to: String,
    /// The declaration that carries the relation -- usually a warrant, sometimes the claim itself.
    pub reason: String,
    pub file: String,
    pub line: usize,
    pub col: usize,
}

fn at(decl: &Decl) -> (String, usize, usize) {
    (decl.file.clone(), decl.line, decl.col)
}

/// Every arrow the document states, in source order.
pub fn arrows(decls: &[Decl]) -> Vec<Arrow> {
    let mut out = Vec::new();
    let by_name = |name: &str| decls.iter().find(|decl| decl.name == name);

    // a rule: its premises are the domain, its conclusion the codomain
    for rule in applications(decls) {
        let Some(warrant) = by_name(&rule.warrant) else {
            continue;
        };
        let (file, line, col) = at(warrant);
        out.push(Arrow {
            kind: "derives",
            from: rule.premises,
            to: rule.target,
            reason: rule.warrant,
            file,
            line,
            col,
        });
    }

    for decl in decls {
        let (file, line, col) = at(decl);

        // opposition: a rule defeats a rule, and one hypothesis can be another's rival
        for target in f_ids(decl, "attacks") {
            out.push(Arrow {
                kind: "attacks",
                from: vec![decl.name.clone()],
                to: target,
                reason: decl.name.clone(),
                file: file.clone(),
                line,
                col,
            });
        }
        if let Some(rival) = f_id(decl, "rival") {
            out.push(Arrow {
                kind: "attacks",
                from: vec![decl.name.clone()],
                to: rival.to_string(),
                reason: decl.name.clone(),
                file: file.clone(),
                line,
                col,
            });
        }

        // withdrawal is an append, so the arrow points from the new declaration to the old one
        for target in f_ids(decl, "supersedes") {
            out.push(Arrow {
                kind: "retires",
                from: vec![decl.name.clone()],
                to: target,
                reason: decl.name.clone(),
                file: file.clone(),
                line,
                col,
            });
        }

        if let Some(parent) = f_id(decl, "parent") {
            out.push(Arrow {
                kind: "sub",
                from: vec![decl.name.clone()],
                to: parent.to_string(),
                reason: decl.name.clone(),
                file: file.clone(),
                line,
                col,
            });
        }

        for target in f_ids(decl, "questions") {
            out.push(Arrow {
                kind: "asks",
                from: vec![decl.name.clone()],
                to: target,
                reason: decl.name.clone(),
                file: file.clone(),
                line,
                col,
            });
        }

        // a quote is the one arrow whose span is computed from the text rather than written: it
        // points at the line and column the quote occupies in the source it names
        if let (Some(source), Some(quote)) = (f_id(decl, "source"), f_str(decl, "quote")) {
            let (file, line, col) = match by_name(source) {
                Some(target) => match provenance::locate(&provenance::lines(target), quote) {
                    Some((line, col)) => (target.file.clone(), line, col),
                    None => (file.clone(), line, col),
                },
                None => (file, line, col),
            };
            out.push(Arrow {
                kind: "reports",
                from: vec![decl.name.clone()],
                to: source.to_string(),
                reason: decl.name.clone(),
                file,
                line,
                col,
            });
        }
    }

    out.sort_by(|left, right| {
        (&left.file, left.line, left.col, left.kind, &left.to).cmp(&(
            &right.file,
            right.line,
            right.col,
            right.kind,
            &right.to,
        ))
    });
    out
}

/// How many arrows of each kind, in a stable order.
pub fn counts(arrows: &[Arrow]) -> Vec<(&'static str, usize)> {
    let mut out: Vec<(&'static str, usize)> = Vec::new();
    for arrow in arrows {
        match out.iter_mut().find(|(kind, _)| *kind == arrow.kind) {
            Some((_, count)) => *count += 1,
            None => out.push((arrow.kind, 1)),
        }
    }
    out.sort();
    out
}

/// Which two kinds actually chain, and how often: a pair is `left ∘ right` when an arrow of the
/// left kind ends where an arrow of the right kind begins.
pub fn composition(arrows: &[Arrow]) -> Vec<(String, usize)> {
    let mut pairs: Vec<(String, usize)> = Vec::new();
    for left in arrows {
        for right in arrows {
            if !right.from.iter().any(|name| name == &left.to) {
                continue;
            }
            let key = format!("{}∘{}", left.kind, right.kind);
            match pairs.iter_mut().find(|(name, _)| name == &key) {
                Some((_, count)) => *count += 1,
                None => pairs.push((key, 1)),
            }
        }
    }
    pairs.sort_by(|left, right| right.1.cmp(&left.1).then(left.0.cmp(&right.0)));
    pairs
}

#[cfg(test)]
mod tests {
    use super::*;
    use tsare_lang::syntax::parse;

    #[test]
    fn a_chain_composes_and_the_span_names_the_rule() {
        let source = "\
fact F_A { claim: \"a\", evidence: user }
proposition P_B { statement: \"b\", from: [F_A] }
warrant W_AB { claim: \"a to b\", from: [F_A], to: P_B, basis: [F_A] }
proposition P_C { statement: \"c\", from: [P_B] }
warrant W_BC { claim: \"b to c\", from: [P_B], to: P_C, basis: [P_B] }
";
        let decls = parse(source).expect("parses");
        let arrows = arrows(&decls);
        assert_eq!(arrows.len(), 2, "{arrows:#?}");
        assert_eq!(arrows[0].kind, "derives");
        assert_eq!(arrows[0].from, vec!["F_A".to_string()]);
        assert_eq!(
            arrows[0].line, 3,
            "the arrow is written where the warrant is"
        );
        let pairs = composition(&arrows);
        assert_eq!(pairs, vec![("derives∘derives".to_string(), 1)]);
    }

    #[test]
    fn a_quote_points_at_the_line_the_words_are_on() {
        let source = "\
source S_T { name: \"t\", text: [\"first line\", \"the words\"] }
fact F_Q { claim: \"q\", evidence: user, source: S_T, quote: \"the words\" }
";
        let decls = parse(source).expect("parses");
        let arrows = arrows(&decls);
        let report = arrows
            .iter()
            .find(|arrow| arrow.kind == "reports")
            .expect("a reports arrow");
        assert_eq!(report.to, "S_T");
        assert_eq!(report.line, 2, "the second line of the source text");
        assert_eq!(report.col, 1);
    }
}
