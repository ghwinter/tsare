//! The growth gate: a declaration enters a library only if the library stays admissible.
//!
//! The gate is the verifier half of growth: it copies one declaration out of a source document,
//! appends it with a content-addressed entry note, and keeps the result only if `check` finds no
//! error.  Whether the abstraction earns its place (the benefit half) is not measured here.

use std::path::Path;

use tsare_core::Diagnostic;
use tsare_lang::hash::hex;
use tsare_lang::syntax::{parse, Decl};

/// A declaration that entered, and the library it entered.
pub struct Grown {
    pub name: String,
    pub source_sha256: String,
    pub warnings: usize,
    pub text: String,
    /// The abstract's bytes, so the coverage can be compared with the library's size.
    pub bytes: usize,
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
    let broken: Vec<Diagnostic> = tsare_core::check(&library)
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
    let diagnostics = tsare_core::check(&decls);
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
    if let Ok(link) = tsare_core::chain::chain(&decls, name) {
        if let Some(hypothesis) = tsare_core::chain::undecided(&link).into_iter().next() {
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
        bytes: block.len(),
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

/// One grown abstract and how often the cases reuse it.
pub struct Entry {
    pub name: String,
    pub bytes: usize,
    pub uses: usize,
}

/// The library report: the abstracts, their bytes, their uses, and the compression.
pub struct LibraryReport {
    pub entries: Vec<Entry>,
    pub abstract_bytes: usize,
    pub uses: usize,
}

impl LibraryReport {
    /// The covered bytes per abstract byte: the weighted reuse, which is the compression.
    pub fn compression(&self) -> f64 {
        if self.abstract_bytes == 0 {
            0.0
        } else {
            let covered: usize = self
                .entries
                .iter()
                .map(|entry| entry.bytes * entry.uses)
                .sum();
            covered as f64 / self.abstract_bytes as f64
        }
    }

    pub fn text(&self) -> String {
        let mut out = format!(
            "library: {} abstract(s), {} bytes, {} use(s), compression {:.2}\n",
            self.entries.len(),
            self.abstract_bytes,
            self.uses,
            self.compression()
        );
        for entry in &self.entries {
            out.push_str(&format!(
                "  {}  {} bytes  {} use(s)\n",
                entry.name, entry.bytes, entry.uses
            ));
        }
        out
    }

    pub fn json(&self) -> tsare_lang::json::Json {
        use tsare_lang::json::Json as J;
        let entries: Vec<J> = self
            .entries
            .iter()
            .map(|entry| {
                J::Obj(vec![
                    ("name".to_string(), J::Str(entry.name.clone())),
                    ("bytes".to_string(), J::Num(entry.bytes as f64)),
                    ("uses".to_string(), J::Num(entry.uses as f64)),
                ])
            })
            .collect();
        J::Obj(vec![
            ("entries".to_string(), J::Arr(entries)),
            (
                "abstract_bytes".to_string(),
                J::Num(self.abstract_bytes as f64),
            ),
            ("uses".to_string(), J::Num(self.uses as f64)),
            ("compression".to_string(), J::Num(self.compression())),
        ])
    }
}

impl tsare_core::out::Report for LibraryReport {
    fn text(&self) -> String {
        LibraryReport::text(self)
    }

    fn json(&self) -> tsare_lang::json::Json {
        LibraryReport::json(self)
    }
}

/// Read a library: the grown notes name the abstracts; the cases say how often each is used.
pub fn report(decls: &[Decl], text: &str, cases: Option<&Path>) -> LibraryReport {
    use std::collections::BTreeMap;
    let mut uses: BTreeMap<String, usize> = BTreeMap::new();
    if let Some(store) = cases {
        for entry in tsare_state::cases::load(store) {
            for name in entry
                .get("abstracts")
                .map(|value| value.strings())
                .unwrap_or_default()
            {
                *uses.entry(name).or_insert(0) += 1;
            }
        }
    }
    let mut entries: Vec<Entry> = Vec::new();
    for decl in decls {
        if decl.kind != "note" || !decl.name.starts_with("N_GROWN_") {
            continue;
        }
        let note = decl
            .fields
            .iter()
            .find(|(key, _)| key == "text")
            .and_then(|(_, value)| match value {
                tsare_lang::syntax::Val::Str(text) => Some(text.clone()),
                _ => None,
            })
            .unwrap_or_default();
        let Some(name) = note
            .split(" as ")
            .nth(1)
            .map(|name| name.trim().to_string())
        else {
            continue;
        };
        let bytes = match decls.iter().find(|candidate| candidate.name == name) {
            Some(abstract_) => slice(text, abstract_).map(|block| block.len()).unwrap_or(0),
            None => 0,
        };
        entries.push(Entry {
            uses: uses.get(&name).copied().unwrap_or(0),
            name,
            bytes,
        });
    }
    let abstract_bytes = entries.iter().map(|entry| entry.bytes).sum();
    let uses = entries.iter().map(|entry| entry.uses).sum();
    LibraryReport {
        entries,
        abstract_bytes,
        uses,
    }
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
                let errors =
                    tsare_core::check(&parse(&grown.text).expect("the grown library parses"))
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
