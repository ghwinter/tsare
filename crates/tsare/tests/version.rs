//! The content-addressed store: save, list, restore.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_tsare")
}

fn work(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("tsare-version-{name}-{}", std::process::id()));
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
fn a_saved_ledger_restores_to_the_same_bytes() {
    let dir = work("roundtrip");
    let doc = dir.join("doc.tsare");
    fs::write(&doc, "tsare 1\nfact F { claim: \"x\", evidence: user }\n").expect("ledger");
    let store = dir.join("store");
    let (out, err, status) = run(&[
        "save",
        doc.to_str().expect("doc path"),
        "--store",
        store.to_str().expect("store path"),
    ]);
    assert_eq!(status, 0, "{out}{err}");
    assert!(out.contains("saved "), "{out}");
    let (out, err, status) = run(&["log", store.to_str().expect("store path")]);
    assert_eq!(status, 0, "{out}{err}");
    let hash = out.lines().next().expect("a hash").to_string();
    assert_eq!(hash.len(), 64);
    let copy = dir.join("copy.tsare");
    let (out, err, status) = run(&[
        "restore",
        &hash,
        "--store",
        store.to_str().expect("store path"),
        "--out",
        copy.to_str().expect("copy path"),
    ]);
    assert_eq!(status, 0, "{out}{err}");
    assert_eq!(
        fs::read(&doc).expect("doc bytes"),
        fs::read(&copy).expect("copy bytes")
    );
    let (out, _, status) = run(&[
        "diff",
        doc.to_str().expect("doc path"),
        copy.to_str().expect("copy path"),
    ]);
    assert_eq!(status, 0, "{out}");
    assert!(out.contains("no semantic change"), "{out}");
}

#[test]
fn a_tampered_store_entry_is_refused() {
    let dir = work("tamper");
    let doc = dir.join("doc.tsare");
    fs::write(&doc, "tsare 1\nfact F { claim: \"x\", evidence: user }\n").expect("ledger");
    let store = dir.join("store");
    let (_, _, status) = run(&[
        "save",
        doc.to_str().expect("doc path"),
        "--store",
        store.to_str().expect("store path"),
    ]);
    assert_eq!(status, 0);
    let (out, _, status) = run(&["log", store.to_str().expect("store path")]);
    assert_eq!(status, 0);
    let hash = out.lines().next().expect("a hash").to_string();
    fs::write(store.join(format!("{hash}.tsare")), "tampered").expect("tamper");
    let copy = dir.join("copy.tsare");
    let (out, _, status) = run(&[
        "restore",
        &hash,
        "--store",
        store.to_str().expect("store path"),
        "--out",
        copy.to_str().expect("copy path"),
    ]);
    assert_eq!(status, 1, "{out}");
    assert!(out.contains("do not match"), "{out}");
}

#[test]
fn an_empty_store_logs_nothing() {
    let dir = work("empty");
    let store = dir.join("store");
    let (out, _, status) = run(&["log", store.to_str().expect("store path")]);
    assert_eq!(status, 0);
    assert!(out.contains("no saved versions"), "{out}");
}
