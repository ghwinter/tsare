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
