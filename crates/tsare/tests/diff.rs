//! The semantic difference between two ledgers.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_tsare")
}

fn work(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("tsare-diff-{name}-{}", std::process::id()));
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

const BEFORE: &str =
    "tsare 1\nfact F { claim: \"x\", evidence: user }\nfact H { claim: \"h\", evidence: user }\n";
const AFTER: &str = "tsare 1\nfact F { claim: \"y\", evidence: tool(\"t\") }\nfact G { claim: \"g\", evidence: user }\n";

#[test]
fn the_diff_names_added_removed_and_changed() {
    let dir = work("changed");
    let before = dir.join("before.tsare");
    let after = dir.join("after.tsare");
    fs::write(&before, BEFORE).expect("before");
    fs::write(&after, AFTER).expect("after");
    let (out, err, status) = run(&[
        "diff",
        before.to_str().expect("before path"),
        after.to_str().expect("after path"),
    ]);
    assert_eq!(status, 1, "{out}{err}");
    assert!(out.contains("+ G fact"), "{out}");
    assert!(out.contains("- H fact"), "{out}");
    assert!(out.contains("~ F fact"), "{out}");
    assert!(out.contains("claim: \"x\" -> \"y\""), "{out}");
    assert!(out.contains("evidence: user -> tool(\"t\")"), "{out}");
    assert!(out.contains("diff "), "{out}");
}

#[test]
fn the_same_document_has_no_change() {
    let dir = work("same");
    let before = dir.join("before.tsare");
    fs::write(&before, BEFORE).expect("before");
    let (out, err, status) = run(&[
        "diff",
        before.to_str().expect("before path"),
        before.to_str().expect("before path"),
    ]);
    assert_eq!(status, 0, "{out}{err}");
    assert!(out.contains("no semantic change"), "{out}");
}

#[test]
fn the_json_view_carries_the_changes() {
    let dir = work("json");
    let before = dir.join("before.tsare");
    let after = dir.join("after.tsare");
    fs::write(&before, BEFORE).expect("before");
    fs::write(&after, AFTER).expect("after");
    let (out, _, status) = run(&[
        "diff",
        before.to_str().expect("before path"),
        after.to_str().expect("after path"),
        "--json",
    ]);
    assert_eq!(status, 1, "{out}");
    assert!(out.contains("\"added\""), "{out}");
    assert!(out.contains("\"G\""), "{out}");
    assert!(out.contains("\"removed\""), "{out}");
    assert!(out.contains("\"changed\""), "{out}");
    assert!(out.contains("\"key\": \"claim\""), "{out}");
}

#[test]
fn a_kind_change_is_named() {
    let dir = work("kind");
    let before = dir.join("before.tsare");
    let after = dir.join("after.tsare");
    fs::write(
        &before,
        "tsare 1\nfact K { claim: \"x\", evidence: user }\n",
    )
    .expect("before");
    fs::write(&after, "tsare 1\nnote K { tag: conjecture, text: \"x\" }\n").expect("after");
    let (out, _, status) = run(&[
        "diff",
        before.to_str().expect("before path"),
        after.to_str().expect("after path"),
    ]);
    assert_eq!(status, 1);
    assert!(out.contains("~ K: kind fact -> note"), "{out}");
}
