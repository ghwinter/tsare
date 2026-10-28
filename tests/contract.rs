//! The `--json` contract, pinned by shape rather than by value.
//!
//! Three envelopes are read by machines: `check`, `engine`, `suggest`. Their field names, their
//! order, and the invariant that a code starting with `E` carries severity `error` are the
//! contract. The messages are not: they are prose and they are allowed to improve.
//!
//! The scanner walks characters and tracks depth instead of pulling in a dependency, because a
//! checker that cannot be tested without a parser crate is a checker nobody can test.

use std::path::Path;
use std::process::Command;

fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_tsare")
}

fn fixture(name: &str) -> String {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
        .display()
        .to_string()
}

fn run(args: &[&str]) -> String {
    let output = Command::new(binary())
        .args(args)
        .output()
        .expect("run the binary");
    String::from_utf8_lossy(&output.stdout).to_string()
}

/// The keys of the objects at nesting depth 1, in the order they appear.
fn top_keys(text: &str) -> Vec<String> {
    let mut depth = 0usize;
    let mut out = Vec::new();
    let mut chars = text.char_indices().peekable();
    while let Some((index, ch)) = chars.next() {
        match ch {
            '{' | '[' => depth += 1,
            '}' | ']' => depth = depth.saturating_sub(1),
            '"' => {
                let (end, key) = read_string(text, index, &mut chars);
                if depth == 1 && followed_by_colon(&mut chars) {
                    out.push(key.to_string());
                }
                let _ = end;
            }
            _ => {}
        }
    }
    out
}

fn read_string<'a>(
    text: &'a str,
    open: usize,
    chars: &mut std::iter::Peekable<std::str::CharIndices<'a>>,
) -> (usize, &'a str) {
    let start = open + 1;
    let mut end = start;
    let mut escaped = false;
    for (index, ch) in chars.by_ref() {
        if escaped {
            escaped = false;
            end = index + ch.len_utf8();
            continue;
        }
        match ch {
            '\\' => escaped = true,
            '"' => return (index, &text[start..index]),
            _ => end = index + ch.len_utf8(),
        }
    }
    (end, &text[start..end])
}

fn followed_by_colon(chars: &mut std::iter::Peekable<std::str::CharIndices<'_>>) -> bool {
    let probe = chars.clone();
    for (_, ch) in probe {
        match ch {
            ' ' | '\t' | '\n' | '\r' => continue,
            ':' => return true,
            _ => return false,
        }
    }
    false
}

#[test]
fn the_json_contract_does_not_drift() {
    let checked = run(&["check", &fixture("talk.v2.tsare"), "--json"]);
    assert_eq!(
        top_keys(&checked),
        ["diagnostics", "errors", "warnings"],
        "{checked}"
    );

    let engine = run(&["engine", &fixture("ungrounded.tsare"), "--json"]);
    assert_eq!(
        top_keys(&engine),
        ["held", "conditional", "blocked", "chains", "gates"],
        "{engine}"
    );

    let suggest = run(&["suggest", &fixture("talk.v2.tsare"), "--json"]);
    assert_eq!(top_keys(&suggest), ["suggestions"], "{suggest}");
}

#[test]
fn a_diagnostic_object_keeps_its_six_fields_in_order() {
    let checked = run(&["check", &fixture("talk.v2.tsare"), "--json"]);
    let first = checked
        .lines()
        .find(|line| line.trim_start().starts_with("{\"code\""))
        .expect("at least one diagnostic");
    let mut cursor = 0;
    for key in ["code", "severity", "file", "line", "col", "message"] {
        let needle = format!("\"{key}\"");
        let found = first[cursor..]
            .find(&needle)
            .unwrap_or_else(|| panic!("the diagnostic object lost {key}: {first}"));
        cursor += found + needle.len();
    }
}

#[test]
fn severity_follows_the_code_prefix() {
    let checked = run(&["check", &fixture("talk.v2.tsare"), "--json"]);
    let mut seen = 0;
    for line in checked.lines() {
        let line = line.trim();
        if !line.starts_with("{\"code\"") {
            continue;
        }
        seen += 1;
        let code = field(line, "code");
        let severity = field(line, "severity");
        if code.starts_with('E') {
            assert_eq!(severity, "error", "{line}");
        } else {
            assert_eq!(severity, "warning", "{line}");
        }
        assert!(!field(line, "message").is_empty(), "{line}");
    }
    assert!(seen > 3, "the fixture should carry several diagnostics");
}

/// A count printed above a list is a claim about the document, not about the flag.  `--top` may cut
/// what is shown, never the total: a loop that reads "1 load-bearing assumption" and stops has been
/// told the wrong number by the tool it is trusting.
#[test]
fn a_cut_list_still_reports_the_whole_total() {
    let path = std::env::temp_dir().join(format!("tsare-total-{}.tsare", std::process::id()));
    std::fs::write(
        &path,
        concat!(
            "fact F1 { claim: \"a\", evidence: model }\n",
            "fact F2 { claim: \"b\", evidence: model }\n",
            "fact F3 { claim: \"c\", evidence: model }\n",
            "proposition P { statement: \"p\", from: [F1, F2, F3] }\n",
        ),
    )
    .expect("write the document");
    let shown = run(&["assumptions", &path.display().to_string(), "--top", "1"]);
    let _ = std::fs::remove_file(&path);
    assert!(shown.contains("3 load-bearing assumption(s)"), "{shown}");
    assert!(shown.contains("2 more"), "{shown}");
}

/// A misspelled command must not turn into a file name.  The shorthand `tsare doc.tsare` is worth
/// keeping; guessing that any unknown word is a path is not, because the error then blames the
/// filesystem for a typo and a file named after it would be checked in its place.
#[test]
fn a_misspelled_command_is_not_a_document() {
    let output = Command::new(binary())
        .args(["shap", "doc.tsare"])
        .output()
        .expect("run the binary");
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(text.contains("unknown command shap"), "{text}");
    assert_eq!(output.status.code(), Some(2), "{text}");
}

fn field(object: &str, key: &str) -> String {
    let needle = format!("\"{key}\": ");
    let start = object
        .find(&needle)
        .unwrap_or_else(|| panic!("no field {key} in {object}"))
        + needle.len();
    let rest = &object[start..];
    if let Some(stripped) = rest.strip_prefix('"') {
        let end = stripped.find('"').expect("a closing quote");
        stripped[..end].to_string()
    } else {
        rest.chars()
            .take_while(|ch| *ch != ',' && *ch != '}')
            .collect::<String>()
            .trim()
            .to_string()
    }
}
