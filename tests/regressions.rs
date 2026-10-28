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

#[test]
fn engine_lists_conditional_rows_in_a_stable_order() {
    // Defect: closure::engine built `conditional` by iterating a HashMap.  The same document
    // produced a different order in every process, so `engine --json` was not diffable.
    let decls = parse(include_str!("../docs/49-engine-order-repro.tsare"));
    let mut orders = std::collections::BTreeSet::new();
    for _ in 0..20 {
        let result = tsare::closure::engine(&decls);
        orders.insert(
            result
                .conditional
                .iter()
                .map(|(name, _)| name.clone())
                .collect::<Vec<String>>(),
        );
    }
    assert_eq!(
        orders.len(),
        1,
        "same document, different orders: {orders:#?}"
    );
    let names: Vec<String> = orders.into_iter().next().unwrap();
    let mut sorted = names.clone();
    sorted.sort();
    assert_eq!(names, sorted, "the rows are printed in name order");
}

#[test]
fn a_misspelled_option_is_refused_not_ignored() {
    // Defect: every --word that was not a known value flag became a flag the dispatcher
    // ignored, so --strcit checked without the gate while the caller believed it was gated.
    let output = Command::new(binary())
        .args(["check", "docs/repro/engine-order.tsare", "--strcit"])
        .current_dir(root())
        .output()
        .expect("run the binary");
    let text = stdout(&output);
    assert_eq!(output.status.code(), Some(2), "{text}");
    assert!(text.contains("unknown option --strcit"), "{text}");
}

#[test]
fn propose_apply_with_a_later_option_still_writes_the_document() {
    // Defect: --apply was in VALUE_FLAGS, so --apply --policy harmless consumed --policy as
    // the output path (fix's meaning) and cmd_propose never saw the flag. The proposal was
    // reported admitted and the file was left unchanged.
    let dir = temp("propose-apply");
    let doc = dir.join("target.tsare");
    std::fs::copy(root().join("docs/repro/propose-target.tsare"), &doc).expect("copy reproducer");
    let proposals = root().join("docs/repro/proposals.json");
    let output = run_in(
        &dir,
        &[
            "propose",
            "target.tsare",
            "--proposals",
            proposals.to_str().unwrap(),
            "--apply",
            "--policy",
            "harmless",
        ],
    );
    let text = stdout(&output);
    assert_eq!(output.status.code(), Some(0), "{text}");
    assert!(text.contains("1 admitted"), "{text}");
    let written = std::fs::read_to_string(&doc).expect("read the document");
    assert!(written.contains("evidence: user"), "not applied: {written}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_settled_hypothesis_owes_no_test() {
    // Defect: program::obligations charged a hypothesis for a missing test even when a measured
    // fact had already settled it, so cost claimed external work that uild called done.
    let decls = parse(include_str!("../docs/repro/decided-hypothesis.tsare"));
    let cost = tsare::cost::report(&tsare::program::Program::new(&decls));
    assert_eq!(cost.total, 0, "{:#?}", cost.rows);
}

#[test]
fn build_does_not_invent_a_tool_for_a_hole() {
    // Defect: build::obligation defaulted a missing or hole vidence to "tool", so OBLIGATIONS.md
    // reported a ready fact whose evidence was unknown while check reported W-HOLE.
    let decls = parse(include_str!("../docs/repro/build-hole-evidence.tsare"));
    let program = tsare::program::Program::new(&decls);
    let row = tsare::build::obligation(
        program.get("F_HOLE").expect("F_HOLE"),
        &program,
        &Default::default(),
    );
    assert_ne!(row.state, "ready");
    assert!(!row.note.contains("tool"), "{}", row.note);
}

#[test]
fn build_does_not_call_an_unwarranted_proposition_ready() {
    // Defect: build::obligation marked a proposition "ready | premises given" as soon as it had
    // a from list, without asking whether any rule concluded it. On the same bytes, check warned
    // W-UNWARRANTED and cost charged "no rule supports it".
    let decls = parse(include_str!("../docs/repro/build-unwarranted.tsare"));
    let program = tsare::program::Program::new(&decls);
    let row = tsare::build::obligation(program.get("P").expect("P"), &program, &Default::default());
    assert_eq!(row.state, "blocked", "{}", row.note);
    assert!(row.note.contains("rule"), "{}", row.note);
}

#[test]
fn derive_does_not_ground_a_conclusion_in_a_retired_fact() {
    // Defect: derive::analyze seeded its least fixpoint with every fact, including the ones a
    // supersedes field had withdrawn. It reported P_FAST grounded while engine reported it
    // conditional through the retired premise.
    let decls = parse(include_str!("../docs/repro/retired-derive.tsare"));
    let analysis = tsare::derive::analyze(&tsare::program::Program::new(&decls));
    assert!(
        !analysis.grounded.contains(&"P_FAST".to_string()),
        "{:?}",
        analysis.grounded
    );
    assert!(
        analysis.conditional.contains(&"P_FAST".to_string()),
        "{:?}",
        analysis.conditional
    );
    let result = tsare::closure::engine(&decls);
    assert!(
        result.conditional.iter().any(|(name, _)| name == "P_FAST"),
        "{:#?}",
        result.conditional
    );
}

#[test]
fn a_class_conclusion_connects_its_rule_to_every_target() {
    // Defect: a warrant may conclude a class of propositions (concludes:), and engine/derive
    // used closure::applications for that while check and argue looked only at 	o:. The same
    // rule was a connection for two commands and no rule at all for two others.
    let decls = parse(include_str!("../docs/48-concludes.tsare"));
    let found = tsare::check::check(&decls);
    assert!(
        !found
            .iter()
            .any(|item| item.code == "W-UNWARRANTED" && item.message.contains("P_OPEN")),
        "{found:#?}"
    );
    let program = tsare::program::Program::new(&decls);
    let rows = tsare::argue::conclusions(&program);
    for name in ["P_OPEN_ONE", "P_OPEN_TWO"] {
        let row = rows.iter().find(|row| row.name == name).expect(name);
        assert_eq!(row.status, "supported", "{name}: {}", row.why);
    }
    let result = tsare::closure::engine(&decls);
    for name in ["P_OPEN_ONE", "P_OPEN_TWO"] {
        assert!(
            result.held.contains(&name.to_string()),
            "{name} is not held"
        );
    }
}

#[test]
fn opposition_is_not_a_justification_cycle() {
    // Defect: check_cycles followed every REF_KEYS edge, including `rival` and `attacks`. Two
    // hypotheses that are each other's rival -- the normal shape of a live disagreement -- were
    // refused with "nothing in the circle is grounded", which is a claim about support.
    let rivals = tsare::check::check(&parse(include_str!("../docs/repro/mutual-rival.tsare")));
    assert!(
        !rivals.iter().any(|item| item.code == "E-CYCLE"),
        "{rivals:#?}"
    );
    let attacks = parse(concat!(
        "fact F { claim: \"x\", evidence: user }\n",
        "proposition P_A { statement: \"a\", from: [F] }\n",
        "proposition P_B { statement: \"b\", from: [F] }\n",
        "warrant W_A { claim: \"a\", from: [F], to: P_A, attacks: [W_B] }\n",
        "warrant W_B { claim: \"b\", from: [F], to: P_B, attacks: [W_A] }\n",
    ));
    assert!(
        !tsare::check::check(&attacks)
            .iter()
            .any(|item| item.code == "E-CYCLE"),
        "mutual attack is an undecided argument, not a circular proof"
    );
    // A support cycle is still refused: the rule and the proposition lean on each other.
    let support = parse(concat!(
        "proposition P_ONE { statement: \"one\", from: [P_TWO] }\n",
        "proposition P_TWO { statement: \"two\", from: [P_ONE] }\n",
    ));
    assert!(tsare::check::check(&support)
        .iter()
        .any(|item| item.code == "E-CYCLE"));
}

#[test]
fn a_field_written_twice_is_refused() {
    // Defect: check_record found the first occurrence of a field and ignored later ones, so
    // `evidence: user, evidence: model` compiled cleanly and every reader (id_field, f_str)
    // silently used the first value. The document said two things; the checker saw one.
    let found = tsare::check::check(&parse(include_str!("../docs/repro/duplicate-field.tsare")));
    assert!(
        found
            .iter()
            .any(|item| item.code == "E-DUP-FIELD" && item.message.contains("evidence")),
        "{found:#?}"
    );
}

#[test]
fn a_percent_suffixed_confidence_is_still_out_of_range() {
    // Defect: check::num_field used a strict f64 parse while the lexer makes "150%" one number
    // token and type_ok accepts it as num. The strict parse returned None, so a confidence of
    // 150% skipped E-CONFIDENCE entirely and compiled as clean.
    let found = tsare::check::check(&parse(include_str!(
        "../docs/repro/confidence-percent.tsare"
    )));
    assert!(
        found.iter().any(|item| item.code == "E-CONFIDENCE"),
        "{found:#?}"
    );
}
