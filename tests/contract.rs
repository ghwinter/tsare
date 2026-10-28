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

fn status(args: &[&str]) -> i32 {
    Command::new(binary())
        .args(args)
        .output()
        .expect("run the binary")
        .status
        .code()
        .unwrap_or(-1)
}

/// A warning is the engine saying the document states something it cannot settle.  By default that
/// is advice, because a document still being written has to compile; `--strict` is how a loop makes
/// it a gate, and the number is the only thing a loop can be trusted to read.
#[test]
fn strict_makes_advice_into_a_gate() {
    let warned = fixture("talk.v2.tsare");
    assert_eq!(status(&["check", &warned]), 0);
    assert_eq!(status(&["check", &warned, "--strict"]), 1);
    assert_eq!(status(&["check", &warned, "--strict", "--json"]), 1);
    let clean = fixture("hello.tsare");
    assert_eq!(status(&["check", &clean, "--strict"]), 0);
}

/// Three answers a loop must be able to tell apart without reading a word: the document is bad,
/// the file is not there, and the invocation is wrong.
#[test]
fn exit_codes_separate_a_bad_document_from_a_bad_invocation() {
    let dir = std::env::temp_dir().join(format!("tsare-exit-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let broken = dir.join("dup.tsare");
    std::fs::write(
        &broken,
        "fact F { claim: \"a\", evidence: user }\nfact F { claim: \"b\", evidence: user }\n",
    )
    .expect("write the document");
    let broken = broken.display().to_string();
    let missing = dir.join("missing.tsare").display().to_string();
    let error = status(&["check", &broken]);
    let absent = status(&["check", &missing]);
    let usage = status(&["check"]);
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(error, 1, "a document with an error");
    assert_eq!(absent, 1, "a file that is not there");
    assert_eq!(usage, 2, "an invocation that names no document");
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

/// The three commands a loop uses to decide what to do next answer in a named envelope, like the
/// rest.  A loop that has to read prose is a loop that has to guess, which is the one thing this
/// tool exists to avoid.
#[test]
fn the_decision_commands_answer_json() {
    let doc = fixture("talk.v2.tsare");
    let cases: [(&[&str], &[&str]); 3] = [
        (&["next", &doc], &["decisions"]),
        (
            &["cost", &doc],
            &["total", "depth", "chain", "by_kind", "rows"],
        ),
        (&["shape", &doc], &["depth", "total", "forms"]),
    ];
    for (args, keys) in cases {
        let mut args: Vec<&str> = args.to_vec();
        args.push("--json");
        let text = run(&args);
        assert_eq!(top_keys(&text), keys.to_vec(), "{text}");
    }

    let against = fixture("relational.v2.tsare");
    let shaped = run(&["shape", &doc, "--against", &against, "--json"]);
    assert_eq!(
        top_keys(&shaped),
        ["depth", "shared", "only_here", "only_there"],
        "{shaped}"
    );

    // the near misses are opt-in, and they are a guess: the envelope says so by name
    let near = run(&[
        "shape",
        &doc,
        "--against",
        &against,
        "--json",
        "--near",
        "2",
    ]);
    assert!(top_keys(&near).contains(&"near".to_string()), "{near}");
}

/// `--near` has nothing to be near without a second document, and a silently ignored flag is a
/// flag the caller will believe took effect.  `--layers` answers a different question, so asking
/// both at once is refused rather than half-answered.
#[test]
fn near_without_a_second_document_is_refused() {
    let doc = fixture("hello.tsare");
    assert_eq!(status(&["shape", &doc, "--near", "3"]), 2);
    let other = fixture("relational.v2.tsare");
    assert_eq!(
        status(&[
            "shape",
            &doc,
            "--against",
            &other,
            "--layers",
            "--near",
            "3"
        ]),
        2
    );
}

#[test]
fn a_diagnostic_object_keeps_its_seven_fields_in_order() {
    let checked = run(&["check", &fixture("talk.v2.tsare"), "--json"]);
    let first = checked
        .lines()
        .find(|line| line.trim_start().starts_with("{\"code\""))
        .expect("at least one diagnostic");
    let mut cursor = 0;
    for key in ["code", "severity", "name", "file", "line", "col", "message"] {
        let needle = format!("\"{key}\"");
        let found = first[cursor..]
            .find(&needle)
            .unwrap_or_else(|| panic!("the diagnostic object lost {key}: {first}"));
        cursor += found + needle.len();
    }
}

/// A report is only diffable if it says which declaration it is about: a line moves whenever
/// anything above it is edited, so a change keyed by position would call every edit a change.
#[test]
fn every_diagnostic_names_its_declaration() {
    let checked = run(&["check", &fixture("talk.v2.tsare"), "--json"]);
    let mut seen = 0;
    for line in checked.lines() {
        let line = line.trim();
        if !line.starts_with("{\"code\"") {
            continue;
        }
        seen += 1;
        assert!(!field(line, "name").is_empty(), "{line}");
    }
    assert!(seen > 3, "the fixture should carry several diagnostics");
}

/// Two reports in, one answer out: what disappeared and what appeared, keyed by declaration.  The
/// loop that made the edit is the only one that can decide whether the change is an improvement.
#[test]
fn a_delta_report_can_be_read_by_a_machine() {
    let dir = std::env::temp_dir().join(format!("tsare-delta-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let before = dir.join("before.json");
    let after = dir.join("after.json");
    std::fs::write(
        &before,
        r#"{"diagnostics": [{"code": "E-UNDEFINED", "name": "P_A", "line": 4}]}"#,
    )
    .expect("write the first report");
    std::fs::write(&after, r#"{"diagnostics": []}"#).expect("write the second report");
    let text = run(&[
        "delta",
        &before.display().to_string(),
        &after.display().to_string(),
        "--json",
    ]);
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(
        top_keys(&text),
        ["before", "after", "gone", "appeared"],
        "{text}"
    );
    assert!(text.contains("\"code\": \"E-UNDEFINED\""), "{text}");
}

/// The memory of repairs: two reports plus the action that produced them become one line, and the
/// next time the same complaint appears the store can say what worked.  Everything in the entry is
/// derived from the reports; the agent supplies only which of three actions it took, because a
/// memory narrated by the party being judged is the thing the audit exists to catch.
#[test]
fn a_repair_is_remembered_and_can_be_recalled() {
    let dir = std::env::temp_dir().join(format!("tsare-cases-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let doc = dir.join("doc.tsare");
    let before = dir.join("before.json");
    let after = dir.join("after.json");
    let store = dir.join("cases.jsonl");
    let shown = doc.display().to_string();
    std::fs::write(
        &doc,
        concat!(
            "fact F { claim: \"the cache is cold\", evidence: model }\n",
            "proposition P { statement: \"the query was slow\", from: [F] }\n",
        ),
    )
    .expect("write the document");
    std::fs::write(&before, run(&["check", &shown, "--json"])).expect("write the first report");
    std::fs::write(
        &doc,
        concat!(
            "fact F { claim: \"the cache is cold\", evidence: user }\n",
            "proposition P { statement: \"the query was slow\", from: [F] }\n",
        ),
    )
    .expect("rewrite the document");
    std::fs::write(&after, run(&["check", &shown, "--json"])).expect("write the second report");
    let recorded = run(&[
        "delta",
        &before.display().to_string(),
        &after.display().to_string(),
        "--record",
        &store.display().to_string(),
        "--doc",
        &shown,
        "--action",
        "model",
    ]);
    assert!(recorded.contains("recorded:"), "{recorded}");
    let recalled = run(&[
        "mem",
        "--store",
        &store.display().to_string(),
        "--recall",
        "W-UNCONFIRMED",
    ]);
    let audit = run(&["mem", "--store", &store.display().to_string()]);
    let _ = std::fs::remove_dir_all(&dir);
    assert!(recalled.contains("improved"), "{recalled}");
    assert!(recalled.contains("W-UNCONFIRMED"), "{recalled}");
    assert!(audit.contains("cases: 1"), "{audit}");
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
