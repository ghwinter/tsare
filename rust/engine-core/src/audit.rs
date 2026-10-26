//! Kind audit: does a kind carry a decompression action of its own, or can it be written
//! with a smaller vocabulary?
//!
//! The test is mechanical.  Rewrite the document without the kind, recompile, and diff the
//! diagnostic codes.  A kind whose checks vanish and are not reproduced in another form is
//! carrying something the smaller language cannot say.
//!
//! It is allowed to conclude "irreducible".

use std::collections::HashSet;

use crate::check::check;
use crate::program::{f_id, f_num, f_str, repr_float};
use crate::syntax::{Decl, Val};

fn codes(decls: &[Decl]) -> HashSet<&'static str> {
    check(decls).iter().map(|item| item.code).collect()
}

fn rebuild(decl: &Decl, kind: &str, name: String, fields: Vec<(String, Val)>) -> Decl {
    let mut copy = decl.clone();
    copy.kind = kind.to_string();
    copy.name = name;
    copy.fields = fields;
    copy
}

pub fn desugar_law(decls: &[Decl]) -> Vec<Decl> {
    let mut out = Vec::new();
    for decl in decls {
        if decl.kind != "law" {
            out.push(decl.clone());
            continue;
        }
        let mut fields: Vec<(String, Val)> = Vec::new();
        if let Some(statement) = decl.fields.iter().find(|(key, _)| key == "statement") {
            fields.push(statement.clone());
        }
        if let Some(checked) = decl.fields.iter().find(|(key, _)| key == "checked_by") {
            if matches!(&checked.1, Val::List(items) if !items.is_empty()) {
                fields.push(("from".to_string(), checked.1.clone()));
            }
        }
        if let Some(layer) = decl.fields.iter().find(|(key, _)| key == "holds_in") {
            fields.push(("layer".to_string(), layer.1.clone()));
        }
        out.push(rebuild(decl, "proposition", decl.name.clone(), fields));
    }
    out
}

/// A value becomes a measurement fact plus a claim about the bound.
pub fn desugar_value(decls: &[Decl]) -> Vec<Decl> {
    let mut out = Vec::new();
    for decl in decls {
        if decl.kind != "value" {
            out.push(decl.clone());
            continue;
        }
        let metric = f_str(decl, "metric").unwrap_or("metric").to_string();
        let direction = f_id(decl, "direction").unwrap_or("min").to_string();
        let unit = f_str(decl, "unit").unwrap_or("?").to_string();
        let threshold = f_num(decl, "threshold");
        let measured = f_str(decl, "measured_by").map(|text| text.to_string());
        let fact_name = format!("{}_MEASURED", decl.name);
        let claim = match &measured {
            Some(command) => format!("{metric} measured by {command}"),
            None => format!("{metric} measured"),
        };
        let evidence = match &measured {
            Some(command) => Val::Call("tool".to_string(), vec![Val::Str(command.clone())]),
            None => Val::Id("model".to_string()),
        };
        let fact = rebuild(
            decl,
            "fact",
            fact_name.clone(),
            vec![("claim".to_string(), Val::Str(claim)), ("evidence".to_string(), evidence)],
        );
        let bound = format!(
            "{metric} should be {direction} {} {unit}",
            threshold.map(repr_float).unwrap_or_else(|| "?".to_string())
        );
        let proposition = rebuild(
            decl,
            "proposition",
            decl.name.clone(),
            vec![
                ("statement".to_string(), Val::Str(bound)),
                (
                    "from".to_string(),
                    Val::List(vec![Val::Id(fact_name)]),
                ),
            ],
        );
        out.push(fact);
        out.push(proposition);
    }
    out
}

/// Not a desugaring: it deletes the visibility structure entirely, including the field.
pub fn drop_layer(decls: &[Decl]) -> Vec<Decl> {
    let mut out = Vec::new();
    for decl in decls {
        if decl.kind == "layer" {
            continue;
        }
        let drop_layer_field = decl.kind == "law";
        if decl.fields.iter().any(|(key, _)| key == "layer" || (drop_layer_field && key == "holds_in")) {
            let fields: Vec<(String, Val)> = decl
                .fields
                .iter()
                .filter(|(key, _)| key != "layer" && !(drop_layer_field && key == "holds_in"))
                .cloned()
                .collect();
            let mut copy = decl.clone();
            copy.fields = fields;
            out.push(copy);
        } else {
            out.push(decl.clone());
        }
    }
    out
}

pub struct Audit {
    pub kind: &'static str,
    pub present: bool,
    pub lost: Vec<&'static str>,
    pub added: Vec<&'static str>,
}

pub fn audit(decls: &[Decl]) -> Vec<Audit> {
    let before = codes(decls);
    let mut rows = Vec::new();
    let rewrites: [(&'static str, fn(&[Decl]) -> Vec<Decl>); 3] =
        [("law", desugar_law), ("value", desugar_value), ("layer", drop_layer)];
    for (kind, rewrite) in rewrites {
        let present = decls.iter().any(|decl| decl.kind == kind);
        if !present {
            rows.push(Audit { kind, present: false, lost: Vec::new(), added: Vec::new() });
            continue;
        }
        let after = codes(&rewrite(decls));
        let mut lost: Vec<&'static str> = before.difference(&after).copied().collect();
        let mut added: Vec<&'static str> = after.difference(&before).copied().collect();
        lost.sort();
        added.sort();
        rows.push(Audit { kind, present: true, lost, added });
    }
    rows
}
