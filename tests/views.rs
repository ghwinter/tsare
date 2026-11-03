//! Five projections of one document, and the two that read back to the IR.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

use tsare::check::{refs, LITERALS, REF_KEYS};
use tsare::json::{parse, Json};
use tsare::program::f_ids;
use tsare::syntax::{parse as parse_doc, Val};

type TreeRow = (String, String, Vec<(String, Val)>);

fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_tsare")
}

fn work(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("tsare-views-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).expect("temp dir");
    path
}

fn run(args: &[&str]) -> (String, String, i32) {
    let output = Command::new(binary()).args(args).output().expect("run");
    (
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
        output.status.code().unwrap_or(-1),
    )
}

const DOC: &str = "tsare 1\n\
fact F { claim: \"f\", evidence: user, attrs: { items: 1, urgent: 0 } }\n\
fact G { claim: \"g\", evidence: user, attrs: { items: 5 } }\n\
concept C { name: \"c\", grounded_by: [F, G] }\n\
proposition P { statement: \"p\", from: [F] }\n\
warrant W { claim: \"w\", from: [F], to: P, basis: [F] }\n";

fn value_of(json: &Json) -> Val {
    match json.get("type").and_then(Json::as_str).unwrap_or("") {
        "id" => Val::Id(
            json.get("value")
                .and_then(Json::as_str)
                .unwrap_or("")
                .to_string(),
        ),
        "str" => Val::Str(
            json.get("value")
                .and_then(Json::as_str)
                .unwrap_or("")
                .to_string(),
        ),
        "num" => Val::Num(
            json.get("value")
                .and_then(Json::as_str)
                .unwrap_or("")
                .to_string(),
        ),
        "hole" => Val::Hole,
        "list" => Val::List(
            json.get("items")
                .and_then(Json::as_arr)
                .unwrap_or(&[])
                .iter()
                .map(value_of)
                .collect(),
        ),
        "call" => Val::Call(
            json.get("name")
                .and_then(Json::as_str)
                .unwrap_or("")
                .to_string(),
            json.get("args")
                .and_then(Json::as_arr)
                .unwrap_or(&[])
                .iter()
                .map(value_of)
                .collect(),
        ),
        "rec" => Val::Rec(
            json.get("fields")
                .and_then(Json::as_arr)
                .unwrap_or(&[])
                .iter()
                .map(|pair| {
                    (
                        pair.get("key")
                            .and_then(Json::as_str)
                            .unwrap_or("")
                            .to_string(),
                        value_of(pair.get("value").expect("a field value")),
                    )
                })
                .collect(),
        ),
        other => panic!("unknown value type {other}"),
    }
}

#[test]
fn the_matrix_reads_back_to_the_facts_attributes() {
    let dir = work("matrix");
    let path = dir.join("doc.tsare");
    fs::write(&path, DOC).expect("doc");
    let (out, err, status) = run(&["views", path.to_str().expect("path"), "--json"]);
    assert_eq!(status, 0, "{out}{err}");
    let views = parse(&out).expect("views JSON");
    let decls = parse_doc(DOC).expect("the document parses");
    let matrix = views.get("matrix").expect("a matrix");
    let rows = matrix
        .get("rows")
        .and_then(Json::as_arr)
        .expect("matrix rows");
    assert!(!rows.is_empty(), "the facts with attrs have rows: {out}");
    for row in rows {
        let name = row.get("name").and_then(Json::as_str).expect("a row name");
        let cells = row
            .get("values")
            .and_then(Json::as_arr)
            .expect("row values");
        let rec = Val::Rec(
            cells
                .iter()
                .map(|pair| {
                    (
                        pair.get("key")
                            .and_then(Json::as_str)
                            .unwrap_or("")
                            .to_string(),
                        value_of(pair.get("value").expect("a cell value")),
                    )
                })
                .collect(),
        );
        let decl = decls
            .iter()
            .find(|decl| decl.name == name)
            .expect("the row's fact");
        let attrs = decl
            .fields
            .iter()
            .find(|(key, _)| key == "attrs")
            .map(|(_, value)| value)
            .expect("the fact's attrs");
        assert_eq!(&rec, attrs, "the matrix row {name}");
    }
}

#[test]
fn the_graph_reads_back_to_the_references() {
    let dir = work("graph");
    let path = dir.join("doc.tsare");
    fs::write(&path, DOC).expect("doc");
    let (out, err, status) = run(&["views", path.to_str().expect("path"), "--json"]);
    assert_eq!(status, 0, "{out}{err}");
    let views = parse(&out).expect("views JSON");
    let decls = parse_doc(DOC).expect("the document parses");
    let edges = views
        .get("graph")
        .and_then(|graph| graph.get("edges"))
        .and_then(Json::as_arr)
        .expect("graph edges");
    let mut seen: Vec<(String, String, String)> = edges
        .iter()
        .map(|edge| {
            (
                edge.get("from")
                    .and_then(Json::as_str)
                    .unwrap_or("")
                    .to_string(),
                edge.get("field")
                    .and_then(Json::as_str)
                    .unwrap_or("")
                    .to_string(),
                edge.get("to")
                    .and_then(Json::as_str)
                    .unwrap_or("")
                    .to_string(),
            )
        })
        .collect();
    seen.sort();
    let mut expected = Vec::new();
    for decl in &decls {
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
                expected.push((decl.name.clone(), key.clone(), target));
            }
        }
    }
    expected.sort();
    assert_eq!(
        seen, expected,
        "the graph edges do not reconstruct the references"
    );
}

#[test]
fn the_other_three_views_are_present() {
    let dir = work("present");
    let path = dir.join("doc.tsare");
    fs::write(&path, DOC).expect("doc");
    let (out, err, status) = run(&["views", path.to_str().expect("path"), "--json"]);
    assert_eq!(status, 0, "{out}{err}");
    let views = parse(&out).expect("views JSON");
    for name in ["tree", "graph", "hypergraph", "matrix", "logic"] {
        assert!(views.get(name).is_some(), "missing {name} in {out}");
    }
    let rules = views
        .get("logic")
        .and_then(|logic| logic.get("rules"))
        .and_then(Json::as_arr)
        .expect("logic rules");
    assert!(rules
        .iter()
        .any(|rule| rule.get("name").and_then(Json::as_str) == Some("W")));
}

#[test]
fn every_view_declares_its_fidelity() {
    let dir = work("fidelity");
    let path = dir.join("doc.tsare");
    fs::write(&path, DOC).expect("doc");
    let (out, err, status) = run(&["views", path.to_str().expect("path"), "--json"]);
    assert_eq!(status, 0, "{out}{err}");
    let views = parse(&out).expect("views JSON");
    let fidelity = views.get("fidelity").expect("fidelity");
    for name in ["tree", "graph", "hypergraph", "matrix", "logic"] {
        let view = fidelity
            .get(name)
            .unwrap_or_else(|| panic!("no fidelity for {name}"));
        let preserves = view
            .get("preserves")
            .and_then(Json::as_arr)
            .expect("preserves");
        assert!(!preserves.is_empty(), "{name} preserves nothing");
        assert!(
            view.get("loses").and_then(Json::as_arr).is_some(),
            "{name} has no loses list"
        );
    }
}

#[test]
fn one_view_can_be_selected() {
    let dir = work("selected");
    let path = dir.join("doc.tsare");
    fs::write(&path, DOC).expect("doc");
    let (out, err, status) = run(&[
        "views",
        path.to_str().expect("path"),
        "--as",
        "matrix",
        "--json",
    ]);
    assert_eq!(status, 0, "{out}{err}");
    let selected = parse(&out).expect("selected JSON");
    assert_eq!(selected.get("view").and_then(Json::as_str), Some("matrix"));
    assert!(selected
        .get("data")
        .and_then(|data| data.get("rows"))
        .is_some());
    let (out, _, status) = run(&["views", path.to_str().expect("path"), "--as", "nope"]);
    assert_eq!(status, 2, "{out}");
    assert!(out.contains("unknown view"), "{out}");
}

#[test]
fn the_tree_reads_back_to_the_declarations() {
    let dir = work("tree");
    let path = dir.join("doc.tsare");
    fs::write(&path, DOC).expect("doc");
    let (out, err, status) = run(&["views", path.to_str().expect("path"), "--json"]);
    assert_eq!(status, 0, "{out}{err}");
    let views = parse(&out).expect("views JSON");
    let decls = parse_doc(DOC).expect("the document parses");
    let tree = views.get("tree").and_then(Json::as_arr).expect("tree");
    let rebuilt: Vec<TreeRow> = tree
        .iter()
        .map(|decl| {
            let name = decl
                .get("name")
                .and_then(Json::as_str)
                .unwrap_or("")
                .to_string();
            let kind = decl
                .get("kind")
                .and_then(Json::as_str)
                .unwrap_or("")
                .to_string();
            let fields = decl
                .get("fields")
                .and_then(Json::as_arr)
                .unwrap_or(&[])
                .iter()
                .map(|field| {
                    (
                        field
                            .get("key")
                            .and_then(Json::as_str)
                            .unwrap_or("")
                            .to_string(),
                        value_of(field.get("value").expect("a field value")),
                    )
                })
                .collect();
            (name, kind, fields)
        })
        .collect();
    let expected: Vec<TreeRow> = decls
        .iter()
        .map(|decl| (decl.name.clone(), decl.kind.clone(), decl.fields.clone()))
        .collect();
    assert_eq!(
        rebuilt, expected,
        "the tree does not reconstruct the declarations"
    );
}

#[test]
fn the_hypergraph_reads_back_to_the_concept_extents() {
    let dir = work("hypergraph");
    let path = dir.join("doc.tsare");
    fs::write(&path, DOC).expect("doc");
    let (out, err, status) = run(&["views", path.to_str().expect("path"), "--json"]);
    assert_eq!(status, 0, "{out}{err}");
    let views = parse(&out).expect("views JSON");
    let decls = parse_doc(DOC).expect("the document parses");
    let edges = views
        .get("hypergraph")
        .and_then(|hyper| hyper.get("hyperedges"))
        .and_then(Json::as_arr)
        .expect("hyperedges");
    let rebuilt: Vec<(String, Vec<String>)> = edges
        .iter()
        .map(|edge| {
            let concept = edge
                .get("concept")
                .and_then(Json::as_str)
                .unwrap_or("")
                .to_string();
            let extent = edge
                .get("extent")
                .and_then(Json::as_arr)
                .unwrap_or(&[])
                .iter()
                .map(|item| item.as_str().unwrap_or("").to_string())
                .collect();
            (concept, extent)
        })
        .collect();
    let expected: Vec<(String, Vec<String>)> = decls
        .iter()
        .filter(|decl| decl.kind == "concept")
        .map(|decl| (decl.name.clone(), f_ids(decl, "grounded_by")))
        .collect();
    assert_eq!(
        rebuilt, expected,
        "the hypergraph does not reconstruct the extents"
    );
}

#[test]
fn the_logic_reads_back_to_the_rules() {
    let dir = work("logic");
    let path = dir.join("doc.tsare");
    fs::write(&path, DOC).expect("doc");
    let (out, err, status) = run(&["views", path.to_str().expect("path"), "--json"]);
    assert_eq!(status, 0, "{out}{err}");
    let views = parse(&out).expect("views JSON");
    let decls = parse_doc(DOC).expect("the document parses");
    let rules = views
        .get("logic")
        .and_then(|logic| logic.get("rules"))
        .and_then(Json::as_arr)
        .expect("logic rules");
    let rebuilt: Vec<(String, String, Vec<String>, Vec<String>)> = rules
        .iter()
        .map(|rule| {
            let name = rule
                .get("name")
                .and_then(Json::as_str)
                .unwrap_or("")
                .to_string();
            let kind = rule
                .get("kind")
                .and_then(Json::as_str)
                .unwrap_or("")
                .to_string();
            let premises = rule
                .get("premises")
                .and_then(Json::as_arr)
                .unwrap_or(&[])
                .iter()
                .map(|item| item.as_str().unwrap_or("").to_string())
                .collect();
            let conclusion = rule
                .get("conclusion")
                .and_then(Json::as_arr)
                .unwrap_or(&[])
                .iter()
                .map(|item| item.as_str().unwrap_or("").to_string())
                .collect();
            (name, kind, premises, conclusion)
        })
        .collect();
    let expected: Vec<(String, String, Vec<String>, Vec<String>)> = decls
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
    assert_eq!(
        rebuilt, expected,
        "the logic view does not reconstruct the rules"
    );
}

#[test]
fn the_check_verifies_all_round_trips() {
    let dir = work("check");
    let path = dir.join("doc.tsare");
    fs::write(&path, DOC).expect("doc");
    let (out, err, status) = run(&["views", path.to_str().expect("path"), "--check"]);
    assert_eq!(status, 0, "{out}{err}");
    assert!(out.contains("round trips ok"), "{out}");
    let (out, err, status) = run(&["views", path.to_str().expect("path"), "--check", "--json"]);
    assert_eq!(status, 0, "{out}{err}");
    assert!(out.contains("\"ok\": true"), "{out}");
}
