//! A quote anchors a claim to a line of its source, and the checker holds it to that line.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_tsare")
}

fn work(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("tsare-provenance-{name}-{}", std::process::id()));
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
fn a_quote_anchors_the_claim_to_its_line() {
    let dir = work("valid");
    let path = dir.join("doc.tsare");
    fs::write(
        &path,
        "tsare 2\n\
         source S_PAPER { name: \"paper.html\", text: [\"the paper says x\"] }\n\
         fact F { claim: \"x\", evidence: doc(\"paper.html\"), source: S_PAPER, quote: \"the paper says x\" }\n",
    )
    .expect("ledger");
    let (out, err, status) = run(&["check", path.to_str().expect("path")]);
    assert_eq!(status, 0, "{out}{err}");
    let (out, err, status) = run(&["provenance", path.to_str().expect("path")]);
    assert_eq!(status, 0, "{out}{err}");
    assert!(out.contains("S_PAPER"), "{out}");
    assert!(out.contains("1:1"), "{out}");
    let (out, _, status) = run(&["provenance", path.to_str().expect("path"), "--json"]);
    assert_eq!(status, 0, "{out}");
    assert!(out.contains("\"line\": 1"), "{out}");
    assert!(out.contains("\"col\": 1"), "{out}");
}

#[test]
fn a_quote_that_is_not_in_the_source_is_an_error() {
    let dir = work("fabricated");
    let path = dir.join("doc.tsare");
    fs::write(
        &path,
        "tsare 2\n\
         source S_PAPER { name: \"paper.html\", text: [\"the paper says x\"] }\n\
         fact F { claim: \"y\", evidence: doc(\"paper.html\"), source: S_PAPER, quote: \"the paper says y\" }\n",
    )
    .expect("ledger");
    let (out, _, status) = run(&["check", path.to_str().expect("path")]);
    assert_eq!(status, 1, "{out}");
    assert!(out.contains("E-QUOTE"), "{out}");
}

#[test]
fn a_quote_without_a_source_is_an_error() {
    let dir = work("orphan");
    let path = dir.join("doc.tsare");
    fs::write(
        &path,
        "tsare 2\nfact F { claim: \"x\", evidence: user, quote: \"orphan\" }\n",
    )
    .expect("ledger");
    let (out, _, status) = run(&["check", path.to_str().expect("path")]);
    assert_eq!(status, 1, "{out}");
    assert!(out.contains("E-QUOTE-SOURCE"), "{out}");
}
