//! A recorded session is still the same session when it is replayed.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_tsare")
}

fn work(name: &str) -> PathBuf {
    let path =
        std::env::temp_dir().join(format!("tsare-session-trace-{name}-{}", std::process::id()));
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

fn write_ledger(dir: &Path, text: &str) -> PathBuf {
    let path = dir.join("doc.tsare");
    fs::write(&path, text).expect("write ledger");
    path
}

fn record(ledger: &Path, trace: &Path) -> (String, String, i32) {
    run(&[
        "trace",
        ledger.to_str().expect("ledger path"),
        "--out",
        trace.to_str().expect("trace path"),
    ])
}

fn append(ledger: &Path, trace: &Path) -> (String, String, i32) {
    run(&[
        "trace",
        ledger.to_str().expect("ledger path"),
        "--append",
        trace.to_str().expect("trace path"),
    ])
}

const CLEAN: &str = "tsare 1\nfact F { claim: \"x\", evidence: user }\n";
const ERROR: &str = "tsare 1\nfact F { claim: \"x\" }\n";

#[test]
fn a_recorded_run_replays_to_the_same_reports() {
    let dir = work("match");
    let ledger = write_ledger(&dir, CLEAN);
    let trace = dir.join("run.trace.json");
    let (out, err, status) = record(&ledger, &trace);
    assert_eq!(status, 0, "{out}{err}");
    assert!(out.contains("run 1"), "{out}");
    assert!(out.contains("check 0"), "{out}");
    assert!(out.contains("ask 0"), "{out}");
    assert!(out.contains("decide 0"), "{out}");
    let text = fs::read_to_string(&trace).expect("read trace");
    assert!(text.contains("\"tool\":"), "{text}");
    assert!(text.contains("\"runs\":["), "{text}");
    for command in ["check", "ask", "decide"] {
        assert!(
            text.contains(&format!("\"command\":\"{command}\"")),
            "the trace has no {command} step: {text}"
        );
    }
    let (out, err, status) = run(&["replay", trace.to_str().expect("trace path")]);
    assert_eq!(status, 0, "{out}{err}");
    assert_eq!(out, "match\n", "{out}");
}

#[test]
fn a_session_of_runs_replays_in_order() {
    let dir = work("session");
    let ledger = write_ledger(&dir, CLEAN);
    let trace = dir.join("session.trace.json");
    let (out, _, status) = append(&ledger, &trace);
    assert_eq!(status, 0, "{out}");
    assert!(out.contains("runs 1"), "{out}");
    fs::write(&ledger, ERROR).expect("second ledger");
    let (out, _, status) = append(&ledger, &trace);
    assert_eq!(status, 0, "{out}");
    assert!(out.contains("runs 2"), "{out}");
    let text = fs::read_to_string(&trace).expect("read trace");
    assert_eq!(text.matches("\"ledger_sha256\"").count(), 2, "{text}");
    let (out, err, status) = run(&["replay", trace.to_str().expect("trace path")]);
    assert_eq!(status, 0, "{out}{err}");
    assert_eq!(out, "match\n", "{out}");
}

#[test]
fn a_report_that_changed_is_a_mismatch() {
    let dir = work("mismatch");
    let ledger = write_ledger(&dir, CLEAN);
    let trace = dir.join("run.trace.json");
    let (_, _, status) = record(&ledger, &trace);
    assert_eq!(status, 0);
    let text = fs::read_to_string(&trace).expect("read trace");
    let tampered = text.replace("\"report\":\"", "\"report\":\"tampered ");
    assert_ne!(text, tampered, "the trace has a report field");
    fs::write(&trace, tampered).expect("write trace");
    let (out, _, status) = run(&["replay", trace.to_str().expect("trace path")]);
    assert_eq!(status, 1, "{out}");
    assert!(out.starts_with("mismatch: run 1 check"), "{out}");
}

#[test]
fn an_erroring_check_is_recorded_with_its_status() {
    let dir = work("status");
    let ledger = write_ledger(&dir, ERROR);
    let trace = dir.join("run.trace.json");
    let (_, _, status) = record(&ledger, &trace);
    assert_eq!(status, 0);
    let text = fs::read_to_string(&trace).expect("read trace");
    assert!(
        text.contains("\"command\":\"check\",\"status\":1"),
        "{text}"
    );
    let (out, _, status) = run(&["replay", trace.to_str().expect("trace path")]);
    assert_eq!(status, 0);
    assert_eq!(out, "match\n");
}

#[test]
fn a_file_that_is_not_a_trace_is_refused() {
    let dir = work("bad");
    let ledger = write_ledger(&dir, CLEAN);
    let (out, _, status) = run(&["replay", ledger.to_str().expect("ledger path")]);
    assert_eq!(status, 2, "{out}");
    assert!(out.contains("not a tsare trace"), "{out}");
}

#[test]
fn a_ledger_that_does_not_match_its_hash_is_refused() {
    let dir = work("hash");
    let ledger = write_ledger(&dir, CLEAN);
    let trace = dir.join("run.trace.json");
    let (_, _, status) = record(&ledger, &trace);
    assert_eq!(status, 0);
    let text = fs::read_to_string(&trace).expect("read trace");
    let tampered = text.replace("evidence: user", "evidence: model");
    assert_ne!(text, tampered);
    fs::write(&trace, tampered).expect("write trace");
    let (out, _, status) = run(&["replay", trace.to_str().expect("trace path")]);
    assert_eq!(status, 2, "{out}");
    assert!(out.contains("does not match its hash"), "{out}");
}

#[test]
fn out_and_append_together_are_refused() {
    let dir = work("usage");
    let ledger = write_ledger(&dir, CLEAN);
    let trace = dir.join("run.trace.json");
    let (out, _, status) = run(&[
        "trace",
        ledger.to_str().expect("ledger path"),
        "--out",
        trace.to_str().expect("trace path"),
        "--append",
        trace.to_str().expect("trace path"),
    ]);
    assert_eq!(status, 2, "{out}");
    assert!(out.contains("usage:"), "{out}");
}
