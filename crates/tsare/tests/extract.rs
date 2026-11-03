//! The extraction pipeline, end to end: a page becomes definitions, the definitions become a
//! document, and the document is checked by the same checker everything else uses.
//!
//! The point is the last step.  An extractor can always produce something; what makes the result a
//! claim is that a second, independent component reads it and either accepts it or says what is
//! wrong.  Here that component is `check`, and the fixture is small enough to count by eye.

use std::path::{Path, PathBuf};
use std::process::Command;

fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_tsare")
}

fn pages() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("pages")
}

fn run(args: &[&str]) -> (String, String, i32) {
    let output = Command::new(binary())
        .args(args)
        .output()
        .expect("run the binary");
    (
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
        output.status.code().unwrap_or(-1),
    )
}

#[test]
fn a_page_yields_the_terms_it_defines() {
    let text = std::fs::read_to_string(pages().join("01-planes.html")).expect("the page");
    let bold = tsare::extract::definitions(&text, "01-planes.html", &[]);
    assert_eq!(bold.len(), 2, "{bold:#?}");
    assert_eq!(bold[0].term, "structure");
    assert_eq!(bold[0].section, "2 The five planes");

    let plain = std::fs::read_to_string(pages().join("02-capabilities.html")).expect("the page");
    assert!(
        tsare::extract::definitions(&plain, "02-capabilities.html", &[]).is_empty(),
        "a header the caller did not name is prose"
    );
    let named = tsare::extract::definitions(&plain, "02-capabilities.html", &["name"]);
    assert_eq!(named.len(), 2, "{named:#?}");
    assert_eq!(named[1].term, "bounded compression");
}

#[test]
fn a_vocabulary_document_names_the_term_columns() {
    let text = std::fs::read_to_string(pages().join("vocabulary.tsare")).expect("the vocabulary");
    let words = tsare::extract::vocabulary(&text, "vocabulary.tsare").expect("it parses");
    assert_eq!(words, vec!["term".to_string(), "name".to_string()]);
    assert!(
        !words
            .iter()
            .any(|word| word == "this note is not part of the vocabulary"),
        "a preference is part of the vocabulary; an intuition is not"
    );
}

#[test]
fn the_extracted_document_compiles() {
    let dir = std::env::temp_dir().join(format!("tsare-extract-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("temp dir");
    let out = dir.join("pages.tsare");
    let (text, err, status) = run(&[
        "extract",
        pages().to_str().expect("pages"),
        "--terms-from",
        pages()
            .join("vocabulary.tsare")
            .to_str()
            .expect("vocabulary"),
        "--write",
        out.to_str().expect("out"),
    ]);
    assert_eq!(status, 0, "{text}{err}");
    // two bold rows and two named-column rows, one numbering across both pages
    assert!(text.contains("4 definition(s)"), "{text}");
    let written = std::fs::read_to_string(&out).expect("the written document");
    assert!(written.starts_with("tsare 1\n"), "{written}");

    let (text, err, status) = run(&["check", out.to_str().expect("out")]);
    assert_eq!(
        status, 0,
        "the extracted document must compile: {text}{err}"
    );
    assert!(text.contains("0 error(s)"), "{text}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_extractor_says_what_it_did_not_read() {
    // A page with tables and no definition is the extractor reporting a decision, not a silence:
    // without a vocabulary the second page has a term column nobody named, and it says so.
    let (text, err, status) = run(&["extract", pages().to_str().expect("pages")]);
    assert_eq!(status, 0, "{text}{err}");
    assert!(text.contains("read 2 page(s), 2 table(s)"), "{text}");
    assert!(
        text.contains("1 page(s) with tables and no definition"),
        "{text}"
    );
    assert!(text.contains("02-capabilities.html"), "{text}");

    // with the vocabulary it reads both, and the count says so
    let (text, err, status) = run(&[
        "extract",
        pages().to_str().expect("pages"),
        "--terms-from",
        pages()
            .join("vocabulary.tsare")
            .to_str()
            .expect("vocabulary"),
    ]);
    assert_eq!(status, 0, "{text}{err}");
    assert!(!text.contains("with tables and no definition"), "{text}");
}
