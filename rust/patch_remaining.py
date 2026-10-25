from pathlib import Path

p = Path(r"D:\GitHub\engine\rust\engine-core\src\check.rs")
t = p.read_text(encoding="utf-8")

t += r'''

fn separates(concept: &Decl, left: &str, right: &str) -> bool {
    let positives = list_field(concept, "grounded_by");
    let negatives = list_field(concept, "against");
    let has = |names: &[String], name: &str| names.iter().any(|item| item == name);
    (has(&positives, left) && has(&negatives, right)) || (has(&positives, right) && has(&negatives, left))
}

fn check_remaining(decls: &[Decl], out: &mut Vec<Diagnostic>) {
    let by_name: std::collections::HashMap<&str, &Decl> =
        decls.iter().map(|decl| (decl.name.as_str(), decl)).collect();

    for decl in decls {
        match decl.kind.as_str() {
            "concept" => {
                let positives = list_field(decl, "grounded_by");
                let negatives = list_field(decl, "against");
                if positives.is_empty() {
                    sem(
                        decl,
                        "E-UNGROUNDED",
                        format!("concept {} has no positive example, so it denotes nothing", decl.name),
                        Some("grounded_by"),
                        out,
                    );
                }
                if negatives.is_empty() {
                    sem(
                        decl,
                        "W-UNTESTED",
                        format!("concept {} is never contrasted with a counterexample", decl.name),
                        None,
                        out,
                    );
                }
                let both: Vec<&String> = positives.iter().filter(|name| negatives.contains(name)).collect();
                if !both.is_empty() {
                    let mut names: Vec<&str> = both.iter().map(|name| name.as_str()).collect();
                    names.sort();
                    let listed = names.iter().map(|name| format!("'{name}'")).collect::<Vec<_>>().join(", ");
                    sem(
                        decl,
                        "E-CONTRADICTION",
                        format!("concept {} counts [{listed}] as both positive and negative", decl.name),
                        Some("against"),
                        out,
                    );
                }
                if let Some(parent) = id_field(decl, "parent") {
                    if let Some(upper) = by_name.get(parent) {
                        if upper.kind == "concept" && !positives.is_empty() {
                            let outer = list_field(upper, "grounded_by");
                            let outside: Vec<&String> =
                                positives.iter().filter(|name| !outer.contains(name)).collect();
                            if !outside.is_empty() {
                                let mut names: Vec<&str> = outside.iter().map(|name| name.as_str()).collect();
                                names.sort();
                                let listed = names.iter().map(|name| format!("'{name}'")).collect::<Vec<_>>().join(", ");
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
                        format!("question {} admits fewer than two answers, so it asks nothing", decl.name),
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
                        format!("blindspot {} needs exactly two witnesses, got {}", decl.name, witness.len()),
                        Some("witness"),
                        out,
                    );
                }
                if signature.is_empty() {
                    sem(
                        decl,
                        "E-NO-SIGNATURE",
                        format!("blindspot {} names no signature, so it is not relative to anything", decl.name),
                        Some("signature"),
                        out,
                    );
                }
                if list_field(decl, "closes_with").is_empty() {
                    sem(
                        decl,
                        "W-NO-EXIT",
                        format!("blindspot {} closes no question, so nothing will ever resolve it", decl.name),
                        Some("cannot_express"),
                        out,
                    );
                }
                if witness.len() == 2 {
                    let (left, right) = (&witness[0], &witness[1]);
                    if let (Some(left_decl), Some(right_decl)) = (by_name.get(left.as_str()), by_name.get(right.as_str())) {
                        if left_decl.kind == "fact" && right_decl.kind == "fact" {
                            for name in &signature {
                                if let Some(concept) = by_name.get(name.as_str()) {
                                    if concept.kind == "concept" && separates(concept, left, right) {
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
                        format!("law {} is asserted, but nothing would break if it were false", decl.name),
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
                let blank: Vec<&str> = ["unit"]
                    .into_iter()
                    .filter(|key| {
                        decl.fields.iter().any(|(name, value)| {
                            name == *key
                                && !matches!(value, Val::Hole)
                                && !matches!(value, Val::Str(text) if !text.trim().is_empty())
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
                                    format!("layer {} exposes {name}, which sits in {target_layer}", decl.name),
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
'''

t = t.replace(
    "    check_semantics(decls, &mut out);",
    "    check_semantics(decls, &mut out);\n    check_remaining(decls, &mut out);",
)

p.write_text(t, encoding="utf-8")
print("remaining checks added")
