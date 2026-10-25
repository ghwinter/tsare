from pathlib import Path

p = Path(r"D:\GitHub\engine\rust\engine-core\src\check.rs")
t = p.read_text(encoding="utf-8")

t += r'''

fn field_pos(decl: &Decl, key: &str) -> (usize, usize) {
    for (index, (name, _)) in decl.fields.iter().enumerate() {
        if name == key {
            return decl.field_pos.get(index).copied().unwrap_or((decl.line, decl.col));
        }
    }
    (decl.line, decl.col)
}

fn list_field(decl: &Decl, key: &str) -> Vec<String> {
    field_ids(decl, key)
}

fn sem(decl: &Decl, code: &'static str, message: String, key: Option<&str>, out: &mut Vec<Diagnostic>) {
    let (line, col) = match key {
        Some(name) => field_pos(decl, name),
        None => (decl.line, decl.col),
    };
    out.push(Diagnostic { code, message, line, col });
}

fn check_semantics(decls: &[Decl], out: &mut Vec<Diagnostic>) {
    let warranty_targets: std::collections::HashSet<String> = decls
        .iter()
        .filter(|decl| decl.kind == "warrant" || decl.kind == "proposition")
        .flat_map(|decl| {
            decl.fields
                .iter()
                .filter(|(key, _)| key == "to")
                .flat_map(|(_, value)| ids(value))
        })
        .collect();

    for decl in decls {
        match decl.kind.as_str() {
            "fact" => {
                if let Some(claim) = string_field(decl, "claim") {
                    if claim.trim().is_empty() {
                        sem(decl, "E-EMPTY", format!("fact {} has an empty claim", decl.name), Some("claim"), out);
                    }
                    let universal = ["所有", "都", "全部", "always", "every", "all "];
                    if universal.iter().any(|word| claim.contains(word)) && string_field(decl, "scope").is_none() {
                        sem(
                            decl,
                            "W-OVERCLAIM",
                            format!("fact {} is a universal claim with no scope", decl.name),
                            Some("claim"),
                            out,
                        );
                    }
                }
                if id_field(decl, "evidence") == Some("model") && string_field(decl, "confirmed_by").is_none() {
                    sem(
                        decl,
                        "W-UNCONFIRMED",
                        format!("fact {} was supplied by a model, not by the user or a tool", decl.name),
                        Some("evidence"),
                        out,
                    );
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
                        format!("proposition {} cites premises, but no rule connects them to it", decl.name),
                        Some("from"),
                        out,
                    );
                }
                if decl.fields.iter().any(|(key, _)| key == "confidence") && !warranty_targets.contains(&decl.name) {
                    sem(
                        decl,
                        "W-CONFIDENCE-ASSERTED",
                        format!("proposition {} carries a confidence but no rule computes it", decl.name),
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
                        format!("hypothesis {} assumes nothing, so nothing could refute it", decl.name),
                        Some("assumes"),
                        out,
                    );
                }
                let test = string_field(decl, "test").map(|text| text.trim().to_string()).unwrap_or_default();
                let rival = id_field(decl, "rival");
                if test.is_empty() && rival.is_none() {
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
                if matches!(id_field(decl, "verdict"), Some("supported") | Some("refuted"))
                    && id_field(decl, "verdict_by").is_none()
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
                        format!("warrant {} states a rule, but nothing makes the rule acceptable", decl.name),
                        Some("claim"),
                        out,
                    );
                }
            }
            _ => {}
        }
    }
}
'''

t = t.replace(
    "    check_tension(decls, &mut out);",
    "    check_semantics(decls, &mut out);\n    check_tension(decls, &mut out);",
)

p.write_text(t, encoding="utf-8")
print("semantic checks added")
