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

fn string_list(items: &[&str]) -> J {
    J::Arr(items.iter().map(|item| J::Str(item.to_string())).collect())
}

/// The five names, in output order.
pub const VIEWS: &[&str] = &["tree", "graph", "hypergraph", "matrix", "logic"];

/// What each projection preserves and loses: a representation change declares its fidelity.
pub fn fidelity() -> J {
    let view = |preserves: &[&str], loses: &[&str]| {
        obj(vec![
            ("preserves", string_list(preserves)),
            ("loses", string_list(loses)),
        ])
    };
    obj(vec![
        (
            "tree",
            view(&["declarations", "kinds", "fields", "typed values"], &[]),
        ),
        (
            "graph",
            view(&["references"], &["field values", "claims", "evidence"]),
        ),
        (
            "hypergraph",
            view(
                &["concept extents"],
                &["concept parents", "layers", "extent kinds"],
            ),
        ),
        (
            "matrix",
            view(
                &["facts' attributes"],
                &["claims", "evidence", "references"],
            ),
        ),
        (
            "logic",
            view(
                &["rule premises", "rule conclusions"],
                &["warrant basis", "confidence", "layers"],
            ),
        ),
    ])
}

/// The five views of a document, as one typed JSON object.
pub fn json(decls: &[Decl]) -> J {
    obj(vec![
        ("tree", tree(decls)),
        ("graph", graph(decls)),
        ("hypergraph", hypergraph(decls)),
        ("matrix", matrix(decls)),
        ("logic", logic(decls)),
        ("fidelity", fidelity()),
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

/// Read a typed value back from its JSON form; `None` when the object is not a typed value.
pub fn value_from_json(json: &J) -> Option<Val> {
    match json.get("type").and_then(J::as_str)? {
        "id" => Some(Val::Id(json.get("value").and_then(J::as_str)?.to_string())),
        "str" => Some(Val::Str(json.get("value").and_then(J::as_str)?.to_string())),
        "num" => Some(Val::Num(json.get("value").and_then(J::as_str)?.to_string())),
        "hole" => Some(Val::Hole),
        "list" => {
            let items = json.get("items").and_then(J::as_arr)?;
            let values = items
                .iter()
                .map(value_from_json)
                .collect::<Option<Vec<_>>>()?;
            Some(Val::List(values))
        }
        "call" => {
            let name = json.get("name").and_then(J::as_str)?.to_string();
            let args = json.get("args").and_then(J::as_arr)?;
            let args = args
                .iter()
                .map(value_from_json)
                .collect::<Option<Vec<_>>>()?;
            Some(Val::Call(name, args))
        }
        "rec" => {
            let fields = json.get("fields").and_then(J::as_arr)?;
            let mut out = Vec::new();
            for field in fields {
                let key = field.get("key").and_then(J::as_str)?.to_string();
                let value = value_from_json(field.get("value")?)?;
                out.push((key, value));
            }
            Some(Val::Rec(out))
        }
        _ => None,
    }
}

type TreeRow = (String, String, Vec<(String, Val)>);
type MatrixRow = (String, Val);
type Edge = (String, String, String);
type Extent = (String, Vec<String>);
type Rule = (String, String, Vec<String>, Vec<String>);

/// Serialize the five views, read them back, and compare each to the IR it projects.
///
/// This is the G2 check: a view that cannot be read back is not a view of the text, only a summary.
pub fn check(decls: &[Decl]) -> Vec<String> {
    let text = crate::json::serialize(&json(decls));
    let views = match crate::json::parse(&text) {
        Ok(views) => views,
        Err(_) => return vec!["the views JSON does not parse back".to_string()],
    };
    let mut issues = Vec::new();
    let tree: Vec<TreeRow> = views
        .get("tree")
        .and_then(J::as_arr)
        .map(|items| {
            items
                .iter()
                .filter_map(|decl| {
                    let name = decl.get("name").and_then(J::as_str)?.to_string();
                    let kind = decl.get("kind").and_then(J::as_str)?.to_string();
                    let fields = decl
                        .get("fields")
                        .and_then(J::as_arr)?
                        .iter()
                        .map(|field| {
                            Some((
                                field.get("key").and_then(J::as_str)?.to_string(),
                                value_from_json(field.get("value")?)?,
                            ))
                        })
                        .collect::<Option<Vec<_>>>()?;
                    Some((name, kind, fields))
                })
                .collect()
        })
        .unwrap_or_default();
    let expected_tree: Vec<TreeRow> = decls
        .iter()
        .map(|decl| (decl.name.clone(), decl.kind.clone(), decl.fields.clone()))
        .collect();
    if tree != expected_tree {
        issues.push("tree: the declarations do not read back".to_string());
    }
    let matrix: Vec<MatrixRow> = views
        .get("matrix")
        .and_then(|matrix| matrix.get("rows"))
        .and_then(J::as_arr)
        .map(|rows| {
            rows.iter()
                .filter_map(|row| {
                    let name = row.get("name").and_then(J::as_str)?.to_string();
                    let cells = row
                        .get("values")
                        .and_then(J::as_arr)?
                        .iter()
                        .map(|pair| {
                            Some((
                                pair.get("key").and_then(J::as_str)?.to_string(),
                                value_from_json(pair.get("value")?)?,
                            ))
                        })
                        .collect::<Option<Vec<_>>>()?;
                    Some((name, Val::Rec(cells)))
                })
                .collect()
        })
        .unwrap_or_default();
    let expected_matrix: Vec<MatrixRow> = decls
        .iter()
        .filter(|decl| decl.kind == "fact")
        .filter_map(|decl| {
            let Some(Val::Rec(fields)) = decl
                .fields
                .iter()
                .find(|(key, _)| key == "attrs")
                .map(|(_, value)| value)
            else {
                return None;
            };
            Some((decl.name.clone(), Val::Rec(fields.clone())))
        })
        .collect();
    if matrix != expected_matrix {
        issues.push("matrix: the attributes do not read back".to_string());
    }
    let mut graph: Vec<Edge> = views
        .get("graph")
        .and_then(|graph| graph.get("edges"))
        .and_then(J::as_arr)
        .map(|edges| {
            edges
                .iter()
                .filter_map(|edge| {
                    Some((
                        edge.get("from").and_then(J::as_str)?.to_string(),
                        edge.get("field").and_then(J::as_str)?.to_string(),
                        edge.get("to").and_then(J::as_str)?.to_string(),
                    ))
                })
                .collect()
        })
        .unwrap_or_default();
    graph.sort();
    let mut expected_graph = Vec::new();
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
                expected_graph.push((decl.name.clone(), key.clone(), target));
            }
        }
    }
    expected_graph.sort();
    if graph != expected_graph {
        issues.push("graph: the references do not read back".to_string());
    }
    let hyper: Vec<Extent> = views
        .get("hypergraph")
        .and_then(|hyper| hyper.get("hyperedges"))
        .and_then(J::as_arr)
        .map(|edges| {
            edges
                .iter()
                .filter_map(|edge| {
                    let concept = edge.get("concept").and_then(J::as_str)?.to_string();
                    let extent = edge
                        .get("extent")
                        .and_then(J::as_arr)?
                        .iter()
                        .filter_map(|item| item.as_str().map(str::to_string))
                        .collect();
                    Some((concept, extent))
                })
                .collect()
        })
        .unwrap_or_default();
    let expected_hyper: Vec<Extent> = decls
        .iter()
        .filter(|decl| decl.kind == "concept")
        .map(|decl| (decl.name.clone(), f_ids(decl, "grounded_by")))
        .collect();
    if hyper != expected_hyper {
        issues.push("hypergraph: the extents do not read back".to_string());
    }
    let rules: Vec<Rule> = views
        .get("logic")
        .and_then(|logic| logic.get("rules"))
        .and_then(J::as_arr)
        .map(|rules| {
            rules
                .iter()
                .filter_map(|rule| {
                    let name = rule.get("name").and_then(J::as_str)?.to_string();
                    let kind = rule.get("kind").and_then(J::as_str)?.to_string();
                    let list = |key: &str| -> Option<Vec<String>> {
                        Some(
                            rule.get(key)
                                .and_then(J::as_arr)?
                                .iter()
                                .filter_map(|item| item.as_str().map(str::to_string))
                                .collect(),
                        )
                    };
                    Some((name, kind, list("premises")?, list("conclusion")?))
                })
                .collect()
        })
        .unwrap_or_default();
    let expected_rules: Vec<Rule> = decls
        .iter()
        .filter(|decl| matches!(decl.kind.as_str(), "warrant" | "law" | "proposition"))
        .map(|decl| {
            let premises = match decl.kind.as_str() {
                "warrant" | "proposition" => f_ids(decl, "from"),
                _ => f_ids(decl, "checked_by"),
            };
            let conclusion = match decl.kind.as_str() {
                "warrant" => f_ids(decl, "to"),
                "law" => f_ids(decl, "holds_in"),
                _ => Vec::new(),
            };
            (decl.name.clone(), decl.kind.clone(), premises, conclusion)
        })
        .collect();
    if rules != expected_rules {
        issues.push("logic: the rules do not read back".to_string());
    }
    issues
}
