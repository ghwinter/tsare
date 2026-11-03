use super::*;

pub(crate) fn string_field<'a>(decl: &'a Decl, key: &str) -> Option<&'a str> {
    decl.fields
        .iter()
        .find(|(name, _)| name == key)
        .and_then(|(_, value)| match value {
            Val::Str(text) => Some(text.as_str()),
            _ => None,
        })
}

pub(crate) fn id_field<'a>(decl: &'a Decl, key: &str) -> Option<&'a str> {
    decl.fields
        .iter()
        .find(|(name, _)| name == key)
        .and_then(|(_, value)| match value {
            Val::Id(text) => Some(text.as_str()),
            _ => None,
        })
}

/// Words that make a sentence universal.  This is a surface cue, not the criterion: the criterion
/// is whether the domain is bound with `over:`.  The cue is deliberately generous on `都`, which is
/// also an adverb: a false positive here costs one line (`over:` naming the set, or a reword), and
/// naming the set is the improvement the warning is asking for anyway.
///
/// English has spaces, so an English cue is matched as a word and not as a substring: `all ` used
/// to fire inside `small` and `stall`, while a capitalised `All` did not fire at all.  Chinese has
/// no such boundary; the character cues stay substring tests.
pub(crate) fn universal_text(text: &str) -> bool {
    // "at all" and "after all" are idioms, not universal claims
    let lowered = text
        .to_lowercase()
        .replace("at all", "")
        .replace("after all", "");
    if ["所有", "全部", "每", "都"]
        .iter()
        .any(|word| lowered.contains(word))
    {
        return true;
    }
    lowered
        .split(|ch: char| !ch.is_alphanumeric())
        .any(|token| token == "all" || token == "always" || token.starts_with("every"))
}

fn num_field(decl: &Decl, key: &str) -> Option<f64> {
    decl.fields
        .iter()
        .find(|(name, _)| name == key)
        .and_then(|(_, value)| crate::program::num_of(value))
}

/// Whether two `value` declarations pull one metric in opposite directions and leave the pair
/// unranked.  Opposite directions are a contradiction only when nothing ranks one above the other:
/// unequal priorities record a trade already decided, while equal or missing priorities mean the
/// document asserts both and chooses neither.
pub(crate) fn check_tension(decls: &[Decl], out: &mut Vec<Diagnostic>) {
    let mut by_metric: std::collections::HashMap<String, Vec<&Decl>> =
        std::collections::HashMap::new();
    for decl in decls.iter().filter(|decl| decl.kind == "value") {
        if let Some(metric) = string_field(decl, "metric") {
            by_metric.entry(metric.to_string()).or_default().push(decl);
        }
    }
    for (metric, group) in by_metric {
        for (index, first) in group.iter().enumerate() {
            for second in &group[index + 1..] {
                let (Some(left), Some(right)) =
                    (id_field(first, "direction"), id_field(second, "direction"))
                else {
                    continue;
                };
                if left == right {
                    continue;
                }
                let left_priority = num_field(first, "priority");
                let right_priority = num_field(second, "priority");
                let allowed = match (left_priority, right_priority) {
                    (Some(a), Some(b)) => a != b,
                    _ => false,
                };
                if allowed {
                    continue;
                }
                let why = match (left_priority, right_priority) {
                    (None, None) => "neither has a priority".to_string(),
                    (None, _) | (_, None) => "one of them has no priority".to_string(),
                    (Some(a), Some(_)) => format!("both have priority {}", a),
                };
                out.push(diag(
                    "E-TENSION",
                    format!(
                        "{} and {} constrain '{metric}' in opposite directions ({left} vs {right}) and {why}",
                        first.name, second.name
                    ),
                    second,
                ));
            }
        }
    }
}

/// A law justifies an abstraction only when something at another layer claims to implement it.
/// `layer` names what is hidden and `law` names what is preserved; `refines` is the third
/// condition, and without it the abstraction is a promise nobody signed.
pub(crate) fn check_refinements(decls: &[Decl], out: &mut Vec<Diagnostic>) {
    // A retired law still resolves and no longer holds a position, so it cannot be the signatory
    // that makes a refinement honest.
    let gone = crate::closure::retired(decls);
    let refiners: Vec<&Decl> = decls
        .iter()
        .filter(|decl| {
            decl.kind == "law" && id_field(decl, "refines").is_some() && !gone.contains(&decl.name)
        })
        .collect();
    for decl in decls
        .iter()
        .filter(|decl| decl.kind == "law" && !gone.contains(&decl.name))
    {
        // a law that refines another layer is itself the grounding act: it is anchored to the
        // layer it implements, so it does not need a refiner of its own
        if id_field(decl, "refines").is_some() {
            continue;
        }
        let Some(layer) = id_field(decl, "holds_in") else {
            continue;
        };
        let backed = refiners.iter().any(|other| {
            !std::ptr::eq(*other, decl)
                && id_field(other, "refines") == Some(layer)
                && id_field(other, "holds_in") != Some(layer)
        });
        if backed {
            continue;
        }
        sem(
            decl,
            "W-UNREFINED",
            format!(
                "law {} holds in {layer}, but no law at another layer refines {layer}: the detail \
                 the layer hides is not claimed to be implemented",
                decl.name
            ),
            Some("holds_in"),
            out,
        );
    }
}

/// Whether a declaration reaches into another layer without that layer exposing the target.  A
/// layer is a promise about what it hides, so a name it does not expose is not available outside
/// it, and the reference is a leak even though it resolves.
pub(crate) fn check_layer(decls: &[Decl], out: &mut Vec<Diagnostic>) {
    let mut exposes: std::collections::HashMap<&str, Vec<String>> =
        std::collections::HashMap::new();
    for decl in decls.iter().filter(|decl| decl.kind == "layer") {
        exposes.insert(decl.name.as_str(), field_ids(decl, "exposes"));
    }
    let by_name: std::collections::HashMap<&str, &Decl> = decls
        .iter()
        .map(|decl| (decl.name.as_str(), decl))
        .collect();

    for decl in decls {
        let Some(source) = id_field(decl, "layer") else {
            continue;
        };
        if decl.kind == "layer" {
            continue;
        }
        for (key, value) in &decl.fields {
            if key == "layer" || !REF_KEYS.contains(&key.as_str()) {
                continue;
            }
            for name in ids(value) {
                let Some(target) = by_name.get(name.as_str()) else {
                    continue;
                };
                if target.kind == "layer" {
                    continue;
                }
                let Some(target_layer) = id_field(target, "layer") else {
                    continue;
                };
                if target_layer == source {
                    continue;
                }
                // a layer the document never declares exposes nothing, so an absent entry is not an
                // exemption
                let exposed = exposes
                    .get(target_layer)
                    .map(|names| names.iter().any(|item| item == &name))
                    .unwrap_or(false);
                if !exposed {
                    out.push(diag(
                        "E-LAYER-LEAK",
                        format!(
                            "{} in {source} reaches {name}, which {target_layer} does not expose",
                            decl.name
                        ),
                        decl,
                    ));
                }
            }
        }
    }
}

pub(crate) fn field_pos(decl: &Decl, key: &str) -> (usize, usize) {
    for (index, (name, _)) in decl.fields.iter().enumerate() {
        if name == key {
            return decl
                .field_span
                .get(index)
                .map(|span| (span.0, span.1))
                .unwrap_or((decl.line, decl.col));
        }
    }
    (decl.line, decl.col)
}

pub(crate) fn list_field(decl: &Decl, key: &str) -> Vec<String> {
    field_ids(decl, key)
}

/// The attribute keys a fact records.  Two facts are comparable only over a shared key.
fn attr_keys(decl: &Decl) -> HashSet<String> {
    match decl
        .fields
        .iter()
        .find(|(name, _)| name == "attrs")
        .map(|(_, value)| value)
    {
        Some(Val::Rec(fields)) => fields.iter().map(|(key, _)| key.clone()).collect(),
        _ => HashSet::new(),
    }
}

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

fn is_incomplete(decl: &Decl) -> bool {
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

fn separates(concept: &Decl, left: &str, right: &str) -> bool {
    let positives = list_field(concept, "grounded_by");
    let negatives = list_field(concept, "against");
    let has = |names: &[String], name: &str| names.iter().any(|item| item == name);
    (has(&positives, left) && has(&negatives, right))
        || (has(&positives, right) && has(&negatives, left))
}

pub(crate) fn check_remaining(decls: &[Decl], out: &mut Vec<Diagnostic>) {
    let by_name: std::collections::HashMap<&str, &Decl> = decls
        .iter()
        .map(|decl| (decl.name.as_str(), decl))
        .collect();

    for decl in decls {
        if is_incomplete(decl) {
            continue;
        }
        match decl.kind.as_str() {
            "partition" => partition::check_partition(decl, decls, &by_name, out),
            "concept" => {
                let positives = list_field(decl, "grounded_by");
                let negatives = list_field(decl, "against");
                if positives.is_empty() {
                    sem(
                        decl,
                        "E-UNGROUNDED",
                        format!(
                            "concept {} has no positive example, so it denotes nothing",
                            decl.name
                        ),
                        Some("grounded_by"),
                        out,
                    );
                }
                if negatives.is_empty() {
                    sem(
                        decl,
                        "W-UNTESTED",
                        format!(
                            "concept {} is never contrasted with a counterexample",
                            decl.name
                        ),
                        None,
                        out,
                    );
                }

                // An example is only an example if it can be compared with the others: it
                // must share at least one recorded attribute with the positives.  Without
                // this, the learner reports "identical attributes" and nobody knows why.
                let positives_share: HashSet<String> = positives
                    .iter()
                    .filter_map(|name| by_name.get(name.as_str()))
                    .flat_map(|fact| attr_keys(fact))
                    .collect();
                if !positives_share.is_empty() {
                    for example in positives.iter().chain(negatives.iter()) {
                        let Some(fact) = by_name.get(example.as_str()) else {
                            continue;
                        };
                        let keys = attr_keys(fact);
                        if !keys.is_empty() && keys.intersection(&positives_share).next().is_none()
                        {
                            sem(
                                decl,
                                "W-EXAMPLE-SHAPE",
                                format!(
                                    "concept {}: {example} shares no recorded attribute with the positives, so it cannot be compared with them",
                                    decl.name
                                ),
                                None,
                                out,
                            );
                        }
                    }
                }
                let both: Vec<&String> = positives
                    .iter()
                    .filter(|name| negatives.contains(name))
                    .collect();
                if !both.is_empty() {
                    let mut names: Vec<&str> = both.iter().map(|name| name.as_str()).collect();
                    names.sort();
                    let listed = names
                        .iter()
                        .map(|name| format!("'{name}'"))
                        .collect::<Vec<_>>()
                        .join(", ");
                    sem(
                        decl,
                        "E-CONTRADICTION",
                        format!(
                            "concept {} counts [{listed}] as both positive and negative",
                            decl.name
                        ),
                        Some("against"),
                        out,
                    );
                }
                if let Some(parent) = id_field(decl, "parent") {
                    if let Some(upper) = by_name.get(parent) {
                        if upper.kind == "concept" && !positives.is_empty() {
                            let outer = list_field(upper, "grounded_by");
                            let outside: Vec<&String> = positives
                                .iter()
                                .filter(|name| !outer.contains(name))
                                .collect();
                            if !outside.is_empty() {
                                let mut names: Vec<&str> =
                                    outside.iter().map(|name| name.as_str()).collect();
                                names.sort();
                                let listed = names
                                    .iter()
                                    .map(|name| format!("'{name}'"))
                                    .collect::<Vec<_>>()
                                    .join(", ");
                                sem(
                                    decl,
                                    "E-HIERARCHY",
                                    format!("{} is declared under {parent}, but [{listed}] lie outside {parent}", decl.name),
                                    Some("parent"),
                                    out,
                                );
                            }
                        }
                    }
                }
            }
            "question" => {
                let answers: Vec<String> = decl
                    .fields
                    .iter()
                    .filter(|(key, _)| key == "answers")
                    .flat_map(|(_, value)| match value {
                        Val::List(items) => items
                            .iter()
                            .filter_map(|item| match item {
                                Val::Str(text) => Some(text.clone()),
                                _ => None,
                            })
                            .collect::<Vec<_>>(),
                        _ => Vec::new(),
                    })
                    .collect();
                let separates_list = list_field(decl, "separates");
                if answers.len() < 2 {
                    sem(
                        decl,
                        "E-NO-ANSWERS",
                        format!(
                            "question {} admits fewer than two answers, so it asks nothing",
                            decl.name
                        ),
                        Some("answers"),
                        out,
                    );
                }
                if separates_list.len() < 2 {
                    sem(
                        decl,
                        "E-NON-DISCRIMINATING",
                        format!("question {} does not separate two hypotheses", decl.name),
                        Some("separates"),
                        out,
                    );
                } else if answers.len() < separates_list.len() {
                    sem(
                        decl,
                        "W-COARSE",
                        format!(
                            "question {} has {} answers for {} hypotheses",
                            decl.name,
                            answers.len(),
                            separates_list.len()
                        ),
                        None,
                        out,
                    );
                }
            }
            "blindspot" => {
                let witness = list_field(decl, "witness");
                let signature = list_field(decl, "signature");
                if witness.len() != 2 {
                    sem(
                        decl,
                        "E-WITNESS",
                        format!(
                            "blindspot {} needs exactly two witnesses, got {}",
                            decl.name,
                            witness.len()
                        ),
                        Some("witness"),
                        out,
                    );
                }
                if signature.is_empty() {
                    sem(
                        decl,
                        "E-NO-SIGNATURE",
                        format!(
                            "blindspot {} names no signature, so it is not relative to anything",
                            decl.name
                        ),
                        Some("signature"),
                        out,
                    );
                }
                if list_field(decl, "closes_with").is_empty() {
                    sem(
                        decl,
                        "W-NO-EXIT",
                        format!(
                            "blindspot {} closes no question, so nothing will ever resolve it",
                            decl.name
                        ),
                        Some("cannot_express"),
                        out,
                    );
                }
                if witness.len() == 2 {
                    let (left, right) = (&witness[0], &witness[1]);
                    if let (Some(left_decl), Some(right_decl)) =
                        (by_name.get(left.as_str()), by_name.get(right.as_str()))
                    {
                        if left_decl.kind == "fact" && right_decl.kind == "fact" {
                            for name in &signature {
                                if let Some(concept) = by_name.get(name.as_str()) {
                                    if concept.kind == "concept" && separates(concept, left, right)
                                    {
                                        sem(
                                            decl,
                                            "E-FALSE-BLINDSPOT",
                                            format!(
                                                "{name} separates {left} from {right}, so {} is not a blind spot",
                                                decl.name
                                            ),
                                            Some("witness"),
                                            out,
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            }
            "law" => {
                if list_field(decl, "checked_by").is_empty() {
                    sem(
                        decl,
                        "W-LAW-UNCHECKED",
                        format!(
                            "law {} is asserted, but nothing would break if it were false",
                            decl.name
                        ),
                        Some("statement"),
                        out,
                    );
                }
            }
            "value" => {
                let missing: Vec<&str> = ["unit", "threshold"]
                    .into_iter()
                    .filter(|key| !decl.fields.iter().any(|(name, _)| name == *key))
                    .collect();
                // A wrong-typed unit is E-TYPE's business; "lacks unit" is reserved for a unit
                // that is present and blank.  Counting a number or a hole as blank produced two
                // diagnostics for one fault, and the second one said something untrue.
                let blank: Vec<&str> = ["unit"]
                    .into_iter()
                    .filter(|key| {
                        decl.fields.iter().any(|(name, value)| {
                            name == *key
                                && matches!(value, Val::Str(text) if text.trim().is_empty())
                        })
                    })
                    .collect();
                let lacking: Vec<&str> = missing.into_iter().chain(blank).collect();
                if !lacking.is_empty() {
                    sem(
                        decl,
                        "E-INCOMMENSURABLE",
                        format!(
                            "value {} lacks {}, so it cannot be compared with anything",
                            decl.name,
                            lacking.join(", ")
                        ),
                        None,
                        out,
                    );
                }
            }
            "layer" => {
                for name in list_field(decl, "exposes") {
                    if let Some(target) = by_name.get(name.as_str()) {
                        if let Some(target_layer) = id_field(target, "layer") {
                            if target_layer != decl.name {
                                sem(
                                    decl,
                                    "W-EXPOSE-FOREIGN",
                                    format!(
                                        "layer {} exposes {name}, which sits in {target_layer}",
                                        decl.name
                                    ),
                                    Some("exposes"),
                                    out,
                                );
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }
}
