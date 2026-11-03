use super::*;

#[test]
fn a_list_checks_its_elements() {
    let source = r#"
hypothesis H_A { assumes: [F], predicts: "a", test: "t" }
hypothesis H_B { assumes: [F], predicts: "b", rival: H_A }
question Q { asks: "q", answers: [H_A, H_B], separates: [H_A, H_B] }
fact F { claim: "x", evidence: user }
"#;
    assert!(
        codes(source).contains(&"E-TYPE"),
        "answers is list:str, so hypotheses are not answers"
    );
}

#[test]
fn every_kind_owes_an_action() {
    use tsare::program::TABLE;
    assert_eq!(TABLE.len(), 13, "one row per kind");
    for (kind, obligation, discharge) in TABLE {
        assert!(
            tsare::specs::spec(kind).is_some(),
            "{kind} has no field spec"
        );
        assert!(
            !obligation.is_empty() && !discharge.is_empty(),
            "{kind} owes nothing?"
        );
        assert_ne!(
            tsare::build::action_of(kind),
            "?",
            "{kind} has no action in OBLIGATIONS.md"
        );
    }
}

#[test]
fn a_session_owns_order_counts_and_shape() {
    let mut session = tsare::session::Session::new("doc.tsare");
    session.emit(tsare::check::Diagnostic {
        code: "W-HOLE",
        message: "later".to_string(),
        line: 9,
        col: 1,
        end_line: 9,
        end_col: 2,
        file: String::new(),
        witness: None,
    });
    session.emit(tsare::check::Diagnostic {
        code: "E-TYPE",
        message: "earlier".to_string(),
        line: 2,
        col: 5,
        end_line: 2,
        end_col: 9,
        file: String::new(),
        witness: None,
    });
    assert_eq!(
        (session.errors(), session.warnings(), session.exit_code()),
        (1, 1, 1)
    );
    assert_eq!(session.summary(), "1 error(s), 1 warning(s)");
    let json = session.json();
    assert!(json.contains("\"code\": \"E-TYPE\"") && json.contains("\"code\": \"W-HOLE\""));
    assert!(json.contains("\"errors\": 1") && json.contains("\"warnings\": 1"));
    assert!(
        json.find("E-TYPE").unwrap() < json.find("W-HOLE").unwrap(),
        "sorted by position"
    );
    // with the source, the text form is the rendered form
    let rendered = tsare::session::Session::new("doc.tsare")
        .with_source("x\nfact F { claim: \"y\" }\n")
        .text();
    assert!(rendered.is_empty(), "an empty session prints nothing");
}

#[test]
fn a_directory_is_rendered_file_by_file() {
    let sources = vec![
        (
            "a.tsare".to_string(),
            "fact F_A { claim: \"x\" }\n".to_string(),
        ),
        (
            "b.tsare".to_string(),
            "fact F_B { claim: \"y\" }\n".to_string(),
        ),
    ];
    let mut session = tsare::session::Session::new("dir").with_sources(&sources);
    session.emit(tsare::check::Diagnostic {
        code: "E-TYPE",
        message: "b is wrong".to_string(),
        line: 1,
        col: 18,
        end_line: 1,
        end_col: 21,
        file: "b.tsare".to_string(),
        witness: None,
    });
    session.emit(tsare::check::Diagnostic {
        code: "E-MISSING",
        message: "a is incomplete".to_string(),
        line: 1,
        col: 1,
        end_line: 1,
        end_col: 4,
        file: "a.tsare".to_string(),
        witness: None,
    });
    let text = session.text();
    assert!(text.contains("b.tsare:1:18: error[E-TYPE]"), "{text}");
    assert!(text.contains("a.tsare:1:1: error[E-MISSING]"), "{text}");
    assert!(
        text.contains("claim"),
        "the source line is printed for each file: {text}"
    );
    assert!(
        text.find("a.tsare").unwrap() < text.find("b.tsare").unwrap(),
        "files in load order"
    );
}

#[test]
fn a_document_can_arrive_on_standard_input() {
    use std::io::Write;
    use std::process::{Command, Stdio};
    let mut child = Command::new(env!("CARGO_BIN_EXE_tsare"))
        .args(["check", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn the compiler");
    child
        .stdin
        .as_mut()
        .expect("stdin")
        .write_all("fact F { claim: \"x\", evidence: user }\n".as_bytes())
        .expect("write to stdin");
    let output = child.wait_with_output().expect("wait");
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(text.contains("0 error(s), 0 warning(s)"), "{text}");
    assert_eq!(output.status.code(), Some(0));
}

#[test]
fn a_declaration_keeps_its_positions() {
    let declared = decls("fact F1 { claim: \"x\", evidence: user }\n");
    assert_eq!(declared[0].line, 1);
    assert_eq!(declared[0].col, 1);
    assert_eq!(
        declared[0].field_span[0],
        (1, 18, 1, 21),
        "the claim value runs from col 18 to 21"
    );
    let reference = decls("proposition P1 { statement: \"a\", from: [F_MISSING] }");
    assert_eq!(reference[0].ref_pos[0].0, "F_MISSING");
    assert_eq!(
        reference[0].ref_pos[0].2, 41,
        "the reference is where the name was written"
    );
}

#[test]
fn the_content_hash_is_a_function_of_the_bytes() {
    assert_eq!(
        tsare::hash::hex(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    assert_ne!(tsare::hash::hex(b"abc"), tsare::hash::hex(b"abd"));
}

#[test]
fn the_store_reader_reads_the_records_it_wrote() {
    let line = r#"{"doc": "a.tsare", "codes": ["E-UNDEFINED", "W-HOLE"], "action": "mechanical", "before": {"errors": 2, "warnings": 4}, "after": {"errors": 1, "warnings": 4}, "checker": "tsare-0.1"}"#;
    let entry = tsare::json::parse(line).expect("parse");
    assert_eq!(
        entry
            .get("codes")
            .map(|value| value.strings())
            .unwrap_or_default(),
        vec!["E-UNDEFINED", "W-HOLE"]
    );
    assert!(tsare::cases::improved(&entry));
    assert_eq!(
        tsare::json::serialize(&entry),
        r#"{"doc": "a.tsare", "codes": ["E-UNDEFINED", "W-HOLE"], "action": "mechanical", "before": {"errors": 2, "warnings": 4}, "after": {"errors": 1, "warnings": 4}, "checker": "tsare-0.1"}"#
    );
    assert_eq!(
        tsare::cases::list_repr(&["A".to_string(), "B".to_string()]),
        "['A', 'B']"
    );
}

#[test]
fn names_resolve_once_per_program() {
    let source = "fact F1 { claim: \"x\", evidence: user }\nproposition P1 { statement: \"a\", from: [F1] }\n";
    let declared = decls(source);
    let program = Program::new(&declared);
    let names: HashSet<&str> = program
        .of_kind("fact")
        .iter()
        .map(|decl| decl.name.as_str())
        .collect();
    assert_eq!(names, HashSet::from(["F1"]));
    assert!(program.get("P1").is_some());
    assert!(program.get("P2").is_none());
}

/// The projection quotes the sentence and keeps everything that makes it checkable beside it.
/// A ready obligation owes nothing, so its note is not repeated; the blocked one carries the
/// reason, which is what the margin is for.
#[test]
fn a_projection_quotes_the_sentence_and_keeps_its_ground_in_the_margin() {
    let declared = decls(
        "layer L_X { name: \"shipping\", exposes: [P_A] }\nfact F_A { claim: \"the build is green\", evidence: model }\nproposition P_A { statement: \"it is ready\", from: [F_A], layer: L_X }\n",
    );
    let program = Program::new(&declared);
    let text = tsare::outline::markdown(&program, &check(program.decls), "t.tsare");
    assert!(text.contains("> the build is green"), "{text}");
    assert!(text.contains("## L_X"), "{text}");
    assert!(text.contains("evidence: model"), "{text}");
    assert!(
        text.contains("obligation: check (blocked), model-supplied, not confirmed"),
        "{text}"
    );
    assert!(text.contains("! W-UNCONFIRMED"), "{text}");
    // a field is never paraphrased, and the counted tail is part of the document
    assert!(text.contains("> it is ready"), "{text}");
    assert!(text.contains("obligation(s),"), "{text}");
}

/// `out` is what the instrument produced.  A fact naming an artifact whose bytes hash to something
/// else is not evidence: the run that produced this document is not the run that produced the file.
/// A hash with nothing to compare is unverified, which is a different thing from wrong.
#[test]
fn an_artifact_that_does_not_match_its_hash_is_refused() {
    let dir = std::env::temp_dir().join("tsare-artifact");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("out.log");
    std::fs::write(&path, b"3 runs, all within threshold\n").expect("write artifact");
    let shown = path.display().to_string().replace('\\', "/");
    let actual = tsare::hash::hex(b"3 runs, all within threshold\n");

    let matching = format!(
        "fact F_OK {{ claim: \"three runs\", evidence: tool(\"bench.sh\"), artifact: \"{shown}\", out: \"{actual}\" }}\n"
    );
    let found = check(&decls(&matching));
    assert!(
        !found
            .iter()
            .any(|item| item.code.starts_with('E') || item.code == "W-OUT-UNVERIFIED"),
        "{found:#?}"
    );

    let wrong = matching.replace(
        &actual,
        "0000000000000000000000000000000000000000000000000000000000000000",
    );
    assert!(check(&decls(&wrong))
        .iter()
        .any(|item| item.code == "E-OUT-MISMATCH"));

    let unverified =
        format!("fact F_ONE {{ claim: \"x\", evidence: tool(\"bench.sh\"), out: \"{actual}\" }}\n");
    assert!(check(&decls(&unverified))
        .iter()
        .any(|item| item.code == "W-OUT-UNVERIFIED"));
}
