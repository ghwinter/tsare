use super::attrs::{id_field, list_field, num_field, string_field, universal_text};
use super::*;

/// Push a diagnostic about `decl`, anchored at the span of the field `key` names when that field
/// can be found and at the declaration's own span otherwise.
pub(crate) fn sem(
    decl: &Decl,
    code: &'static str,
    message: String,
    key: Option<&str>,
    out: &mut Vec<Diagnostic>,
) {
    let (line, col, end_line, end_col) = match key {
        Some(name) => decl
            .fields
            .iter()
            .position(|(field, _)| field == name)
            .and_then(|index| decl.field_span.get(index).copied())
            .unwrap_or((decl.line, decl.col, decl.end_line, decl.end_col)),
        None => (decl.line, decl.col, decl.end_line, decl.end_col),
    };
    out.push(Diagnostic {
        code,
        message,
        line,
        col,
        end_line,
        end_col,
        file: decl.file.clone(),
        witness: Some(decl.name.clone()),
    });
}

/// Whether a field is carried: present, not a hole, not an empty list.
fn derives(decl: &Decl, key: &str) -> bool {
    match decl
        .fields
        .iter()
        .find(|(name, _)| name == key)
        .map(|(_, value)| value)
    {
        None | Some(Val::Hole) => false,
        Some(Val::List(items)) => !items.is_empty(),
        Some(_) => true,
    }
}

/// What a licence of this kind looks like, in the words the diagnostic uses.
fn criterion_for(kind: &str) -> &'static str {
    match kind {
        "fact" => "a model's opinion is not evidence anyone else can check",
        "proposition" => "it must stand (not `status: open`) and some rule must conclude it",
        "concept" => "it has no positive example",
        "partition" => "it has no elements",
        "layer" => "the layer holds no declaration",
        "law" => "the law is checked by nothing",
        "value" => "it needs a threshold and an instrument",
        "note" => "a note owes nothing, so it cannot discharge anything",
        "hypothesis" => "a hypothesis is not a licence",
        "question" => "a question is not a licence",
        "blindspot" => "a blind spot is not a licence",
        _ => "it is not the kind of thing that licenses an inference",
    }
}

pub(crate) fn is_incomplete(decl: &Decl) -> bool {
    let Some(fields) = spec(&decl.kind) else {
        return true;
    };
    fields
        .iter()
        .any(|&(name, _, required)| required && !decl.fields.iter().any(|(key, _)| key == name))
}

pub(crate) fn check_semantics(decls: &[Decl], out: &mut Vec<Diagnostic>) {
    // A rule connects its premises to a conclusion either by naming it (`to`) or by naming a
    // class it belongs to (`concludes`).  This is the set the fixpoint uses, so the "nothing
    // connects them" warning must use it too -- and a retired rule is not a connection.
    let gone = crate::closure::retired(decls);
    let warranty_targets: std::collections::HashSet<String> = crate::closure::applications(decls)
        .into_iter()
        .filter(|rule| !gone.contains(&rule.warrant))
        .map(|rule| rule.target)
        .collect();

    for decl in decls {
        if is_incomplete(decl) {
            continue;
        }
        match decl.kind.as_str() {
            "fact" => {
                if let Some(claim) = string_field(decl, "claim") {
                    if claim.trim().is_empty() {
                        sem(
                            decl,
                            "E-EMPTY",
                            format!("fact {} has an empty claim", decl.name),
                            Some("claim"),
                            out,
                        );
                    }
                    // "at all" is an idiom, not a universal claim: drop it before looking for
                    // the words that do claim universality
                    if universal_text(claim) && !evidence::carries(decl, "over") {
                        sem(
                            decl,
                            "W-OVERCLAIM",
                            format!(
                                "fact {} is universal and binds no domain: `scope:` is prose, so it cannot be enumerated; bind it with `over:`",
                                decl.name
                            ),
                            Some("claim"),
                            out,
                        );
                    }
                }
                if id_field(decl, "evidence") == Some("model")
                    && string_field(decl, "confirmed_by").is_none()
                {
                    sem(
                        decl,
                        "W-UNCONFIRMED",
                        format!(
                            "fact {} was supplied by a model, not by the user or a tool",
                            decl.name
                        ),
                        Some("evidence"),
                        out,
                    );
                }
                if id_field(decl, "evidence") == Some("model") {
                    if let Some(confidence) = num_field(decl, "confidence") {
                        if confidence >= 0.8 {
                            sem(
                                decl,
                                "W-CONFIDENCE-MODEL",
                                format!(
                                    "fact {} is model-supplied but claims confidence {}",
                                    decl.name, confidence
                                ),
                                Some("confidence"),
                                out,
                            );
                        }
                    }
                }
                if let Some(confidence) = num_field(decl, "confidence") {
                    if !(0.0..=1.0).contains(&confidence) {
                        sem(
                            decl,
                            "E-CONFIDENCE",
                            format!("fact {} claims confidence {confidence}", decl.name),
                            Some("confidence"),
                            out,
                        );
                    }
                }
            }
            "proposition" => {
                let uses = list_field(decl, "uses");
                let from = list_field(decl, "from");
                if uses.is_empty() && from.is_empty() {
                    sem(
                        decl,
                        "E-VACUOUS",
                        format!(
                            "proposition {} names no concept and cites no premise, so it is anchored in nothing",
                            decl.name
                        ),
                        None,
                        out,
                    );
                }
                if id_field(decl, "status") == Some("open") && from.is_empty() {
                    sem(
                        decl,
                        "W-OPEN",
                        format!("proposition {} is open and has no premises", decl.name),
                        None,
                        out,
                    );
                }
                if !from.is_empty() && !warranty_targets.contains(&decl.name) {
                    sem(
                        decl,
                        "W-UNWARRANTED",
                        format!(
                            "proposition {} cites premises, but no rule connects them to it",
                            decl.name
                        ),
                        Some("from"),
                        out,
                    );
                }
                // a proposition has no `scope:` field at all, so a universal one is unbound
                // until it names the domain it ranges over
                if string_field(decl, "statement").is_some_and(universal_text)
                    && !evidence::carries(decl, "over")
                {
                    sem(
                        decl,
                        "W-OVERCLAIM",
                        format!(
                            "proposition {} is universal and binds no domain: add `over:` naming the extent it covers",
                            decl.name
                        ),
                        Some("statement"),
                        out,
                    );
                }
                // `by` is a claim about the inference, so the structure has to support it:
                // a rule makes it deduction, concepts without a rule make it induction.
                if let Some(declared) = id_field(decl, "by") {
                    let concluded = decls.iter().any(|other| {
                        other.kind == "warrant" && id_field(other, "to") == Some(decl.name.as_str())
                    });
                    let explained = decls.iter().any(|other| {
                        other.kind == "hypothesis"
                            && list_field(other, "assumes")
                                .iter()
                                .any(|name| name == &decl.name)
                    });
                    let derived = if concluded {
                        "deductive"
                    } else if !uses.is_empty() {
                        "inductive"
                    } else if explained {
                        "abductive"
                    } else {
                        "none"
                    };
                    if derived != declared {
                        sem(
                            decl,
                            "W-BY-MISMATCH",
                            format!(
                                "proposition {} is declared {declared}, but its structure is {derived}: {}",
                                decl.name,
                                match derived {
                                    "deductive" => "a warrant concludes it, which is deduction",
                                    "inductive" => "it generalises concepts and no rule concludes it",
                                    "abductive" => "a hypothesis takes it as an assumption and no rule concludes it",
                                    _ => "nothing concludes it and it generalises nothing",
                                }
                            ),
                            Some("by"),
                            out,
                        );
                    }
                }
                if decl.fields.iter().any(|(key, _)| key == "confidence")
                    && !warranty_targets.contains(&decl.name)
                {
                    sem(
                        decl,
                        "W-CONFIDENCE-ASSERTED",
                        format!(
                            "proposition {} carries a confidence but no rule computes it",
                            decl.name
                        ),
                        Some("confidence"),
                        out,
                    );
                }
            }
            "hypothesis" => {
                if list_field(decl, "assumes").is_empty() {
                    sem(
                        decl,
                        "E-NO-ASSUMPTION",
                        format!(
                            "hypothesis {} assumes nothing, so nothing could refute it",
                            decl.name
                        ),
                        Some("assumes"),
                        out,
                    );
                }
                let test = string_field(decl, "test")
                    .map(|text| text.trim().to_string())
                    .unwrap_or_default();
                let rival = id_field(decl, "rival");
                // a hypothesis already settled by a fact does not need a test: the evidence
                // that refuted it is the observation.
                let decided = matches!(
                    id_field(decl, "verdict"),
                    Some("supported") | Some("refuted")
                ) && id_field(decl, "verdict_by").is_some();
                if test.is_empty() && rival.is_none() && !decided {
                    sem(
                        decl,
                        "E-UNFALSIFIABLE",
                        format!(
                            "hypothesis {} has neither a test nor a rival, so no observation could refute it",
                            decl.name
                        ),
                        None,
                        out,
                    );
                }
                if rival == Some(decl.name.as_str()) {
                    sem(
                        decl,
                        "E-CYCLE",
                        format!("hypothesis {} is its own rival", decl.name),
                        Some("rival"),
                        out,
                    );
                }
                if matches!(
                    id_field(decl, "verdict"),
                    Some("supported") | Some("refuted")
                ) && id_field(decl, "verdict_by").is_none()
                {
                    sem(
                        decl,
                        "E-NO-EVIDENCE",
                        format!("hypothesis {} is marked but no fact decides it", decl.name),
                        Some("verdict"),
                        out,
                    );
                }
            }
            "warrant" => {
                let premises = list_field(decl, "from");
                let has_to = id_field(decl, "to").is_some();
                let has_concludes = decl
                    .fields
                    .iter()
                    .any(|(key, value)| key == "concludes" && matches!(value, Val::Rec(_)));
                if premises.is_empty() {
                    sem(
                        decl,
                        "E-NO-PREMISE",
                        format!("warrant {} licenses an inference from nothing", decl.name),
                        Some("from"),
                        out,
                    );
                }
                if has_to == has_concludes {
                    sem(
                        decl,
                        "E-WARRANT-MODE",
                        format!("warrant {} must conclude exactly one of a named proposition or a class", decl.name),
                        None,
                        out,
                    );
                }
                if list_field(decl, "basis").is_empty() {
                    sem(
                        decl,
                        "W-UNWARRANTED-BASIS",
                        format!(
                            "warrant {} states a rule, but nothing makes the rule acceptable",
                            decl.name
                        ),
                        Some("basis"),
                        out,
                    );
                }
                // A basis is not "whatever parses": it has to be the kind of thing that licenses an
                // inference.  Each kind has one criterion, and it is the same one the kind owes
                // elsewhere -- a law is checked by something, a value was measured, a proposition
                // stands (not open) and some rule concludes it.
                for cited in list_field(decl, "basis") {
                    let Some(target) = decls.iter().find(|item| item.name == cited) else {
                        continue;
                    };
                    let licensed = match target.kind.as_str() {
                        // a model's word is not evidence, unless a person confirmed it: the same
                        // grade E-SELF-VERDICT uses, so the two checks cannot disagree
                        "fact" => crate::program::grade(target) >= crate::program::Grade::Confirmed,
                        "proposition" => {
                            id_field(target, "status") != Some("open")
                                && warranty_targets.contains(&target.name)
                        }
                        "concept" => !list_field(target, "grounded_by").is_empty(),
                        "partition" => !list_field(target, "elements").is_empty(),
                        "layer" => decls
                            .iter()
                            .any(|item| id_field(item, "layer") == Some(target.name.as_str())),
                        "law" => !list_field(target, "checked_by").is_empty(),
                        "value" => derives(target, "threshold") && derives(target, "measured_by"),
                        _ => false,
                    };
                    if licensed {
                        continue;
                    }
                    sem(
                        decl,
                        "W-BASIS-NO-CRITERION",
                        format!(
                            "warrant {} rests on {cited}, but a {} is not a licence here: {}",
                            decl.name,
                            target.kind,
                            criterion_for(&target.kind)
                        ),
                        Some("basis"),
                        out,
                    );
                }
            }
            _ => {}
        }
    }
}

pub(crate) fn separates(concept: &Decl, left: &str, right: &str) -> bool {
    let positives = list_field(concept, "grounded_by");
    let negatives = list_field(concept, "against");
    let has = |names: &[String], name: &str| names.iter().any(|item| item == name);
    (has(&positives, left) && has(&negatives, right))
        || (has(&positives, right) && has(&negatives, left))
}
