//! The case store travels: a store recorded in one directory recalls in another.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_tsare")
}

fn work(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("tsare-portable-{name}-{}", std::process::id()));
    if dir.exists() {
        fs::remove_dir_all(&dir).expect("clear");
    }
    fs::create_dir_all(&dir).expect("work dir");
    dir
}

fn run(args: &[&str], cwd: &Path) -> (String, String, i32) {
    let output = Command::new(binary())
        .args(args)
        .current_dir(cwd)
        .output()
        .expect("run");
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

#[test]
fn the_store_travels_to_another_directory_and_still_recalls() {
    let dir = work("travel");
    let a = dir.join("a");
    let b = dir.join("b");
    fs::create_dir_all(&a).expect("a");
    fs::create_dir_all(&b).expect("b");
    let doc = a.join("doc.tsare");
    fs::write(&doc, "tsare 1\nfact F { claim: \"a\", evidence: user }\nfact F { claim: \"a\", evidence: user }\n").expect("doc");
    let store = a.join("cases.jsonl");
    let (out, err, status) = run(
        &[
            "apply",
            doc.to_str().expect("doc"),
            "--record",
            store.to_str().expect("store"),
        ],
        &a,
    );
    assert_eq!(status, 0, "{out}{err}");
    let moved = b.join("cases.jsonl");
    fs::copy(&store, &moved).expect("copy");
    fs::remove_file(&doc).expect("remove the original document");
    let (text, err, status) = run(
        &["mem", "--store", moved.to_str().expect("moved"), "--json"],
        &b,
    );
    assert_eq!(status, 0, "{text}{err}");
    assert!(text.contains("\"cases\": 1"), "{text}");
    assert!(text.contains("\"expired\": 0"), "{text}");
    assert!(text.contains("\"orphans\": 1"), "{text}");
    let sig = signature(&moved);
    let (text, err, status) = run(
        &[
            "mem",
            "--store",
            moved.to_str().expect("moved"),
            "--process",
            &sig,
        ],
        &b,
    );
    assert_eq!(status, 0, "{text}{err}");
    assert!(
        text.contains("improved") || text.contains("refused"),
        "{text}"
    );
}
