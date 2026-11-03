//! Blame: which version on the store's HEAD chain last changed each field.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_tsare")
}

fn work(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("tsare-blame-{name}-{}", std::process::id()));
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

fn save(store: &Path, doc: &Path) -> String {
    let (out, err, status) = run(&[
        "save",
        doc.to_str().expect("doc path"),
        "--store",
        store.to_str().expect("store path"),
    ]);
    assert_eq!(status, 0, "{out}{err}");
    out.split_whitespace()
        .nth(1)
        .expect("the saved hash")
        .to_string()
}

fn history(store: &Path) -> Vec<String> {
    let (out, err, status) = run(&["log", store.to_str().expect("store path")]);
    assert_eq!(status, 0, "{out}{err}");
    out.lines().map(str::to_string).collect()
}

#[test]
fn each_field_is_charged_to_the_version_that_changed_it() {
    let dir = work("fields");
    let store = dir.join("store");
    let v1 = dir.join("v1.tsare");
    let v2 = dir.join("v2.tsare");
    let v3 = dir.join("v3.tsare");
    fs::write(&v1, "tsare 1\nfact F { claim: \"x\", evidence: user }\n").expect("v1");
    fs::write(&v2, "tsare 1\nfact F { claim: \"y\", evidence: user }\n").expect("v2");
    fs::write(
        &v3,
        "tsare 1\nfact F { claim: \"y\", evidence: tool(\"t\") }\n",
    )
    .expect("v3");
    let h1 = save(&store, &v1);
    let h2 = save(&store, &v2);
    let h3 = save(&store, &v3);
    let (out, err, status) = run(&["blame", store.to_str().expect("store path"), "F"]);
    assert_eq!(status, 0, "{out}{err}");
    let line = |field: &str| {
        out.lines()
            .find(|line| line.trim_start().starts_with(field))
            .unwrap_or_else(|| panic!("no {field} line in {out}"))
    };
    assert!(line("kind").contains(&h1), "kind: {out}");
    assert!(line("claim").contains(&h2), "claim: {out}");
    assert!(line("evidence").contains(&h3), "evidence: {out}");
    let full = history(&store);
    let v2_full = full
        .iter()
        .find(|hash| hash.starts_with(&h2))
        .expect("the v2 hash");
    let (out, _, status) = run(&["blame", store.to_str().expect("store path"), "F", "--json"]);
    assert_eq!(status, 0, "{out}");
    assert!(out.contains("\"field\": \"claim\""), "{out}");
    assert!(
        out.contains(&format!("\"version\": \"{v2_full}\"")),
        "{out}"
    );
}

#[test]
fn an_empty_store_has_no_head() {
    let dir = work("empty");
    let store = dir.join("store");
    let (out, _, status) = run(&["blame", store.to_str().expect("store path"), "F"]);
    assert_eq!(status, 1, "{out}");
    assert!(out.contains("no HEAD"), "{out}");
}

#[test]
fn a_name_that_is_not_in_the_head_is_refused() {
    let dir = work("absent");
    let store = dir.join("store");
    let v1 = dir.join("v1.tsare");
    fs::write(&v1, "tsare 1\nfact F { claim: \"x\", evidence: user }\n").expect("v1");
    save(&store, &v1);
    let (out, _, status) = run(&["blame", store.to_str().expect("store path"), "NOPE"]);
    assert_eq!(status, 1, "{out}");
    assert!(out.contains("not in the head version"), "{out}");
}
