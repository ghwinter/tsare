use super::attrs::{id_field, num_field, string_field};
use super::*;

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
    let gone = tsare_lang::program::retired(decls);
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
