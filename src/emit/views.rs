//! Five projections of one document: tree, graph, hypergraph, matrix and logic.
//!
//! Each view is a typed JSON object derived from the declarations.  The matrix and the graph can be
//! read back into the attributes and references they came from, which is what makes them views of
//! the same text rather than summaries of it.

use crate::check::{refs, LITERALS, REF_KEYS};
use crate::json::Json as J;
use crate::program::f_ids;
use crate::syntax::{Decl, Val};

fn obj(fields: Vec<(&str, J)>) -> J {
    J::Obj(
        fields
            .into_iter()
            .map(|(key, value)| (key.to_string(), value))
            .collect(),
    )
}

fn strings(items: &[String]) -> J {
    J::Arr(items.iter().map(|item| J::Str(item.clone())).collect())
}

/// The five views of a document, as one typed JSON object.
pub fn json(decls: &[Decl]) -> J {
    obj(vec![
        ("tree", tree(decls)),
        ("graph", graph(decls)),
        ("hypergraph", hypergraph(decls)),
        ("matrix", matrix(decls)),
        ("logic", logic(decls)),
    ])
}

fn tree(decls: &[Decl]) -> J {
    J::Arr(
        decls
            .iter()
            .map(|decl| {
                let fields: Vec<J> = decl
                    .fields
                    .iter()
                    .map(|(key, value)| {
                        obj(vec![
                            ("key", J::Str(key.clone())),
                            ("value", value_json(value)),
                        ])
                    })
                    .collect();
                obj(vec![
                    ("name", J::Str(decl.name.clone())),
                    ("kind", J::Str(decl.kind.clone())),
                    ("fields", J::Arr(fields)),
                ])
            })
            .collect(),
    )
}

fn graph(decls: &[Decl]) -> J {
    let nodes: Vec<J> = decls
        .iter()
        .map(|decl| {
            obj(vec![
                ("name", J::Str(decl.name.clone())),
                ("kind", J::Str(decl.kind.clone())),
            ])
        })
        .collect();
    let mut edges = Vec::new();
    for decl in decls {
        for (key, value) in &decl.fields {
            if !REF_KEYS.contains(&key.as_str()) {
                continue;
            }
            let mut found = Vec::new();
            refs(value, &mut found);
            for target in found {
                if LITERALS.contains(&target.as_str()) {
                    continue;
                }
                edges.push(obj(vec![
                    ("from", J::Str(decl.name.clone())),
                    ("field", J::Str(key.clone())),
                    ("to", J::Str(target)),
                ]));
            }
        }
    }
    obj(vec![("nodes", J::Arr(nodes)), ("edges", J::Arr(edges))])
}

fn hypergraph(decls: &[Decl]) -> J {
    let edges: Vec<J> = decls
        .iter()
        .filter(|decl| decl.kind == "concept")
        .map(|decl| {
            obj(vec![
                ("concept", J::Str(decl.name.clone())),
                ("extent", strings(&f_ids(decl, "grounded_by"))),
            ])
        })
        .collect();
    obj(vec![("hyperedges", J::Arr(edges))])
}

fn matrix(decls: &[Decl]) -> J {
    let mut attributes: Vec<String> = Vec::new();
    let mut rows: Vec<(&Decl, Vec<(String, Val)>)> = Vec::new();
    for decl in decls.iter().filter(|decl| decl.kind == "fact") {
        let Some(Val::Rec(fields)) = decl
            .fields
            .iter()
            .find(|(key, _)| key == "attrs")
            .map(|(_, value)| value)
        else {
            continue;
        };
        for (key, _) in fields {
            if !attributes.contains(key) {
                attributes.push(key.clone());
            }
        }
        rows.push((decl, fields.clone()));
    }
    attributes.sort();
    let rows: Vec<J> = rows
        .iter()
        .map(|(decl, fields)| {
            let values: Vec<J> = fields
                .iter()
                .map(|(key, value)| {
                    obj(vec![
                        ("key", J::Str(key.clone())),
                        ("value", value_json(value)),
                    ])
                })
                .collect();
            obj(vec![
                ("name", J::Str(decl.name.clone())),
                ("values", J::Arr(values)),
            ])
        })
        .collect();
    obj(vec![
        ("attributes", strings(&attributes)),
        ("rows", J::Arr(rows)),
    ])
}

fn logic(decls: &[Decl]) -> J {
    let rules: Vec<J> = decls
        .iter()
        .filter(|decl| matches!(decl.kind.as_str(), "warrant" | "law" | "proposition"))
        .map(|decl| {
            let premises = match decl.kind.as_str() {
                "warrant" => f_ids(decl, "from"),
                "proposition" => f_ids(decl, "from"),
                _ => f_ids(decl, "checked_by"),
            };
            let conclusion = match decl.kind.as_str() {
                "warrant" => f_ids(decl, "to"),
                "law" => f_ids(decl, "holds_in"),
                _ => Vec::new(),
            };
            obj(vec![
                ("name", J::Str(decl.name.clone())),
                ("kind", J::Str(decl.kind.clone())),
                ("premises", strings(&premises)),
                ("conclusion", strings(&conclusion)),
            ])
        })
        .collect();
    obj(vec![("rules", J::Arr(rules))])
}

/// A value as a typed JSON object, so a matrix or tree cell can be read back to the value it was.
pub fn value_json(value: &Val) -> J {
    match value {
        Val::Id(name) => obj(vec![
            ("type", J::Str("id".to_string())),
            ("value", J::Str(name.clone())),
        ]),
        Val::Str(text) => obj(vec![
            ("type", J::Str("str".to_string())),
            ("value", J::Str(text.clone())),
        ]),
        Val::Num(num) => obj(vec![
            ("type", J::Str("num".to_string())),
            ("value", J::Str(num.clone())),
        ]),
        Val::Hole => obj(vec![("type", J::Str("hole".to_string()))]),
        Val::List(items) => obj(vec![
            ("type", J::Str("list".to_string())),
            ("items", J::Arr(items.iter().map(value_json).collect())),
        ]),
        Val::Call(name, args) => obj(vec![
            ("type", J::Str("call".to_string())),
            ("name", J::Str(name.clone())),
            ("args", J::Arr(args.iter().map(value_json).collect())),
        ]),
        Val::Rec(fields) => {
            let fields: Vec<J> = fields
                .iter()
                .map(|(key, value)| {
                    obj(vec![
                        ("key", J::Str(key.clone())),
                        ("value", value_json(value)),
                    ])
                })
                .collect();
            obj(vec![
                ("type", J::Str("rec".to_string())),
                ("fields", J::Arr(fields)),
            ])
        }
    }
}
