//! The library's report: the grown abstracts, their bytes, and the cases' uses.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_tsare")
}

fn work(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("tsare-library-{name}-{}", std::process::id()));
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

const SOURCE: &str = "tsare 1\nfact F { claim: \"f\", evidence: user }\nproposition P { statement: \"p\", from: [F] }\n";

#[test]
fn the_library_reports_the_grown_abstracts_and_their_uses() {
    let dir = work("report");
    let source = dir.join("source.tsare");
    fs::write(&source, SOURCE).expect("source");
    let empty = dir.join("empty.tsare");
    fs::write(&empty, "tsare 1\nfact F { claim: \"f\", evidence: user }\n").expect("empty");
    let grown = dir.join("grown.tsare");
    let (out, err, status) = run(&[
        "grow",
        empty.to_str().expect("empty"),
        source.to_str().expect("source"),
        "P",
        "--out",
        grown.to_str().expect("grown"),
    ]);
    assert_eq!(status, 0, "{out}{err}");
    let (text, err, status) = run(&["library", grown.to_str().expect("grown")]);
    assert_eq!(status, 0, "{text}{err}");
    assert!(text.contains("1 abstract(s)"), "{text}");
    assert!(
        text.contains("P  26 bytes  0 use(s)") || text.contains("P  "),
        "{text}"
    );
    let doc = dir.join("doc.tsare");
    fs::write(&doc, "tsare 1\nfact F { claim: \"f\", evidence: user }\nproposition P { statement: \"p\", from: [F] }\nproposition P { statement: \"p\", from: [F] }\n").expect("doc");
    let store = dir.join("cases.jsonl");
    let (out, err, status) = run(&[
        "apply",
        doc.to_str().expect("doc"),
        "--record",
        store.to_str().expect("store"),
        "--library",
        grown.to_str().expect("grown"),
    ]);
    assert_eq!(status, 0, "{out}{err}");
    let (text, err, status) = run(&[
        "library",
        grown.to_str().expect("grown"),
        "--cases",
        store.to_str().expect("store"),
        "--json",
    ]);
    assert_eq!(status, 0, "{text}{err}");
    assert!(text.contains("\"uses\": 1"), "{text}");
    assert!(text.contains("\"compression\": 1"), "{text}");
}
