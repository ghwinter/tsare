//! Regression tests for defects found by checking the documents in `docs/`.
//!
//! Each test names the reproducer in `docs/repro/` and the behaviour the checker owes.  The
//! tests were written before the code they cover: the commit that adds one is the commit that
//! makes it pass, and the test failed on the parent commit.

#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_tsare")
}

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn temp(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("tsare-regression-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("temp dir");
    dir
}

fn run_in(dir: &Path, args: &[&str]) -> Output {
    Command::new(binary())
        .args(args)
        .current_dir(dir)
        .output()
        .expect("run the binary")
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).to_string()
}

fn parse(source: &str) -> Vec<tsare::syntax::Decl> {
    tsare::syntax::parse(source).expect("the reproducer parses")
}

#[test]
fn a_cache_hit_is_the_same_report_as_a_fresh_check() {
    // Defect: cache::restore rebuilt a Diagnostic with `witness: None`, so the second
    // `check --json --cache` reported an empty `name` for every complaint while a plain check
    // reported the declaration.  `name` is the only stable key a delta has.
    let dir = temp("cache-hit");
    let doc = dir.join("doc.tsare");
    std::fs::copy(root().join("docs/repro/engine-order.tsare"), &doc).expect("copy reproducer");
    let run = |extra: &[&str]| {
        let mut args = vec!["check", "doc.tsare", "--json"];
        args.extend_from_slice(extra);
        stdout(&run_in(&dir, &args))
    };
    let fresh = run(&[]);
    let _miss = run(&["--cache"]);
    let hit = run(&["--cache"]);
    assert!(fresh.contains("\"name\": \"P1\""), "{fresh}");
    assert_eq!(fresh, hit, "a cache hit must replay the same report");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_blindspot_signature_is_a_resolved_concept_reference() {
    // Defect: `signature` was missing from check::REF_KEYS, so neither resolution nor kind
    // checking ran on a blind spot's concept list.
    let found = tsare::check::check(&parse(include_str!(
        "../docs/repro/signature-undefined.tsare"
    )));
    assert!(
        found
            .iter()
            .any(|item| item.code == "E-UNDEFINED" && item.message.contains("C_MISSING")),
        "{found:#?}"
    );
    let found = tsare::check::check(&parse(include_str!("../docs/repro/signature-kind.tsare")));
    assert!(
        found
            .iter()
            .any(|item| item.code == "E-KIND" && item.message.contains("F_A")),
        "{found:#?}"
    );
}

#[test]
fn universal_words_need_a_word_boundary() {
    // Defect: the English cues were raw substrings, so "small" contains "all " and "stall" too,
    // while capitalised "All" did not match.  Chinese cues have no spaces and stay substrings.
    let codes = |source: &str| -> Vec<&'static str> {
        tsare::check::check(&parse(source))
            .into_iter()
            .map(|item| item.code)
            .collect()
    };
    assert!(
        !codes(r#"fact F { claim: "the small cache cut latency", evidence: user }"#)
            .contains(&"W-OVERCLAIM"),
        "'small' is not 'all'"
    );
    assert!(
        codes(r#"fact F { claim: "All of the runs passed", evidence: user }"#)
            .contains(&"W-OVERCLAIM"),
        "a capitalised All is still universal"
    );
    assert!(
        codes(r#"fact F { claim: "所有请求都成功", evidence: user }"#).contains(&"W-OVERCLAIM"),
        "所有 is universal"
    );
}
