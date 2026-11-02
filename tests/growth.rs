//! The growth gate keeps a library admissible, not merely bigger.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_tsare")
}

fn work(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("tsare-growth-{name}-{}", std::process::id()));
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

const LIBRARY: &str = "tsare 1\nfact F_LIB { claim: \"library fact\", evidence: user }\n";

#[test]
fn a_checked_declaration_enters_and_the_library_still_checks() {
    let dir = work("accept");
    let library = dir.join("library.tsare");
    let source = dir.join("source.tsare");
    let grown = dir.join("grown.tsare");
    fs::write(&library, LIBRARY).expect("library");
    fs::write(
        &source,
        "tsare 1\nfact F_NEW { claim: \"new\", evidence: tool(\"x\") }\n",
    )
    .expect("source");
    let (text, err, status) = run(&[
        "grow",
        library.to_str().expect("library path"),
        source.to_str().expect("source path"),
        "F_NEW",
        "--out",
        grown.to_str().expect("grown path"),
    ]);
    assert_eq!(status, 0, "{text}{err}");
    assert!(text.contains("accepted F_NEW"), "{text}");
    let grown_text = fs::read_to_string(&grown).expect("grown library");
    assert!(grown_text.contains("N_GROWN_F_NEW"), "{grown_text}");
    assert!(grown_text.contains("entered from"), "{grown_text}");
    let (text, err, status) = run(&["check", grown.to_str().expect("grown path")]);
    assert_eq!(status, 0, "{text}{err}");
}

#[test]
fn a_dangling_reference_is_rejected_and_nothing_is_written() {
    let dir = work("reject");
    let library = dir.join("library.tsare");
    let source = dir.join("source.tsare");
    let grown = dir.join("grown.tsare");
    fs::write(&library, LIBRARY).expect("library");
    fs::write(
        &source,
        "tsare 1\nproposition P_NEW { statement: \"s\", from: [F_MISSING] }\n",
    )
    .expect("source");
    let (text, _, status) = run(&[
        "grow",
        library.to_str().expect("library path"),
        source.to_str().expect("source path"),
        "P_NEW",
        "--out",
        grown.to_str().expect("grown path"),
    ]);
    assert_eq!(status, 1, "{text}");
    assert!(text.contains("rejected P_NEW"), "{text}");
    assert!(text.contains("E-UNDEFINED"), "{text}");
    assert!(
        !grown.exists(),
        "a rejected declaration must not be written"
    );
}

#[test]
fn a_note_is_not_a_commitment() {
    let dir = work("note");
    let library = dir.join("library.tsare");
    let source = dir.join("source.tsare");
    let grown = dir.join("grown.tsare");
    fs::write(&library, LIBRARY).expect("library");
    fs::write(
        &source,
        "tsare 1\nnote N_NEW { tag: conjecture, text: \"x\" }\n",
    )
    .expect("source");
    let (text, _, status) = run(&[
        "grow",
        library.to_str().expect("library path"),
        source.to_str().expect("source path"),
        "N_NEW",
        "--out",
        grown.to_str().expect("grown path"),
    ]);
    assert_eq!(status, 1, "{text}");
    assert!(text.contains("not a commitment"), "{text}");
}

#[test]
fn a_missing_out_is_an_invocation_error() {
    let dir = work("usage");
    let library = dir.join("library.tsare");
    let source = dir.join("source.tsare");
    fs::write(&library, LIBRARY).expect("library");
    fs::write(
        &source,
        "tsare 1\nnote N { tag: conjecture, text: \"x\" }\n",
    )
    .expect("source");
    let (text, _, status) = run(&[
        "grow",
        library.to_str().expect("library path"),
        source.to_str().expect("source path"),
        "N",
    ]);
    assert_eq!(status, 2, "{text}");
    assert!(text.contains("usage:"), "{text}");
}
