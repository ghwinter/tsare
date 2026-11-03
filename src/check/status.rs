//! The status report: one reading of a ledger, rendered as a line or as JSON.
//!
//! The reading is data; `text` and `json` are two renderings of the same data.  The CLI chooses a
//! sink, so a caller can check the report without spawning the process.

use crate::ask;
use crate::check;
use crate::decide;
use crate::epistemic;
use crate::json::Json as J;
use crate::program::Program;
use crate::syntax::Decl;

/// One diagnostic summary, for the JSON's `first`.
pub struct First {
    pub code: String,
    pub name: String,
    pub message: String,
}

/// One debt, for the JSON's `debt`.
pub struct Debt {
    pub name: String,
    pub kind: String,
    pub reason: String,
}

/// The reading of a ledger: the decision, the counts, the next question and the gaps.
pub struct StatusReport {
    pub decision: String,
    pub verified: usize,
    pub inferred: usize,
    pub unverified: usize,
    pub errors: usize,
    pub warnings: usize,
    pub question: Option<String>,
    pub first: Option<First>,
    pub claims: usize,
    pub debt: Option<Debt>,
    pub open: Vec<(String, String, Vec<String>)>,
}

/// Read a document once; the two renderings read this, not the document again.
pub fn report(decls: &[Decl]) -> StatusReport {
    let diagnostics = check::check(decls);
    let errors = diagnostics
        .iter()
        .filter(|item| item.code.starts_with('E'))
        .count();
    let warnings = diagnostics
        .iter()
        .filter(|item| item.code.starts_with('W'))
        .count();
    let verdict = decide::report(decls);
    let rows = epistemic::report(decls);
    let verified = rows
        .iter()
        .filter(|row| row.knowing == epistemic::Knowing::Verified)
        .count();
    let inferred = rows
        .iter()
        .filter(|row| row.knowing == epistemic::Knowing::Inferred)
        .count();
    let unverified = rows.len() - verified - inferred;
    let question = ask::questions(&Program::new(decls))
        .into_iter()
        .next()
        .map(|item| item.question);
    let first = diagnostics.first().map(|item| First {
        code: item.code.to_string(),
        name: item.witness.clone().unwrap_or_default(),
        message: item.message.clone(),
    });
    let debt = verdict.items.first().map(|item| Debt {
        name: item.name.clone(),
        kind: item.kind.clone(),
        reason: item.reason.to_string(),
    });
    StatusReport {
        decision: verdict.decision.label().to_string(),
        verified,
        inferred,
        unverified,
        errors,
        warnings,
        question,
        first,
        claims: decls.len(),
        debt,
        open: crate::chain::open(decls),
    }
}

impl StatusReport {
    /// The one line: the first error, or the decision, the counts, the open chains and the question.
    pub fn text(&self) -> String {
        if self.errors > 0 {
            let detail = match &self.first {
                Some(item) => format!("first {} {}: {}", item.code, item.name, item.message),
                None => "no diagnostic rows".to_string(),
            };
            return format!(
                "not checkable; {} error(s), {} warning(s); {detail}",
                self.errors, self.warnings
            );
        }
        let warning = if self.warnings > 0 {
            format!(", {} warning(s)", self.warnings)
        } else {
            String::new()
        };
        let chains = match self.open.first() {
            Some((name, _kind, hypotheses)) => format!(
                "; {} open chain(s) ({name} on {})",
                self.open.len(),
                hypotheses.join(", ")
            ),
            None => String::new(),
        };
        let next = match &self.question {
            Some(text) => format!("next question — {text}"),
            None => "nothing to ask".to_string(),
        };
        format!(
            "{}; {} verified, {} inferred, {} unverified{warning}{chains}; {next}",
            self.decision, self.verified, self.inferred, self.unverified
        )
    }

    /// The same reading as data, with the field order the goldens record.
    pub fn json(&self) -> J {
        let first = match &self.first {
            Some(item) => J::Obj(vec![
                ("code".to_string(), J::Str(item.code.clone())),
                ("name".to_string(), J::Str(item.name.clone())),
                ("message".to_string(), J::Str(item.message.clone())),
            ]),
            None => J::Null,
        };
        let debt = match &self.debt {
            Some(item) => J::Obj(vec![
                ("name".to_string(), J::Str(item.name.clone())),
                ("kind".to_string(), J::Str(item.kind.clone())),
                ("reason".to_string(), J::Str(item.reason.clone())),
            ]),
            None => J::Null,
        };
        let open_first = match self.open.first() {
            Some((name, kind, hypotheses)) => J::Obj(vec![
                ("name".to_string(), J::Str(name.clone())),
                ("kind".to_string(), J::Str(kind.clone())),
                (
                    "hypotheses".to_string(),
                    J::Arr(hypotheses.iter().map(|h| J::Str(h.clone())).collect()),
                ),
            ]),
            None => J::Null,
        };
        J::Obj(vec![
            ("decision".to_string(), J::Str(self.decision.clone())),
            ("verified".to_string(), J::Num(self.verified as f64)),
            ("inferred".to_string(), J::Num(self.inferred as f64)),
            ("unverified".to_string(), J::Num(self.unverified as f64)),
            ("errors".to_string(), J::Num(self.errors as f64)),
            ("warnings".to_string(), J::Num(self.warnings as f64)),
            (
                "question".to_string(),
                match &self.question {
                    Some(text) => J::Str(text.clone()),
                    None => J::Null,
                },
            ),
            ("first".to_string(), first),
            ("claims".to_string(), J::Num(self.claims as f64)),
            ("debt".to_string(), debt),
            ("open_chains".to_string(), J::Num(self.open.len() as f64)),
            ("open_chain".to_string(), open_first),
        ])
    }
}

/// The one line, for callers that only need the text.
pub fn line(decls: &[Decl]) -> String {
    report(decls).text()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::syntax::parse;

    const DOC: &str = "tsare 1\nfact F { claim: \"f\", evidence: user }\n";

    #[test]
    fn the_report_renders_text_without_a_process() {
        let decls = parse(DOC).expect("doc");
        let report = report(&decls);
        assert!(report.text().starts_with("closed;"), "{}", report.text());
        assert!(
            report.text().contains("nothing to ask"),
            "{}",
            report.text()
        );
    }

    #[test]
    fn the_report_renders_json_with_the_recorded_fields() {
        let decls = parse(DOC).expect("doc");
        let json = crate::json::serialize(&report(&decls).json());
        assert!(json.contains("\"decision\": \"closed\""), "{json}");
        assert!(json.contains("\"open_chains\": 0"), "{json}");
        assert!(json.contains("\"open_chain\": null"), "{json}");
    }
}
