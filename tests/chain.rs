//! The support path from a declaration to its evidence.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_tsare")
}

fn work(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("tsare-chain-{name}-{}", std::process::id()));
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
fact F { claim: \"f\", evidence: tool(\"t\") }\n\
proposition P { statement: \"p\", from: [F] }\n\
warrant W { claim: \"w\", from: [F], to: P, basis: [F] }\n";

#[test]
fn the_chain_reaches_the_evidence() {
    let dir = work("evidence");
    let path = dir.join("doc.tsare");
    fs::write(&path, DOC).expect("doc");
    let (out, err, status) = run(&["chain", path.to_str().expect("path"), "P"]);
    assert_eq!(status, 0, "{out}{err}");
    assert!(out.contains("proposition P"), "{out}");
    assert!(out.contains("warrant W"), "{out}");
    assert!(out.contains("fact F"), "{out}");
    assert!(out.contains("tool(\"t\") (measured)"), "{out}");
}

#[test]
fn the_json_is_a_tree() {
    let dir = work("json");
    let path = dir.join("doc.tsare");
    fs::write(&path, DOC).expect("doc");
    let (out, err, status) = run(&["chain", path.to_str().expect("path"), "P", "--json"]);
    assert_eq!(status, 0, "{out}{err}");
    assert!(out.contains("\"children\""), "{out}");
    assert!(out.contains("\"W\""), "{out}");
    assert!(out.contains("\"kind\": \"fact\""), "{out}");
}

#[test]
fn a_missing_name_is_an_error() {
    let dir = work("missing");
    let path = dir.join("doc.tsare");
    fs::write(&path, DOC).expect("doc");
    let (out, _, status) = run(&["chain", path.to_str().expect("path"), "NOPE"]);
    assert_eq!(status, 1, "{out}");
    assert!(out.contains("not declared"), "{out}");
}

const OPEN: &str = "tsare 1\n\
fact F { claim: \"f\", evidence: user }\n\
hypothesis H { assumes: [F], predicts: \"p\", test: \"t\" }\n\
proposition P { statement: \"s\", from: [H] }\n";

#[test]
fn the_chains_audit_lists_an_unbacked_hypothesis() {
    let dir = work("chains-open");
    let path = dir.join("doc.tsare");
    fs::write(&path, OPEN).expect("doc");
    let (out, err, status) = run(&["chains", path.to_str().expect("path")]);
    assert_eq!(status, 1, "{out}{err}");
    assert!(
        out.contains("P proposition stands on the unbacked hypothesis H"),
        "{out}"
    );
    let (out, err, status) = run(&["chains", path.to_str().expect("path"), "--json"]);
    assert_eq!(status, 1, "{out}{err}");
    assert!(out.contains("\"count\": 1"), "{out}");
}

#[test]
fn a_closed_document_has_no_open_chains() {
    let dir = work("chains-closed");
    let path = dir.join("doc.tsare");
    fs::write(&path, DOC).expect("doc");
    let (out, err, status) = run(&["chains", path.to_str().expect("path")]);
    assert_eq!(status, 0, "{out}{err}");
    assert!(out.contains("no open chains"), "{out}");
}

#[test]
fn a_write_back_with_an_open_chain_is_refused() {
    let dir = work("apply-gate");
    let path = dir.join("doc.tsare");
    let out = dir.join("fixed.tsare");
    fs::write(
        &path,
        "tsare 1\nfact F { claim: \"same\", evidence: user }\nfact G { claim: \"same\", evidence: user }\nfact H0 { claim: \"f\", evidence: user }\nhypothesis H { assumes: [H0], predicts: \"p\", test: \"t\" }\nproposition P { statement: \"s\", from: [H] }\n",
    )
    .expect("doc");
    let (out_text, err, status) = run(&[
        "fix",
        path.to_str().expect("path"),
        "--apply",
        out.to_str().expect("out"),
    ]);
    assert_eq!(status, 1, "{out_text}{err}");
    let combined = format!("{out_text}{err}");
    assert!(combined.contains("refused"), "{combined}");
    assert!(!out.exists(), "nothing is written on a refusal");
}
