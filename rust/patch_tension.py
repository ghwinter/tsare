from pathlib import Path

p = Path(r"D:\GitHub\engine\rust\engine-core\src\check.rs")
t = p.read_text(encoding="utf-8")

t += r'''

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
                    (Some(a), Some(_)) => format!("both have priority {a:g}"),
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
'''

t = t.replace(
    "    check_cycles(decls, &mut out);\n    check_collapse(decls, &mut out);",
    "    check_cycles(decls, &mut out);\n    check_collapse(decls, &mut out);\n    check_tension(decls, &mut out);\n    check_layer(decls, &mut out);",
)

p.write_text(t, encoding="utf-8")
print("tension and layer added")
