//! Regression tests for defects found by checking the documents in `tests/fixtures/harden/`.
//!
//! Each test names the reproducer in `tests/fixtures/harden/repro/` and the behaviour the checker owes.  The
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
    std::fs::copy(
        root().join("tests/fixtures/harden/repro/engine-order.tsare"),
        &doc,
    )
    .expect("copy reproducer");
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
        "../tests/fixtures/harden/repro/signature-undefined.tsare"
    )));
    assert!(
        found
            .iter()
            .any(|item| item.code == "E-UNDEFINED" && item.message.contains("C_MISSING")),
        "{found:#?}"
    );
    let found = tsare::check::check(&parse(include_str!(
        "../tests/fixtures/harden/repro/signature-kind.tsare"
    )));
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
    let decls = parse(include_str!("fixtures/harden/49-engine-order-repro.tsare"));
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
        .args([
            "check",
            "tests/fixtures/harden/repro/engine-order.tsare",
            "--strcit",
        ])
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
    std::fs::copy(
        root().join("tests/fixtures/harden/repro/propose-target.tsare"),
        &doc,
    )
    .expect("copy reproducer");
    let proposals = root().join("tests/fixtures/harden/repro/proposals.json");
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
    let decls = parse(include_str!(
        "../tests/fixtures/harden/repro/decided-hypothesis.tsare"
    ));
    let cost = tsare::cost::report(&tsare::program::Program::new(&decls));
    assert_eq!(cost.total, 0, "{:#?}", cost.rows);
}

#[test]
fn build_does_not_invent_a_tool_for_a_hole() {
    // Defect: build::obligation defaulted a missing or hole vidence to "tool", so OBLIGATIONS.md
    // reported a ready fact whose evidence was unknown while check reported W-HOLE.
    let decls = parse(include_str!(
        "../tests/fixtures/harden/repro/build-hole-evidence.tsare"
    ));
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
    let decls = parse(include_str!(
        "../tests/fixtures/harden/repro/build-unwarranted.tsare"
    ));
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
    let decls = parse(include_str!(
        "../tests/fixtures/harden/repro/retired-derive.tsare"
    ));
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
    let decls = parse(include_str!("fixtures/harden/48-concludes.tsare"));
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
    let rivals = tsare::check::check(&parse(include_str!(
        "../tests/fixtures/harden/repro/mutual-rival.tsare"
    )));
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
    let found = tsare::check::check(&parse(include_str!(
        "../tests/fixtures/harden/repro/duplicate-field.tsare"
    )));
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
        "../tests/fixtures/harden/repro/confidence-percent.tsare"
    )));
    assert!(
        found.iter().any(|item| item.code == "E-CONFIDENCE"),
        "{found:#?}"
    );
}

#[test]
fn a_mistyped_unit_is_one_type_error_not_also_a_missing_field() {
    // Defect: the "blank unit" test counted every present value that was not a non-empty string,
    // so `unit: 5` produced E-TYPE and E-INCOMMENSURABLE, the second saying the value "lacks
    // unit" when it has one of the wrong type.
    let found = tsare::check::check(&parse(include_str!(
        "../tests/fixtures/harden/repro/unit-num.tsare"
    )));
    let types = found.iter().filter(|item| item.code == "E-TYPE").count();
    let missing = found
        .iter()
        .filter(|item| item.code == "E-INCOMMENSURABLE")
        .count();
    assert_eq!(types, 1, "{found:#?}");
    assert_eq!(missing, 0, "{found:#?}");
}

#[test]
fn an_unresolved_dimension_does_not_condemn_its_elements() {
    // Defect: check_partition reported E-PARTITION-ELEMENT for every element when the dimension
    // was undefined, claiming "F_A is not an instance of C_NOPE" about a concept that does not
    // exist. E-UNDEFINED had already said the only thing that is known.
    let found = tsare::check::check(&parse(include_str!(
        "../tests/fixtures/harden/repro/partition-undef.tsare"
    )));
    assert!(
        found.iter().any(|item| item.code == "E-UNDEFINED"),
        "{found:#?}"
    );
    assert!(
        !found.iter().any(|item| item.code == "E-PARTITION-ELEMENT"),
        "{found:#?}"
    );
}

#[test]
fn weigh_propagates_confidence_through_a_proposition() {
    // Defect: weigh folded only facts among a rule's premises and ignored a proposition premise's
    // own weight, so a two-link chain reported 1.00 for the second link where the weakest link
    // was 0.90. check_confidence_flow reads weigh, so W-CONFIDENCE-INFLATED missed the same case.
    let decls = parse(include_str!(
        "../tests/fixtures/harden/repro/confidence-chain.tsare"
    ));
    let weights = tsare::weigh::weights(&tsare::program::Program::new(&decls));
    let second = weights.iter().find(|row| row.name == "P2").expect("P2");
    let value = second.value.expect("P2 has a weight");
    assert!((value - 0.9).abs() < 1e-9, "{value}");
    let found = tsare::check::check(&decls);
    assert!(
        found
            .iter()
            .any(|item| item.code == "W-CONFIDENCE-INFLATED" && item.message.contains("P2")),
        "{found:#?}"
    );
}

#[test]
fn a_syntax_error_does_not_hide_the_other_files_semantic_errors() {
    // Defect: cmd_check skipped the whole semantic pass when any file failed to parse, so a
    // directory with one unterminated string hid the duplicate declaration in the file next to
    // it. Recovery had already parsed that file; the guard threw its diagnostics away.
    let dir = root().join("tests/fixtures/harden/repro/project-dir");
    let output = Command::new(binary())
        .arg("check")
        .arg(&dir)
        .output()
        .expect("run");
    let text = stdout(&output);
    assert!(text.contains("error[E-SYNTAX]"), "{text}");
    assert!(text.contains("error[E-DUP-ID]"), "{text}");
    assert_eq!(output.status.code(), Some(1), "{text}");
}

#[test]
fn project_groups_every_file_the_way_check_does() {
    // Defect: cmd_project called load(), which turns the first syntax error into one string and
    // returns; the string was attributed to the directory, and no other file's diagnostics were
    // reported. PROTOCOL says project is "the same, grouped by file".
    let dir = root().join("tests/fixtures/harden/repro/project-dir");
    let output = Command::new(binary())
        .arg("project")
        .arg(&dir)
        .output()
        .expect("run");
    let text = stdout(&output);
    assert!(text.contains("# b.tsare"), "{text}");
    assert!(text.contains("# c.tsare"), "{text}");
    assert!(text.contains("E-DUP-ID"), "{text}");
    assert!(text.contains("E-SYNTAX"), "{text}");
    assert_eq!(output.status.code(), Some(1), "{text}");
}

#[test]
fn a_command_that_reports_an_error_exits_nonzero() {
    // Defect: argue, impact, weigh, cat, induce and graph printed "N error(s)" and returned 0,
    // so a gate that read the exit status was told the analysis succeeded on a broken program.
    // check, project and build already returned 1 for the same bytes.
    let dir = temp("analysis-exits");
    let doc = dir.join("broken.tsare");
    std::fs::copy(
        root().join("tests/fixtures/harden/repro/project-dir/b.tsare"),
        &doc,
    )
    .expect("copy reproducer");
    let graph_out = dir.join("graph.mmd");
    let shown = doc.to_str().unwrap().to_string();
    let cases: Vec<Vec<&str>> = vec![
        vec!["argue", &shown],
        vec!["impact", &shown],
        vec!["weigh", &shown],
        vec!["cat", &shown],
        vec!["induce", &shown],
        vec!["graph", &shown, "--out", graph_out.to_str().unwrap()],
    ];
    for argv in cases {
        let output = run_in(root(), &argv);
        assert_eq!(
            output.status.code(),
            Some(1),
            "{argv:?}: {}",
            stdout(&output)
        );
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_case_that_got_worse_is_not_called_a_no_op() {
    // Defect: cases::audit counted "not improved and the counts changed" as `noop` and printed
    // "changed nothing either way". That bucket is a regression: the record it is derived from
    // has different before and after counts.
    let dir = temp("mem-regression");
    let store = dir.join("cases.jsonl");
    std::fs::write(
        &store,
        format!(
            "{{\"doc\": \"gone.tsare\", \"codes\": [\"W-UNCONFIRMED\"], \"action\": \"model\", \"before\": {{\"errors\": 0, \"warnings\": 0}}, \"after\": {{\"errors\": 0, \"warnings\": 1}}, \"checker\": \"{}\"}}\n",
            tsare::cache::SALT
        ),
    )
    .expect("write the store");
    let output = run_in(root(), &["mem", "--store", store.to_str().unwrap()]);
    let text = stdout(&output);
    assert!(text.contains("regressed:"), "{text}");
    assert!(text.contains("not for the better"), "{text}");
    assert!(!text.contains("no-op:"), "{text}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_overclaim_explanation_says_what_the_diagnostic_reads() {
    // Defect: --explain W-OVERCLAIM advised "say who or what was sampled, with `scope:`", while
    // the diagnostic itself says scope is prose and cannot be enumerated, and the check only
    // looks at `over:`. Following the explanation could not silence the warning.
    let text = tsare::explain::explain("W-OVERCLAIM").expect("explained");
    assert!(text.contains("`over:`"), "{text}");
    assert!(!text.contains("with `scope:`"), "{text}");
}

#[test]
fn the_help_screen_does_not_promise_exit_two_for_a_missing_file() {
    // Defect: usage() said exit 2 was "a usage problem, or a file could not be read", while
    // cli::fail returns 1 for an unreadable path and PROTOCOL says 2 is "the invocation is
    // wrong". The help text described a contract the binary does not implement.
    let output = Command::new(binary()).arg("--help").output().expect("run");
    let text = stdout(&output);
    assert!(text.contains("2  usage problem"), "{text}");
    assert!(!text.contains("or a file could not be read"), "{text}");
}

#[test]
fn suggest_json_names_the_file_its_span_indexes() {
    // Defect: fix::Suggestion.file is empty for a single-file load (decl.file is only filled by
    // the directory loader), and cmd_suggest printed it as-is. A consumer got line numbers and
    // no document to apply them to, although PROTOCOL's example shows `file`.
    let output = Command::new(binary())
        .args([
            "suggest",
            "tests/fixtures/harden/repro/project-dir/b.tsare",
            "--json",
        ])
        .current_dir(root())
        .output()
        .expect("run");
    let text = stdout(&output);
    assert!(
        text.contains("\"file\": \"tests/fixtures/harden/repro/project-dir/b.tsare\""),
        "{text}"
    );
}

#[test]
fn an_unwritable_output_is_reported_not_a_panic() {
    // Defect: graph, egraph, fix --apply and build wrote with fs::write(...).expect(...), so a
    // path that cannot be written panicked with exit 101 instead of a file problem.
    let dir = temp("unwritable");
    let blocker = dir.join("blocker");
    std::fs::write(&blocker, b"a file, not a directory").expect("write blocker");
    let target = blocker.join("graph.mmd");
    let output = Command::new(binary())
        .args([
            "graph",
            "tests/fixtures/harden/repro/project-dir/a.tsare",
            "--out",
            target.to_str().unwrap(),
        ])
        .current_dir(root())
        .output()
        .expect("run");
    let text = format!(
        "{}{}",
        stdout(&output),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.status.code(), Some(1), "{text}");
    assert!(!text.contains("panicked"), "{text}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn build_on_a_directory_emits_the_same_five_artifacts_as_a_file() {
    // Defect: the directory branch of cmd_build wrote OBLIGATIONS.md and LEARNED.md and printed
    // "emitted", while the file branch wrote all five artifacts. The help text says build PATH,
    // and PATH may be a directory; the missing three were not mentioned.
    let dir = temp("build-directory");
    let out = dir.join("out");
    let output = run_in(
        root(),
        &[
            "build",
            "tests/fixtures/harden/project",
            "--out",
            out.to_str().unwrap(),
        ],
    );
    let text = stdout(&output);
    assert_eq!(output.status.code(), Some(0), "{text}");
    for name in [
        "OBLIGATIONS.md",
        "LEARNED.md",
        "OPEN.md",
        "NOTES.md",
        "predicates.py",
    ] {
        assert!(out.join(name).is_file(), "{name} was not emitted\n{text}");
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn build_refuses_a_directory_that_does_not_compile() {
    // Defect: the directory branch returned 0 without checking the program, so build emitted
    // obligations for a directory with a duplicate declaration; the file branch refuses.
    let dir = temp("build-broken-directory");
    let source = dir.join("src");
    std::fs::create_dir_all(&source).expect("source dir");
    std::fs::copy(
        root().join("tests/fixtures/harden/repro/project-dir/b.tsare"),
        source.join("broken.tsare"),
    )
    .expect("copy reproducer");
    let out = dir.join("out");
    let output = run_in(
        root(),
        &[
            "build",
            source.to_str().unwrap(),
            "--out",
            out.to_str().unwrap(),
        ],
    );
    let text = stdout(&output);
    assert_eq!(output.status.code(), Some(1), "{text}");
    assert!(text.contains("not emitted"), "{text}");
    assert!(!out.join("OBLIGATIONS.md").exists(), "{text}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_retired_law_does_not_sign_a_refinement() {
    // Defect: check_refinements counted a law with `refines:` as a signatory without asking
    // whether the law had been withdrawn. A retired law kept signing for its layer, so
    // W-UNREFINED stayed silent after the only implementation claim was superseded.
    let source = concat!(
        "layer L_UP { name: \"interface\", exposes: [F_A] }\n",
        "layer L_DOWN { name: \"implementation\", exposes: [] }\n",
        "fact F_A { claim: \"a\", evidence: tool(\"t\") }\n",
        "law K_UP { statement: \"the interface holds\", holds_in: L_UP, checked_by: [F_A] }\n",
        "law K_OLD { statement: \"down implements up\", holds_in: L_DOWN, checked_by: [F_A], refines: L_UP }\n",
        "law K_NEW { statement: \"a different down rule\", holds_in: L_DOWN, checked_by: [F_A], supersedes: [K_OLD], because: [F_A] }\n",
    );
    let found = tsare::check::check(&parse(source));
    assert!(
        found
            .iter()
            .any(|item| item.code == "W-UNREFINED" && item.message.contains("K_UP")),
        "{found:#?}"
    );
}

#[test]
fn a_retired_warrant_does_not_connect_premises_to_a_conclusion() {
    // Defect: the W-UNWARRANTED target set was built from every warrant's `to` field, retired or
    // not. A proposition whose only rule had been withdrawn looked connected, while the engine
    // no longer applied the rule and called the conclusion conditional.
    let source = concat!(
        "fact F { claim: \"x\", evidence: tool(\"t\") }\n",
        "proposition P { statement: \"p\", from: [F] }\n",
        "proposition P2 { statement: \"p2\", from: [F] }\n",
        "warrant W_OLD { claim: \"old rule\", from: [F], to: P, basis: [F] }\n",
        "warrant W_NEW { claim: \"new rule\", from: [F], to: P2, basis: [F], supersedes: [W_OLD], because: [F] }\n",
    );
    let found = tsare::check::check(&parse(source));
    assert!(
        found
            .iter()
            .any(|item| item.code == "W-UNWARRANTED" && item.message.contains("P")),
        "{found:#?}"
    );
}

#[test]
fn a_conditional_premise_is_labelled_with_its_kind() {
    // Defect: closure built its reason with `state.get(premise)`; the state map only contains
    // propositions, so every other premise fell back to the word "fact". A hypothesis that a
    // conclusion rests on was reported as "(fact)", which is the one thing it is not.
    let found = tsare::check::check(&parse(include_str!(
        "fixtures/harden/49-engine-order-repro.tsare"
    )));
    let conditional = found
        .iter()
        .find(|item| item.code == "W-CONDITIONAL")
        .expect("a conditional warning");
    assert!(
        conditional.message.contains("(hypothesis)"),
        "{}",
        conditional.message
    );
    assert!(
        !conditional.message.contains("(fact)"),
        "{}",
        conditional.message
    );
}

#[test]
fn a_cut_list_says_how_much_it_cut() {
    // Defect: engine showed three gates, derive three reasons and debt three facts with no
    // marker, and W-CONDITIONAL joined at most two reasons in silence. A reader could not tell
    // a complete list from a truncated one; the JSON carried the rest only for engine.
    let dir = temp("cut-lists");
    let doc = dir.join("lists.tsare");
    std::fs::write(
        &doc,
        concat!(
            "fact F_1 { claim: \"one\", evidence: model }\n",
            "fact F_2 { claim: \"two\", evidence: model }\n",
            "fact F_3 { claim: \"three\", evidence: model }\n",
            "fact F_4 { claim: \"four\", evidence: model }\n",
            "proposition P_GATED { statement: \"gated\", from: [F_1, F_2, F_3, F_4] }\n",
            "warrant W_GATED { claim: \"four premises\", from: [F_1, F_2, F_3, F_4], to: P_GATED }\n",
            "hypothesis H_1 { assumes: [F_1], predicts: \"h1\", test: \"t\" }\n",
            "hypothesis H_2 { assumes: [F_2], predicts: \"h2\", test: \"t\" }\n",
            "hypothesis H_3 { assumes: [F_3], predicts: \"h3\", test: \"t\" }\n",
            "hypothesis H_4 { assumes: [F_4], predicts: \"h4\", test: \"t\" }\n",
            "proposition P_COND { statement: \"conditional\", from: [H_1, H_2, H_3, H_4] }\n",
            "warrant W_COND { claim: \"four hypotheses\", from: [H_1, H_2, H_3, H_4], to: P_COND }\n",
        ),
    )
    .expect("write the document");
    let shown = doc.to_str().unwrap().to_string();
    let cases = [
        ("engine", "more"),
        ("derive", "more"),
        ("debt", "more"),
        ("check", "more"),
    ];
    for (command, marker) in cases {
        let text = stdout(&run_in(root(), &[command, &shown]));
        assert!(
            text.contains(marker),
            "{command} did not say a list was cut:\n{text}"
        );
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn explain_does_not_promise_a_migration_command_that_does_not_exist() {
    // Defect: --explain E-VERSION told the reader to use "the format's migration command".
    // There is no such command in the dispatch table, and the format has never moved, so the
    // sentence named a repair the tool cannot perform.
    let text = tsare::explain::explain("E-VERSION").expect("explained");
    assert!(!text.contains("format's migration command"), "{text}");
    assert!(text.contains("No migration command exists"), "{text}");
}

#[test]
fn a_gap_with_no_separator_does_not_end_the_sentence_with_from() {
    // Defect: when the listed elements had no common attribute and no twin, the gap question
    // ended "no recorded attribute separates it from " -- the list it named was empty.
    let dir = temp("ask-gap");
    let doc = dir.join("gap.tsare");
    std::fs::write(
        &doc,
        concat!(
            "fact F_A { claim: \"a\", evidence: user, attrs: { a: 1 } }\n",
            "fact F_B { claim: \"b\", evidence: user, attrs: { b: 2 } }\n",
            "fact F_GAP { claim: \"g\", evidence: user }\n",
            "concept C_THINGS { name: \"things\", grounded_by: [F_A, F_B, F_GAP] }\n",
            "partition P_SOME { dimension: C_THINGS, elements: [F_A, F_B], closure: open }\n",
        ),
    )
    .expect("write the document");
    let shown = doc.to_str().unwrap().to_string();
    let text = stdout(&run_in(root(), &["ask", &shown]));
    assert!(text.contains("from any listed element"), "{text}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_confirmed_model_fact_can_license_a_rule() {
    // Defect: the basis criterion for a fact was `evidence != model`, ignoring `confirmed_by`.
    // E-SELF-VERDICT already treats a confirmed model fact as strong enough to settle a
    // hypothesis, while W-BASIS-NO-CRITERION called the same fact "a model's opinion".
    let source = concat!(
        "fact F { claim: \"x\", evidence: model, confirmed_by: \"reviewer\" }\n",
        "proposition P { statement: \"p\", from: [F] }\n",
        "warrant W { claim: \"r\", from: [F], to: P, basis: [F] }\n",
    );
    let found = tsare::check::check(&parse(source));
    assert!(
        !found.iter().any(|item| item.code == "W-BASIS-NO-CRITERION"),
        "{found:#?}"
    );
}

#[test]
fn a_cached_check_does_not_outlive_the_artifact_it_read() {
    // Defect: the cache key is the source bytes, but E-OUT-MISMATCH compares an artifact's
    // bytes. After the artifact changed, `check --cache` replayed the old clean report while a
    // plain check reported the mismatch: two answers for the same document and the same state.
    let dir = temp("cache-artifact");
    let artifact = dir.join("artifact.log");
    std::fs::write(&artifact, b"version one\n").expect("write the artifact");
    let digest = tsare::hash::hex(b"version one\n");
    std::fs::write(
        dir.join("doc.tsare"),
        format!(
            "fact F {{ claim: \"x\", evidence: tool(\"t\"), artifact: \"artifact.log\", out: \"{digest}\" }}\n"
        ),
    )
    .expect("write the document");
    let run = |extra: &[&str]| {
        let mut args = vec!["check", "doc.tsare", "--json"];
        args.extend_from_slice(extra);
        stdout(&run_in(&dir, &args))
    };
    let _miss = run(&["--cache"]);
    std::fs::write(&artifact, b"version two\n").expect("change the artifact");
    let cached = run(&["--cache"]);
    let plain = run(&[]);
    assert!(plain.contains("E-OUT-MISMATCH"), "{plain}");
    assert!(cached.contains("E-OUT-MISMATCH"), "{cached}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_uncovered_message_names_the_kinds_that_count() {
    // Defect: the message said goals are "addressed by no value or question", while
    // coverage::CLAIM_KINDS also counts a hypothesis, a concept, a proposition and a blind spot.
    let goals = tsare::coverage::load_goals("ship\tnothing matches this\n");
    let decls = parse("fact F { claim: \"x\", evidence: user }\n");
    let report = tsare::coverage::coverage(&tsare::program::Program::new(&decls), &goals);
    let diagnostics = tsare::coverage::diagnostics(&report);
    let message = &diagnostics[0].message;
    assert!(message.contains("commitment"), "{message}");
    assert!(!message.contains("value or question"), "{message}");
}

#[test]
fn a_withdrawal_cycle_is_one_error_and_it_names_the_loop() {
    // Defect: A withdraws B and B withdraws A, so both are retired; `check` reported only the
    // missing reasons, `engine` reported nothing held, and `derive` said every conclusion was
    // grounded.  The edge is not a dependency, so nothing walked the graph.
    let decls = parse(
        "fact F_R { claim: \"r\", evidence: user }\n\
         fact F_A { claim: \"a\", evidence: user, supersedes: [F_B], because: [F_R] }\n\
         fact F_B { claim: \"b\", evidence: user, supersedes: [F_A], because: [F_R] }\n",
    );
    let found = tsare::check::check(&decls);
    let cycles: Vec<_> = found
        .iter()
        .filter(|item| item.code == "E-SUPERSEDE-CYCLE")
        .collect();
    assert_eq!(cycles.len(), 1, "{found:#?}");
    assert!(cycles[0].message.contains("F_A"), "{}", cycles[0].message);
    assert!(cycles[0].message.contains("F_B"), "{}", cycles[0].message);
}

#[test]
fn a_longer_withdrawal_cycle_is_still_one_error() {
    let decls = parse(
        "fact F_R { claim: \"r\", evidence: user }\n\
         fact F_A { claim: \"a\", evidence: user, supersedes: [F_C], because: [F_R] }\n\
         fact F_B { claim: \"b\", evidence: user, supersedes: [F_A], because: [F_R] }\n\
         fact F_C { claim: \"c\", evidence: user, supersedes: [F_B], because: [F_R] }\n",
    );
    let found = tsare::check::check(&decls);
    let cycles: Vec<_> = found
        .iter()
        .filter(|item| item.code == "E-SUPERSEDE-CYCLE")
        .collect();
    assert_eq!(cycles.len(), 1, "{found:#?}");
    for name in ["F_A", "F_B", "F_C"] {
        assert!(cycles[0].message.contains(name), "{}", cycles[0].message);
    }
}

#[test]
fn a_withdrawal_chain_is_not_a_cycle() {
    let decls = parse(
        "fact F_R { claim: \"r\", evidence: user }\n\
         fact F_A { claim: \"a\", evidence: user }\n\
         fact F_B { claim: \"b\", evidence: user, supersedes: [F_A], because: [F_R] }\n\
         fact F_C { claim: \"c\", evidence: user, supersedes: [F_B], because: [F_R] }\n",
    );
    let found = tsare::check::check(&decls);
    assert!(
        found.iter().all(|item| item.code != "E-SUPERSEDE-CYCLE"),
        "{found:#?}"
    );
}
