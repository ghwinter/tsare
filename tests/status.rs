//! The loop's one line: decision, counts, question, and the exit code.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_tsare")
}

fn work(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("tsare-status-{name}-{}", std::process::id()));
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

#[test]
fn a_clean_document_reports_the_decision_and_the_counts() {
    let dir = work("clean");
    let path = dir.join("doc.tsare");
    fs::write(&path, "tsare 1\nfact F { claim: \"x\", evidence: user }\n").expect("ledger");
    let (out, err, status) = run(&["status", path.to_str().expect("path")]);
    assert_eq!(status, 0, "{out}{err}");
    assert!(out.starts_with("closed;"), "{out}");
    assert!(out.contains("1 verified"), "{out}");
    assert!(out.contains("nothing to ask"), "{out}");
}

#[test]
fn an_erroring_document_is_not_checkable_and_exits_one() {
    let dir = work("error");
    let path = dir.join("doc.tsare");
    fs::write(&path, "tsare 1\nfact F { claim: \"x\" }\n").expect("ledger");
    let (out, _, status) = run(&["status", path.to_str().expect("path")]);
    assert_eq!(status, 1, "{out}");
    assert!(out.starts_with("not checkable;"), "{out}");
    assert!(out.contains("E-MISSING"), "{out}");
}

#[test]
fn the_json_view_carries_the_same_fields() {
    let dir = work("json");
    let path = dir.join("doc.tsare");
    fs::write(&path, "tsare 1\nfact F { claim: \"x\", evidence: user }\n").expect("ledger");
    let (out, _, status) = run(&["status", path.to_str().expect("path"), "--json"]);
    assert_eq!(status, 0, "{out}");
    assert!(out.contains("\"decision\""), "{out}");
    assert!(out.contains("closed"), "{out}");
    assert!(out.contains("\"verified\""), "{out}");
    assert!(out.contains("\"open_chains\": 0"), "{out}");
    assert!(out.contains("\"open_chain\": null"), "{out}");
    assert!(out.contains("\"errors\""), "{out}");
    assert!(out.contains("\"first\""), "{out}");
    assert!(out.contains("\"claims\""), "{out}");
    assert!(out.contains("\"debt\""), "{out}");
}

#[test]
fn status_appends_a_replayable_run() {
    let dir = work("trace");
    let path = dir.join("doc.tsare");
    fs::write(&path, "tsare 1\nfact F { claim: \"x\", evidence: user }\n").expect("ledger");
    let trace = dir.join("session.trace.json");
    let (out, err, status) = run(&[
        "status",
        path.to_str().expect("path"),
        "--json",
        "--trace",
        trace.to_str().expect("trace path"),
    ]);
    assert_eq!(status, 0, "{out}{err}");
    let text = fs::read_to_string(&trace).expect("trace");
    assert!(text.contains("\"command\":\"status\""), "{text}");
    let (out, err, status) = run(&["replay", trace.to_str().expect("trace path")]);
    assert_eq!(status, 0, "{out}{err}");
    assert_eq!(out, "match\n");
    let (_, _, status) = run(&[
        "status",
        path.to_str().expect("path"),
        "--trace",
        trace.to_str().expect("trace path"),
    ]);
    assert_eq!(status, 0);
    let (out, _, status) = run(&["replay", trace.to_str().expect("trace path")]);
    assert_eq!(status, 0, "{out}");
    assert_eq!(out, "match\n");
}

#[test]
fn the_json_counts_open_chains() {
    let dir = work("open-chains");
    let path = dir.join("doc.tsare");
    fs::write(
        &path,
        "tsare 1\nfact F { claim: \"f\", evidence: user }\nhypothesis H { assumes: [F], predicts: \"p\", test: \"t\" }\nproposition P { statement: \"s\", from: [H] }\n",
    )
    .expect("doc");
    let (out, err, status) = run(&["status", path.to_str().expect("path"), "--json"]);
    assert_eq!(status, 0, "{out}{err}");
    assert!(out.contains("\"open_chains\": 1"), "{out}");
    assert!(out.contains("\"open_chain\": {\"name\": \"P\""), "{out}");
}

#[test]
fn the_line_names_the_first_open_chain() {
    let dir = work("open-line");
    let path = dir.join("doc.tsare");
    fs::write(
        &path,
        "tsare 1\nfact F { claim: \"f\", evidence: user }\nhypothesis H { assumes: [F], predicts: \"p\", test: \"t\" }\nproposition P { statement: \"s\", from: [H] }\n",
    )
    .expect("doc");
    let (out, err, status) = run(&["status", path.to_str().expect("path")]);
    assert_eq!(status, 0, "{out}{err}");
    assert!(out.contains("1 open chain(s) (P on H)"), "{out}");
}
