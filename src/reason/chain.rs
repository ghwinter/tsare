//! The support path behind a declaration: from a claim to the evidence, link by link.

use std::collections::HashSet;

use crate::program::{f_id, f_ids, grade};
use crate::syntax::Decl;

/// One link in the support tree: a declaration, what it owes, and what supports it.
pub struct Link {
    pub name: String,
    pub kind: String,
    pub detail: String,
    pub children: Vec<Link>,
}

/// Build the support tree for `name`; `Err` when the name is not declared.
pub fn chain(decls: &[Decl], name: &str) -> Result<Link, String> {
    build(decls, name, &mut HashSet::new(), 0)
}

fn build(
    decls: &[Decl],
    name: &str,
    seen: &mut HashSet<String>,
    depth: usize,
) -> Result<Link, String> {
    let Some(decl) = decls.iter().find(|item| item.name == name) else {
        return Err(format!("{name} is not declared"));
    };
    let mut children = Vec::new();
    // A cycle is a diagnostic, not a reason to hang: the path stops where it repeats.
    if depth < 32 && !seen.contains(name) {
        seen.insert(name.to_string());
        for child in supports(decls, decl) {
            if let Ok(link) = build(decls, &child, seen, depth + 1) {
                children.push(link);
            }
        }
        seen.remove(name);
    }
    Ok(Link {
        name: decl.name.clone(),
        kind: decl.kind.clone(),
        detail: detail_of(decl),
        children,
    })
}

fn detail_of(decl: &Decl) -> String {
    match decl.kind.as_str() {
        "fact" => {
            let evidence = decl
                .fields
                .iter()
                .find(|(key, _)| key == "evidence")
                .map(|(_, value)| crate::format::value_of(value))
                .unwrap_or_default();
            format!("{evidence} ({})", grade(decl).label())
        }
        "value" => decl
            .fields
            .iter()
            .find(|(key, _)| key == "measured_by")
            .map(|(_, value)| crate::format::value_of(value))
            .unwrap_or_default(),
        "hypothesis" => match f_id(decl, "verdict_by") {
            Some(verdict) => format!("decided by {verdict}"),
            None => "unbacked".to_string(),
        },
        _ => String::new(),
    }
}

/// The hypotheses on a support path that no verdict fact has decided.
pub fn undecided(link: &Link) -> Vec<String> {
    let mut out = Vec::new();
    walk_undecided(link, &mut out);
    out
}

fn walk_undecided(link: &Link, out: &mut Vec<String>) {
    if link.kind == "hypothesis" && link.detail == "unbacked" {
        out.push(link.name.clone());
    }
    for child in &link.children {
        walk_undecided(child, out);
    }
}

fn supports(decls: &[Decl], decl: &Decl) -> Vec<String> {
    let mut out = Vec::new();
    let mut add = |key: &str| {
        if let Some(id) = f_id(decl, key) {
            out.push(id.to_string());
        }
        out.extend(f_ids(decl, key));
    };
    match decl.kind.as_str() {
        "proposition" => {
            add("from");
            for rule in crate::closure::applications(decls) {
                if rule.target == decl.name && !out.contains(&rule.warrant) {
                    out.push(rule.warrant);
                }
            }
        }
        "warrant" => add("from"),
        "hypothesis" => {
            add("verdict_by");
            add("assumes");
        }
        "concept" => add("grounded_by"),
        "law" => add("checked_by"),
        "partition" => add("basis"),
        "layer" => add("exposes"),
        _ => add("because"),
    }
    out
}
