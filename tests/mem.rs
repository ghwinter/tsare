//! The case store's reuse metrics: the process signature, the reuse rate, the orphan and the
//! regression.  R5's measurable form: the memory is addressed by the situation, and the reuse is
//! counted.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_tsare")
}

fn work(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("tsare-mem-{name}-{}", std::process::id()));
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

fn entry(codes: &str, action: &str, before: (i32, i32), after: (i32, i32)) -> String {
    format!(
        "{{\"doc\":\"a.tsare\",\"codes\":[{codes}],\"action\":\"{action}\",\"before\":{{\"errors\":{},\"warnings\":{}}},\"after\":{{\"errors\":{},\"warnings\":{}}},\"checker\":\"test\"}}",
        before.0, before.1, after.0, after.1
    )
}

#[test]
fn the_audit_reports_the_reuse_rate_and_the_orphans() {
    let dir = work("reuse");
    let store = dir.join("cases.jsonl");
    let lines = [
        entry("\"E-1\"", "fix", (2, 0), (0, 0)),
        entry("\"E-1\"", "fix", (3, 0), (1, 0)),
        entry("\"W-2\"", "rename", (0, 2), (0, 1)),
        entry("\"E-3\"", "fix", (0, 1), (1, 2)),
    ];
    fs::write(&store, format!("{}\n", lines.join("\n"))).expect("store");
    let (text, err, status) = run(&["mem", "--store", store.to_str().expect("store"), "--json"]);
    assert_eq!(status, 0, "{text}{err}");
    assert!(text.contains("\"cases\": 4"), "{text}");
    assert!(text.contains("\"signatures\": 3"), "{text}");
    assert!(text.contains("\"reused\": 1"), "{text}");
    assert!(text.contains("\"single\": 2"), "{text}");
    assert!(text.contains("\"regressed\": 1"), "{text}");
    assert!(text.contains("\"reuse_rate\": 0.25"), "{text}");
    let (text, err, status) = run(&["mem", "--store", store.to_str().expect("store")]);
    assert_eq!(status, 0, "{text}{err}");
    assert!(text.contains("reuse rate:  0.25"), "{text}");
    assert!(text.contains("signatures:  3"), "{text}");
    assert!(text.contains("actions:     fix 3, rename 1"), "{text}");
}
