//! The growth gate: a declaration enters a library only if the library stays admissible.
//!
//! The gate is the verifier half of the whitepaper's growth law: it copies one declaration out of
//! a source document, appends it with a content-addressed entry note, and keeps the result only
//! if `check` finds no error.  Whether the abstraction earns its place (the benefit half) is not
//! measured here.

use crate::check::{self, Diagnostic};
use crate::hash::hex;
use crate::syntax::{parse, Decl};

/// A declaration that entered, and the library it entered.
pub struct Grown {
    pub name: String,
    pub source_sha256: String,
    pub warnings: usize,
    pub text: String,
}

/// The gate's answer: the grown library, or why the declaration did not enter.
pub enum Verdict {
    Accepted(Grown),
    Rejected {
        reason: String,
        diagnostics: Vec<Diagnostic>,
    },
}

pub fn grow(library_text: &str, source_text: &str, name: &str) -> Verdict {
    let library = match parse(library_text) {
        Ok(decls) => decls,
        Err(error) => {
            return Verdict::Rejected {
                reason: format!("the library does not parse: {error}"),
                diagnostics: Vec::new(),
            }
        }
    };
    let broken: Vec<Diagnostic> = check::check(&library)
        .into_iter()
        .filter(|item| item.code.starts_with('E'))
        .collect();
    if !broken.is_empty() {
        return Verdict::Rejected {
            reason: "the library is not admissible".to_string(),
            diagnostics: broken,
        };
    }
    let source = match parse(source_text) {
        Ok(decls) => decls,
        Err(error) => {
            return Verdict::Rejected {
                reason: format!("the source does not parse: {error}"),
                diagnostics: Vec::new(),
            }
        }
    };
    let Some(candidate) = source.iter().find(|decl| decl.name == name) else {
        return Verdict::Rejected {
            reason: format!("the source declares no `{name}`"),
            diagnostics: Vec::new(),
        };
    };
    if candidate.kind == "note" {
        return Verdict::Rejected {
            reason: "a note is not a commitment".to_string(),
            diagnostics: Vec::new(),
        };
    }
    if library.iter().any(|decl| decl.name == name) {
        return Verdict::Rejected {
            reason: format!("the library already declares `{name}`"),
            diagnostics: Vec::new(),
        };
    }
    let Some(block) = slice(source_text, candidate) else {
        return Verdict::Rejected {
            reason: format!("`{name}` has no source span"),
            diagnostics: Vec::new(),
        };
    };
    let source_sha256 = hex(source_text.as_bytes());
    let note = entry_note(&library, name, &source_sha256);
    let merged = format!(
        "{}\n\n{}\n\n{}\n",
        library_text.trim_end(),
        block.trim_end(),
        note.trim_end()
    );
    let decls = match parse(&merged) {
        Ok(decls) => decls,
        Err(error) => {
            return Verdict::Rejected {
                reason: format!("the grown library does not parse: {error}"),
                diagnostics: Vec::new(),
            }
        }
    };
    let diagnostics = check::check(&decls);
    let errors: Vec<Diagnostic> = diagnostics
        .iter()
        .filter(|item| item.code.starts_with('E'))
        .cloned()
        .collect();
    if !errors.is_empty() {
        return Verdict::Rejected {
            reason: "the grown library is not admissible".to_string(),
            diagnostics: errors,
        };
    }
    // A conclusion written back must reach evidence: a hypothesis no verdict has decided is a
    // question, not a premise.
    if let Ok(link) = crate::chain::chain(&decls, name) {
        if let Some(hypothesis) = crate::chain::undecided(&link).into_iter().next() {
            return Verdict::Rejected {
                reason: format!("{name} stands on the unbacked hypothesis {hypothesis}"),
                diagnostics: Vec::new(),
            };
        }
    }
    let warnings = diagnostics
        .iter()
        .filter(|item| item.code.starts_with('W'))
        .count();
    Verdict::Accepted(Grown {
        name: name.to_string(),
        source_sha256,
        warnings,
        text: merged,
    })
}

/// The lines the declaration occupies, or `None` when its span is outside the text.
fn slice(text: &str, decl: &Decl) -> Option<String> {
    let lines: Vec<&str> = text.lines().collect();
    if decl.line == 0 || decl.end_line < decl.line || decl.end_line > lines.len() {
        return None;
    }
    Some(lines[decl.line - 1..decl.end_line].join("\n"))
}

/// A note name the library does not use, so the entry cannot shadow a declaration.
fn entry_note(library: &[Decl], name: &str, sha: &str) -> String {
    let base = format!("N_GROWN_{name}");
    let chosen = if library.iter().all(|decl| decl.name != base) {
        base
    } else {
        let alt = format!("{base}_{}", &sha[..8]);
        if library.iter().all(|decl| decl.name != alt) {
            alt
        } else {
            format!("{base}_{}", &sha[..16])
        }
    };
    format!(
        "note {chosen} {{ tag: conjecture, text: \"entered from {} as {name}\" }}",
        &sha[..16]
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const LIBRARY: &str = "tsare 1\nfact F_LIB { claim: \"library fact\", evidence: user }\n";

    #[test]
    fn a_checked_declaration_enters_with_its_source_hash() {
        let source = "tsare 1\nfact F_NEW { claim: \"new\", evidence: tool(\"x\") }\n";
        match grow(LIBRARY, source, "F_NEW") {
            Verdict::Accepted(grown) => {
                assert!(grown.text.contains("F_NEW"));
                assert!(grown.text.contains("N_GROWN_F_NEW"));
                assert!(grown.text.contains("entered from"));
                assert_eq!(grown.source_sha256.len(), 64);
                let errors = check::check(&parse(&grown.text).expect("the grown library parses"))
                    .iter()
                    .filter(|item| item.code.starts_with('E'))
                    .count();
                assert_eq!(errors, 0);
            }
            Verdict::Rejected { reason, .. } => panic!("rejected: {reason}"),
        }
    }

    #[test]
    fn a_dangling_reference_is_rejected() {
        let source = "tsare 1\nproposition P_NEW { statement: \"s\", from: [F_MISSING] }\n";
        match grow(LIBRARY, source, "P_NEW") {
            Verdict::Rejected { diagnostics, .. } => {
                assert!(
                    diagnostics.iter().any(|item| item.code == "E-UNDEFINED"),
                    "{diagnostics:?}"
                );
            }
            Verdict::Accepted(_) => panic!("a dangling reference must not enter"),
        }
    }

    #[test]
    fn a_name_the_library_has_is_rejected() {
        let source = "tsare 1\nfact F_LIB { claim: \"again\", evidence: user }\n";
        assert!(matches!(
            grow(LIBRARY, source, "F_LIB"),
            Verdict::Rejected { .. }
        ));
    }

    #[test]
    fn a_note_is_not_a_commitment() {
        let source = "tsare 1\nnote N_NEW { tag: conjecture, text: \"x\" }\n";
        assert!(matches!(
            grow(LIBRARY, source, "N_NEW"),
            Verdict::Rejected { .. }
        ));
    }

    #[test]
    fn a_broken_library_is_not_grown() {
        let library = "tsare 1\nfact F_LIB { claim: \"no evidence\" }\n";
        let source = "tsare 1\nfact F_NEW { claim: \"x\", evidence: user }\n";
        match grow(library, source, "F_NEW") {
            Verdict::Rejected { reason, .. } => {
                assert!(reason.contains("not admissible"), "{reason}");
            }
            Verdict::Accepted(_) => panic!("a broken library must not grow"),
        }
    }
}
