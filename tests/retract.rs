//! Retract: one assumption removed, its dependents enumerated.
//!
//! R6: a withdrawal re-evaluates the document, and the conclusions whose status changed are
//! listed before and after — the stale conclusions are enumerable.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_tsare")
}

fn work(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("tsare-retract-{name}-{}", std::process::id()));
    if dir.exists() {
        fs::remove_dir_all(&dir).expect("clear");
    }
    fs::create_dir_all(&dir).expect("work dir");
    dir
}

fn run(args: &[&str]) -> (String, String, i32) {
    let output = Command::new(binary()).args(args).output().expect("run");
    (
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
        output.status.code().unwrap_or(-1),
    )
}

const DOC: &str = "tsare 1\nfact F { claim: \"f\", evidence: user }\nproposition P { claim: \"p\", from: [F] }\nwarrant W { claim: \"w\", from: [F], to: P, basis: [F] }\n";

#[test]
fn retracting_an_assumption_enumerates_its_dependents() {
    let dir = work("dependents");
    let path = dir.join("doc.tsare");
    let out = dir.join("reduced.tsare");
    fs::write(&path, DOC).expect("doc");
    let (text, err, status) = run(&[
        "retract",
        path.to_str().expect("path"),
        "F",
        "--out",
        out.to_str().expect("out"),
        "--json",
    ]);
    assert_eq!(status, 0, "{text}{err}");
    assert!(text.contains("\"retracted\": \"F\""), "{text}");
    assert!(text.contains("\"name\": \"P\""), "{text}");
    let reduced = fs::read_to_string(&out).expect("reduced");
    assert!(!reduced.contains("fact F"), "{reduced}");
    assert!(reduced.contains("proposition P"), "{reduced}");
}

#[test]
fn retracting_a_proposition_is_refused() {
    let dir = work("refused");
    let path = dir.join("doc.tsare");
    fs::write(&path, DOC).expect("doc");
    let (text, err, status) = run(&["retract", path.to_str().expect("path"), "P"]);
    assert_eq!(status, 1, "{text}{err}");
    assert!(text.contains("refusing"), "{text}");
}
