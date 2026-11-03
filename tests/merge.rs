//! A three-way structural merge: one side's change wins, both sides' conflict.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_tsare")
}

fn work(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("tsare-merge-{name}-{}", std::process::id()));
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

const BASE: &str =
    "tsare 1\nfact F { claim: \"x\", evidence: user }\nfact H { claim: \"h\", evidence: user }\n";
const OURS: &str =
    "tsare 1\nfact F { claim: \"y\", evidence: user }\nfact H { claim: \"h\", evidence: user }\n";
const THEIRS: &str = "tsare 1\nfact F { claim: \"x\", evidence: tool(\"t\") }\nfact H { claim: \"h\", evidence: user }\n";

fn files(dir: &Path) -> (PathBuf, PathBuf, PathBuf, PathBuf) {
    let base = dir.join("base.tsare");
    let ours = dir.join("ours.tsare");
    let theirs = dir.join("theirs.tsare");
    let out = dir.join("merged.tsare");
    fs::write(&base, BASE).expect("base");
    fs::write(&ours, OURS).expect("ours");
    fs::write(&theirs, THEIRS).expect("theirs");
    (base, ours, theirs, out)
}

#[test]
fn non_overlapping_field_changes_merge() {
    let dir = work("fields");
    let (base, ours, theirs, out) = files(&dir);
    let (text, err, status) = run(&[
        "merge",
        base.to_str().expect("base path"),
        ours.to_str().expect("ours path"),
        theirs.to_str().expect("theirs path"),
        "--out",
        out.to_str().expect("out path"),
    ]);
    assert_eq!(status, 0, "{text}{err}");
    let merged = fs::read_to_string(&out).expect("merged");
    assert!(merged.contains("claim: \"y\""), "{merged}");
    assert!(merged.contains("evidence: tool(\"t\")"), "{merged}");
    let (checked, checked_err, checked_status) = run(&["check", out.to_str().expect("out path")]);
    assert_eq!(checked_status, 0, "{checked}{checked_err}");
}

#[test]
fn a_both_changed_field_is_a_conflict_and_nothing_is_written() {
    let dir = work("conflict");
    let (base, ours, theirs, out) = files(&dir);
    fs::write(
        &theirs,
        "tsare 1\nfact F { claim: \"z\", evidence: user }\nfact H { claim: \"h\", evidence: user }\n",
    )
    .expect("theirs");
    let (text, _, status) = run(&[
        "merge",
        base.to_str().expect("base path"),
        ours.to_str().expect("ours path"),
        theirs.to_str().expect("theirs path"),
        "--out",
        out.to_str().expect("out path"),
    ]);
    assert_eq!(status, 1, "{text}");
    assert!(text.contains("conflict F claim"), "{text}");
    assert!(!out.exists(), "a conflict must not write");
}

#[test]
fn an_addition_and_a_removal_combine() {
    let dir = work("add-remove");
    let (base, ours, theirs, out) = files(&dir);
    fs::write(
        &ours,
        "tsare 1\nfact F { claim: \"x\", evidence: user }\nfact H { claim: \"h\", evidence: user }\nfact G { claim: \"g\", evidence: user }\n",
    )
    .expect("ours");
    fs::write(
        &theirs,
        "tsare 1\nfact F { claim: \"x\", evidence: user }\n",
    )
    .expect("theirs");
    let (text, err, status) = run(&[
        "merge",
        base.to_str().expect("base path"),
        ours.to_str().expect("ours path"),
        theirs.to_str().expect("theirs path"),
        "--out",
        out.to_str().expect("out path"),
    ]);
    assert_eq!(status, 0, "{text}{err}");
    let merged = fs::read_to_string(&out).expect("merged");
    assert!(merged.contains("fact G"), "{merged}");
    assert!(!merged.contains("fact H"), "{merged}");
    let (checked, checked_err, checked_status) = run(&["check", out.to_str().expect("out path")]);
    assert_eq!(checked_status, 0, "{checked}{checked_err}");
}

#[test]
fn the_json_names_the_conflicts() {
    let dir = work("json");
    let (base, ours, theirs, out) = files(&dir);
    fs::write(
        &theirs,
        "tsare 1\nfact F { claim: \"z\", evidence: user }\nfact H { claim: \"h\", evidence: user }\n",
    )
    .expect("theirs");
    let (text, _, status) = run(&[
        "merge",
        base.to_str().expect("base path"),
        ours.to_str().expect("ours path"),
        theirs.to_str().expect("theirs path"),
        "--out",
        out.to_str().expect("out path"),
        "--json",
    ]);
    assert_eq!(status, 1, "{text}");
    assert!(text.contains("\"merged\": false"), "{text}");
    assert!(text.contains("\"field\": \"claim\""), "{text}");
}
