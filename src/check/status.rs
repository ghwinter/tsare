//! The loop's one line: the decision, the epistemic counts and the next question.
//!
//! One reading of a ledger, in the same terms as the gate's brief.  The line is for a reader; the
//! `status` command's JSON carries the same fields for a caller.

use crate::ask;
use crate::check;
use crate::decide;
use crate::epistemic;
use crate::program::Program;
use crate::syntax::Decl;

/// The one line: the first error, or the decision, the counts and the next question.
pub fn line(decls: &[Decl]) -> String {
    let diagnostics = check::check(decls);
    let errors = diagnostics
        .iter()
        .filter(|item| item.code.starts_with('E'))
        .count();
    let warnings = diagnostics
        .iter()
        .filter(|item| item.code.starts_with('W'))
        .count();
    if errors > 0 {
        let detail = match diagnostics.first() {
            Some(item) => format!(
                "first {} {}: {}",
                item.code,
                item.witness.as_deref().unwrap_or(""),
                item.message
            ),
            None => "no diagnostic rows".to_string(),
        };
        return format!("not checkable; {errors} error(s), {warnings} warning(s); {detail}");
    }
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
    let warning = if warnings > 0 {
        format!(", {warnings} warning(s)")
    } else {
        String::new()
    };
    let question = ask::questions(&Program::new(decls))
        .into_iter()
        .next()
        .map(|item| item.question);
    let next = match question {
        Some(text) => format!("next question — {text}"),
        None => "nothing to ask".to_string(),
    };
    format!(
        "{}; {verified} verified, {inferred} inferred, {unverified} unverified{warning}; {next}",
        verdict.decision.label()
    )
}
