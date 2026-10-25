//! Record, type, duplicate and reference checks.

use std::collections::{HashMap, HashSet};

use crate::specs::spec;
use crate::syntax::{Decl, Val};

pub const REF_KEYS: &[&str] = &[
    "from", "to", "grounded_by", "against", "uses", "assumes", "answers", "separates",
    "rival", "witness", "basis", "attacks", "checked_by", "holds_in", "exposes",
    "closes_with", "parent", "questions",
];

/// Values that name a procedure rather than a declaration.
pub const LITERALS: &[&str] = &["tool", "doc", "code", "user", "model", "observation", "experiment"];

#[derive(Debug, Clone, PartialEq)]
pub struct Diagnostic {
    pub code: &'static str,
    pub message: String,
    pub line: usize,
    pub col: usize,
}

fn diag(code: &'static str, message: String, decl: &Decl) -> Diagnostic {
    Diagnostic { code, message, line: decl.line, col: decl.col }
}

pub fn refs(value: &Val, out: &mut Vec<String>) {
    match value {
        Val::Id(name) => out.push(name.clone()),
        Val::List(items) => items.iter().for_each(|item| refs(item, out)),
        Val::Call(_, args) => args.iter().for_each(|item| refs(item, out)),
        Val::Rec(fields) => fields.iter().for_each(|(_, item)| refs(item, out)),
        _ => {}
    }
}

fn is_instrument(value: &Val) -> bool {
    match value {
        Val::Str(_) => true,
        Val::Call(name, args) => {
            (name == "tool" || name == "doc") && args.len() == 1 && matches!(args[0], Val::Str(_))
        }
        _ => false,
    }
}

fn is_evidence(value: &Val) -> bool {
    match value {
        Val::Id(name) => ["user", "model", "observation"].contains(&name.as_str()),
        Val::Call(name, args) => {
            (name == "tool" || name == "doc" || name == "code")
                && args.len() == 1
                && matches!(args[0], Val::Str(_))
        }
        _ => false,
    }
}

fn type_ok(ftype: &str, value: &Val) -> bool {
    if matches!(value, Val::Hole) {
        return true;
    }
    match ftype {
        "str" => matches!(value, Val::Str(_)),
        "num" => matches!(value, Val::Num(_)),
        "attrs" | "rec:any" => matches!(value, Val::Rec(_)),
        "instrument" => is_instrument(value),
        "evidence" => is_evidence(value),
        _ if ftype.starts_with("enum:") => match value {
            Val::Id(name) => ftype[5..].split(',').any(|option| option == name),
            _ => false,
        },
        _ if ftype.starts_with("list:") => matches!(value, Val::List(_)),
        _ if ftype.starts_with("ref:") => matches!(value, Val::Id(_)),
        _ => true,
    }
}

fn check_record(decl: &Decl, out: &mut Vec<Diagnostic>) {
    let Some(fields) = spec(&decl.kind) else {
        out.push(diag("E-KIND-UNKNOWN", format!("unknown kind {}", decl.kind), decl));
        return;
    };
    for (index, (key, value)) in decl.fields.iter().enumerate() {
        let (line, col) = decl
            .field_pos
            .get(index)
            .copied()
            .unwrap_or((decl.line, decl.col));
        match fields.iter().find(|(name, _, _)| name == key) {
            Some((_, ftype, _)) => {
                if !type_ok(ftype, value) {
                    out.push(Diagnostic {
                        code: "E-TYPE",
                        message: format!("{}.{key}: expected {ftype}", decl.name),
                        line,
                        col,
                    });
                }
            }
            None => out.push(Diagnostic {
                code: "E-FIELD",
                message: format!("{} has no field {key}", decl.kind),
                line,
                col,
            }),
        }
    }
    for &(name, _, required) in fields {
        if required && !decl.fields.iter().any(|(key, _)| key == name) {
            out.push(diag("E-MISSING", format!("{} needs {name}", decl.name), decl));
        }
    }
}

pub fn check(decls: &[Decl]) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    let mut by_name: HashMap<&str, &str> = HashMap::new();
    for decl in decls {
        if by_name.contains_key(decl.name.as_str()) {
            out.push(diag("E-DUP-ID", format!("{} is declared twice", decl.name), decl));
        } else {
            by_name.insert(decl.name.as_str(), decl.kind.as_str());
        }
    }

    for decl in decls {
        check_record(decl, &mut out);
    }

    check_cycles(decls, &mut out);
    check_collapse(decls, &mut out);
    check_semantics(decls, &mut out);
    check_tension(decls, &mut out);
    check_layer(decls, &mut out);

    let mut seen: HashSet<(String, String)> = HashSet::new();
    for decl in decls {
        for (key, value) in &decl.fields {
            if !REF_KEYS.contains(&key.as_str()) {
                continue;
            }
            let mut found = Vec::new();
            refs(value, &mut found);
            for name in found {
                if LITERALS.contains(&name.as_str()) || by_name.contains_key(name.as_str()) {
                    continue;
                }
                if seen.insert((decl.name.clone(), name.clone())) {
                    out.push(diag("E-UNDEFINED", format!("{}.{key} refers to {name}", decl.name), decl));
                }
            }
        }
    }
    out
}


fn ids(value: &Val) -> Vec<String> {
    let mut out = Vec::new();
    refs(value, &mut out);
    out
}

fn field_ids(decl: &Decl, key: &str) -> Vec<String> {
    decl.fields
        .iter()
        .filter(|(name, _)| name == key)
        .flat_map(|(_, value)| ids(value))
        .collect()
}

fn check_cycles(decls: &[Decl], out: &mut Vec<Diagnostic>) {
    let names: Vec<String> = decls.iter().map(|decl| decl.name.clone()).collect();
    let mut by_name: std::collections::HashMap<&str, &Decl> = std::collections::HashMap::new();
    for decl in decls {
        by_name.insert(decl.name.as_str(), decl);
    }
    let mut edges: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
    for decl in decls {
        let mut targets = Vec::new();
        for (key, value) in &decl.fields {
            if REF_KEYS.contains(&key.as_str()) {
                targets.extend(ids(value));
            }
        }
        targets.retain(|target| names.contains(target));
        edges.insert(decl.name.clone(), targets);
    }

    fn visit(
        node: &str,
        edges: &std::collections::HashMap<String, Vec<String>>,
        by_name: &std::collections::HashMap<&str, &Decl>,
        color: &mut std::collections::HashMap<String, u8>,
        stack: &mut Vec<String>,
        out: &mut Vec<Diagnostic>,
    ) {
        color.insert(node.to_string(), 1);
        stack.push(node.to_string());
        if let Some(targets) = edges.get(node) {
            for target in targets.clone() {
                match color.get(&target).copied().unwrap_or(0) {
                    0 => visit(&target, edges, by_name, color, stack, out),
                    1 => {
                        let position = stack.iter().position(|item| item == &target).unwrap_or(0);
                        let mut cycle = stack[position..].to_vec();
                        cycle.push(target.clone());
                        if let Some(decl) = by_name.get(node) {
                            out.push(diag(
                                "E-CYCLE",
                                format!("dependency cycle: {}", cycle.join(" -> ")),
                                decl,
                            ));
                        }
                    }
                    _ => {}
                }
            }
        }
        stack.pop();
        color.insert(node.to_string(), 2);
    }

    let mut color = std::collections::HashMap::new();
    for decl in decls {
        if color.get(&decl.name).copied().unwrap_or(0) == 0 {
            let mut stack = Vec::new();
            visit(&decl.name, &edges, &by_name, &mut color, &mut stack, out);
        }
    }
}

fn check_collapse(decls: &[Decl], out: &mut Vec<Diagnostic>) {
    let mut seen: std::collections::HashMap<(Vec<String>, Vec<String>), String> =
        std::collections::HashMap::new();
    for decl in decls.iter().filter(|decl| decl.kind == "concept") {
        let mut positives = field_ids(decl, "grounded_by");
        let mut negatives = field_ids(decl, "against");
        positives.sort();
        negatives.sort();
        match seen.get(&(positives.clone(), negatives.clone())) {
            Some(first) => out.push(diag(
                "E-COLLAPSE",
                format!(
                    "concepts {first} and {} have the same extent, so they are one concept with two names",
                    decl.name
                ),
                decl,
            )),
            None => {
                seen.insert((positives, negatives), decl.name.clone());
            }
        }
    }
}


fn string_field<'a>(decl: &'a Decl, key: &str) -> Option<&'a str> {
    decl.fields.iter().find(|(name, _)| name == key).and_then(|(_, value)| match value {
        Val::Str(text) => Some(text.as_str()),
        _ => None,
    })
}

fn id_field<'a>(decl: &'a Decl, key: &str) -> Option<&'a str> {
    decl.fields.iter().find(|(name, _)| name == key).and_then(|(_, value)| match value {
        Val::Id(text) => Some(text.as_str()),
        _ => None,
    })
}

fn num_field(decl: &Decl, key: &str) -> Option<f64> {
    decl.fields.iter().find(|(name, _)| name == key).and_then(|(_, value)| match value {
        Val::Num(text) => text.parse().ok(),
        _ => None,
    })
}

fn check_tension(decls: &[Decl], out: &mut Vec<Diagnostic>) {
    let mut by_metric: std::collections::HashMap<String, Vec<&Decl>> = std::collections::HashMap::new();
    for decl in decls.iter().filter(|decl| decl.kind == "value") {
        if let Some(metric) = string_field(decl, "metric") {
            by_metric.entry(metric.to_string()).or_default().push(decl);
        }
    }
    for (metric, group) in by_metric {
        for (index, first) in group.iter().enumerate() {
            for second in &group[index + 1..] {
                let (Some(left), Some(right)) = (id_field(first, "direction"), id_field(second, "direction")) else {
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

fn check_layer(decls: &[Decl], out: &mut Vec<Diagnostic>) {
    let mut exposes: std::collections::HashMap<&str, Vec<String>> = std::collections::HashMap::new();
    for decl in decls.iter().filter(|decl| decl.kind == "layer") {
        exposes.insert(decl.name.as_str(), field_ids(decl, "exposes"));
    }
    let by_name: std::collections::HashMap<&str, &Decl> =
        decls.iter().map(|decl| (decl.name.as_str(), decl)).collect();

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
