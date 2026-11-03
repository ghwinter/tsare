//! The support path behind a declaration: from a claim to the evidence, link by link.

use std::collections::HashSet;

use tsare_lang::json::Json as J;
use tsare_lang::program::{f_id, f_ids, grade};
use tsare_lang::syntax::Decl;

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
                .map(|(_, value)| tsare_lang::format::value_of(value))
                .unwrap_or_default();
            format!("{evidence} ({})", grade(decl).label())
        }
        "value" => decl
            .fields
            .iter()
            .find(|(key, _)| key == "measured_by")
            .map(|(_, value)| tsare_lang::format::value_of(value))
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

/// The conclusions whose support path reaches an unbacked hypothesis.
pub fn open(decls: &[Decl]) -> Vec<(String, String, Vec<String>)> {
    let mut rows = Vec::new();
    for decl in decls {
        if !matches!(decl.kind.as_str(), "proposition" | "warrant") {
            continue;
        }
        if let Ok(link) = chain(decls, &decl.name) {
            let undecided = undecided(&link);
            if !undecided.is_empty() {
                rows.push((decl.name.clone(), decl.kind.clone(), undecided));
            }
        }
    }
    rows
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

/// Every conclusion whose support path reaches an unbacked hypothesis.
pub struct ChainsReport {
    pub rows: Vec<(String, String, Vec<String>)>,
}

/// Read the open chains of a document.
pub fn chains(decls: &[Decl]) -> ChainsReport {
    ChainsReport { rows: open(decls) }
}

impl ChainsReport {
    pub fn text(&self) -> String {
        if self.rows.is_empty() {
            return "no open chains\n".to_string();
        }
        let mut out = String::new();
        for (name, kind, hypotheses) in &self.rows {
            out.push_str(&format!(
                "{name} {kind} stands on the unbacked hypothesis {}\n",
                hypotheses.join(", ")
            ));
        }
        out
    }

    pub fn json(&self) -> J {
        let items: Vec<J> = self
            .rows
            .iter()
            .map(|(name, kind, hypotheses)| {
                J::Obj(vec![
                    ("name".to_string(), J::Str(name.clone())),
                    ("kind".to_string(), J::Str(kind.clone())),
                    (
                        "hypotheses".to_string(),
                        J::Arr(hypotheses.iter().map(|h| J::Str(h.clone())).collect()),
                    ),
                ])
            })
            .collect();
        J::Obj(vec![
            ("count".to_string(), J::Num(self.rows.len() as f64)),
            ("open".to_string(), J::Arr(items)),
        ])
    }
}

impl crate::out::Report for ChainsReport {
    fn text(&self) -> String {
        ChainsReport::text(self)
    }

    fn json(&self) -> J {
        ChainsReport::json(self)
    }
}
