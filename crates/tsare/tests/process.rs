//! R5's process-level memory: the same steps over different documents reuse each other, and a
//! refused process is recorded and recalled.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_tsare")
}

fn work(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("tsare-process-{name}-{}", std::process::id()));
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

fn signature(store: &Path) -> String {
    let text = fs::read_to_string(store).expect("store");
    let marker = "\"signature\": \"";
    let start = text.find(marker).expect("a signature") + marker.len();
    let rest = &text[start..];
    rest[..rest.find('"').expect("a closing quote")].to_string()
}

const A: &str =
    "tsare 1\nfact F { claim: \"a\", evidence: user }\nfact F { claim: \"a\", evidence: user }\n";
const B: &str =
    "tsare 1\nfact G { claim: \"b\", evidence: user }\nfact G { claim: \"b\", evidence: user }\n";
const C: &str = "tsare 1\nfact F { claim: \"f\", evidence: user }\nfact F { claim: \"f\", evidence: user }\nhypothesis H { assumes: [F], predicts: \"p\", test: \"t\" }\nproposition P { statement: \"s\", from: [H] }\n";

#[test]
fn the_same_process_over_different_documents_is_a_cross_task_reuse() {
    let dir = work("cross");
    let store = dir.join("cases.jsonl");
    let a = dir.join("a.tsare");
    let b = dir.join("b.tsare");
    fs::write(&a, A).expect("a");
    fs::write(&b, B).expect("b");
    let store = store.to_str().expect("store");
    let (out, err, status) = run(&[
        "apply",
        a.to_str().expect("a"),
        "--record",
        store,
        "--action",
        "apply",
    ]);
    assert_eq!(status, 0, "{out}{err}");
    let (out, err, status) = run(&[
        "apply",
        b.to_str().expect("b"),
        "--record",
        store,
        "--action",
        "apply",
    ]);
    assert_eq!(status, 0, "{out}{err}");
    let (text, err, status) = run(&["mem", "--store", store, "--json"]);
    assert_eq!(status, 0, "{text}{err}");
    assert!(text.contains("\"processes\": 1"), "{text}");
    assert!(text.contains("\"cross_task\": 1"), "{text}");
    assert!(text.contains("\"process_reuse\": 1"), "{text}");
    let (text, _, _) = run(&["mem", "--store", store]);
    assert!(text.contains("cross task:  1"), "{text}");
}

#[test]
fn a_refused_process_is_recorded_and_recalled() {
    let dir = work("refused");
    let store = dir.join("cases.jsonl");
    let c = dir.join("c.tsare");
    fs::write(&c, C).expect("c");
    let store_text = store.to_str().expect("store");
    let (out, err, status) = run(&[
        "apply",
        c.to_str().expect("c"),
        "--record",
        store_text,
        "--action",
        "apply",
    ]);
    assert_eq!(status, 1, "{out}{err}");
    assert!(out.contains("refused"), "{out}{err}");
    let (text, err, status) = run(&["mem", "--store", store_text, "--json"]);
    assert_eq!(status, 0, "{text}{err}");
    assert!(text.contains("\"refused\": 1"), "{text}");
    assert!(text.contains("\"vetoed\": 1"), "{text}");
    let sig = signature(&store);
    let (text, err, status) = run(&["mem", "--store", store_text, "--process", &sig]);
    assert_eq!(status, 0, "{text}{err}");
    assert!(text.contains("refused ("), "{text}");
}
