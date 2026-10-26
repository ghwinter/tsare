//! Render the typed relation net the checker sees.
//!
//! Two documents can have the same prose and different graphs.  The graph is the part a
//! compiler can act on: nodes are declarations, edges are the fields that point at them.

use crate::program::{f_id, f_ids, Program};

pub fn nodes(prog: &Program) -> Vec<String> {
    prog.decls
        .iter()
        .map(|decl| {
            format!(
                "  {}[\"{}<br/><i>{}</i>\"]",
                decl.name, decl.name, decl.kind
            )
        })
        .collect()
}

struct EdgeSet {
    lines: Vec<String>,
    seen: std::collections::HashSet<(String, String, String)>,
}

impl EdgeSet {
    fn add(&mut self, prog: &Program, src: &str, dst: &str, label: &str, dashed: bool) {
        if src.is_empty() || dst.is_empty() {
            return;
        }
        if !prog.has(src) || !prog.has(dst) {
            return;
        }
        let key = (src.to_string(), dst.to_string(), label.to_string());
        if !self.seen.insert(key) {
            return;
        }
        let arrow = if dashed { "-.->" } else { "-->" };
        self.lines.push(format!("  {src} {arrow}|{label}| {dst}"));
    }
}

pub fn edges(prog: &Program) -> Vec<String> {
    let mut set = EdgeSet {
        lines: Vec::new(),
        seen: std::collections::HashSet::new(),
    };
    for decl in prog.decls {
        match decl.kind.as_str() {
            "concept" => {
                for fact in f_ids(decl, "grounded_by") {
                    set.add(prog, &fact, &decl.name, "positive", false);
                }
                for fact in f_ids(decl, "against") {
                    set.add(prog, &fact, &decl.name, "negative", true);
                }
                if let Some(parent) = f_id(decl, "parent") {
                    set.add(prog, &decl.name, parent, "parent", true);
                }
            }
            "proposition" => {
                for concept in f_ids(decl, "uses") {
                    set.add(prog, &decl.name, &concept, "uses", false);
                }
                for premise in f_ids(decl, "from") {
                    set.add(prog, &premise, &decl.name, "from", false);
                }
                if let Some(warrant) = f_id(decl, "warrant") {
                    set.add(prog, warrant, &decl.name, "warrants", false);
                }
            }
            "hypothesis" => {
                for premise in f_ids(decl, "assumes") {
                    set.add(prog, &premise, &decl.name, "assumes", false);
                }
                if let Some(rival) = f_id(decl, "rival") {
                    set.add(prog, &decl.name, rival, "rival", true);
                }
                if let Some(fact) = f_id(decl, "verdict_by") {
                    set.add(prog, fact, &decl.name, "verdict", false);
                }
            }
            "question" => {
                for hypothesis in f_ids(decl, "separates") {
                    set.add(prog, &hypothesis, &decl.name, "separates", false);
                }
            }
            "blindspot" => {
                for witness in f_ids(decl, "witness") {
                    set.add(prog, &witness, &decl.name, "witness", true);
                }
                for concept in f_ids(decl, "signature") {
                    set.add(prog, &concept, &decl.name, "signature", true);
                }
                for question in f_ids(decl, "closes_with") {
                    set.add(prog, &decl.name, &question, "closes with", true);
                }
            }
            "warrant" => {
                for premise in f_ids(decl, "from") {
                    set.add(prog, &premise, &decl.name, "premise", false);
                }
                for basis in f_ids(decl, "basis") {
                    set.add(prog, &basis, &decl.name, "basis", true);
                }
                if let Some(conclusion) = f_id(decl, "to") {
                    set.add(prog, &decl.name, conclusion, "concludes", false);
                }
                for target in f_ids(decl, "attacks") {
                    set.add(prog, &decl.name, &target, "attacks", true);
                }
            }
            _ => {}
        }
    }
    set.lines
}

pub fn to_mermaid(prog: &Program) -> String {
    let mut lines = vec!["graph LR".to_string()];
    lines.extend(nodes(prog));
    lines.extend(edges(prog));
    format!("{}\n", lines.join("\n"))
}

pub fn summary(prog: &Program) -> (usize, usize) {
    (prog.decls.len(), edges(prog).len())
}
