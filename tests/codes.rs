//! Every diagnostic has a document that turns it on.
//!
//! A code nothing can produce is a code nobody can rely on: it may be unreachable, its wording may
//! have gone stale, or it may fire only in shapes we stopped writing.  This is the check the
//! engine owes itself -- the `F_CHECKS` fact in `SELF.tsare` says every diagnostic has an explanation,
//! and this file is what makes that claim mean something.
//!
//! Three sources, because not every code can come from one document:
//!
//! 1. `tests/codes/*.tsare` -- the smallest single document per code;
//! 2. `tests/fixtures/golden/*.txt` -- recorded output of the binary, which covers the codes that
//!    only a two-document command or a goal file produces;
//! 3. parsing and the kernel, asserted directly below because they are not `check`.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use tsare::check::check;
use tsare::syntax::parse;

fn corpus() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("codes")
}

fn codes_in(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for (open, close) in [("warning[", "]"), ("error[", "]")] {
        let mut rest = text;
        while let Some(start) = rest.find(open) {
            rest = &rest[start + open.len()..];
            if let Some(end) = rest.find(close) {
                out.push(rest[..end].to_string());
                rest = &rest[end..];
            }
        }
    }
    // JSON output spells the code as a field
    let mut rest = text;
    while let Some(start) = rest.find("\"code\": \"") {
        rest = &rest[start + 9..];
        if let Some(end) = rest.find('"') {
            out.push(rest[..end].to_string());
        }
    }
    // a note line names a code without brackets, as `confront` does
    let bytes = text.as_bytes();
    for index in 0..bytes.len() {
        if (bytes[index] == b'E' || bytes[index] == b'W')
            && index + 2 < bytes.len()
            && bytes[index + 1] == b'-'
            && bytes[index + 2].is_ascii_uppercase()
            && (index == 0 || !bytes[index - 1].is_ascii_alphanumeric())
        {
            let mut end = index;
            while end < bytes.len()
                && (bytes[end].is_ascii_uppercase()
                    || bytes[end] == b'-'
                    || bytes[end].is_ascii_digit())
            {
                end += 1;
            }
            out.push(text[index..end].to_string());
        }
    }
    out
}

/// Codes that no single document produces through `check`, with the operation that does produce
/// them.  Each is exercised below, so the mapping cannot rot into a comment.
const BY_PARSE: &[(&str, &str)] = &[
    ("E-SYNTAX", "fact F_A { claim: \"unterminated }\n"),
    (
        "E-VERSION",
        "tsare 9\nfact F_A { claim: \"a\", evidence: user }\n",
    ),
];

#[test]
fn every_diagnostic_has_a_document_that_turns_it_on() {
    let mut seen: HashSet<String> = HashSet::new();
    let mut from_documents: HashSet<String> = HashSet::new();

    // 1. one small document per code
    let mut files: Vec<PathBuf> = std::fs::read_dir(corpus())
        .expect("tests/codes exists")
        .filter_map(|entry| entry.ok().map(|item| item.path()))
        .filter(|path| path.extension().map(|ext| ext == "tsare").unwrap_or(false))
        .collect();
    files.sort();
    for path in &files {
        let source = std::fs::read_to_string(path).expect("read a trigger document");
        let decls = match parse(&source) {
            Ok(decls) => decls,
            // the empty document is a trigger for E-EMPTY-DOCUMENT and parses to nothing
            Err(error) => panic!("{} does not parse: {error:?}", path.display()),
        };
        let found: Vec<String> = check(&decls)
            .iter()
            .map(|item| item.code.to_string())
            .collect();
        from_documents.extend(found.iter().cloned());
        let expected = path
            .file_stem()
            .map(|stem| stem.to_string_lossy().to_string())
            .unwrap_or_default();
        assert!(
            found.iter().any(|code| code == &expected),
            "{} should produce {expected}, produced {found:?}",
            path.display()
        );
        seen.extend(found);
    }

    // 2. recorded output of the binary
    let golden = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("golden");
    for entry in std::fs::read_dir(golden).expect("golden directory") {
        let path = entry.expect("entry").path();
        if path.extension().map(|ext| ext == "txt").unwrap_or(false) {
            let text = std::fs::read_to_string(&path).unwrap_or_default();
            seen.extend(codes_in(&text));
        }
    }

    // 3. two documents, a goal file: codes no single file can produce
    let kernel = parse(
        "layer L_K { name: \"k\", exposes: [K_ONE] }\n\
         fact F_K { claim: \"k\", evidence: user }\n\
         law K_ONE { statement: \"s\", holds_in: L_K, checked_by: [F_K] }\n",
    )
    .expect("the kernel parses");
    let target = parse(
        "law K_ONE { statement: \"redefined\", holds_in: L_K, checked_by: [F_T] }\n\
         fact F_T { claim: \"t\", evidence: user, layer: L_K }\n",
    )
    .expect("the target parses");
    seen.extend(
        tsare::kernel::check(&kernel, &target)
            .iter()
            .map(|item| item.code.to_string()),
    );

    let goals = tsare::coverage::load_goals("ship the thing\tnothing recorded matches this\n");
    let plain = parse("fact F_A { claim: \"a\", evidence: user }\n").expect("a plain document");
    let report = tsare::coverage::coverage(&tsare::program::Program::new(&plain), &goals);
    seen.extend(
        tsare::coverage::diagnostics(&report)
            .iter()
            .map(|item| item.code.to_string()),
    );

    // 4. parsing, which `check` never sees
    for (code, source) in BY_PARSE {
        assert!(
            parse(source).is_err(),
            "the document for {code} was expected not to parse"
        );
        seen.insert(code.to_string());
    }

    let missing: Vec<&str> = tsare::explain::CODES
        .iter()
        .filter(|code| !seen.contains(**code))
        .copied()
        .collect();
    assert!(
        missing.is_empty(),
        "{} of {} codes have no document that turns them on: {missing:?}",
        missing.len(),
        tsare::explain::CODES.len()
    );

    // a code with a document must not be an explanation for something nothing emits
    let unknown: Vec<&String> = seen
        .iter()
        .filter(|code| !tsare::explain::CODES.contains(&code.as_str()))
        .collect();
    assert!(
        unknown.is_empty(),
        "these codes are produced but not explained: {unknown:?}"
    );
    eprintln!(
        "covered {} codes ({} from single documents)",
        seen.len(),
        from_documents.len()
    );
}
