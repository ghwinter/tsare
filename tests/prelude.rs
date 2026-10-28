//! The prelude: what this project admits, and one document that does not.
//!
//! These files are provisional the way an unstable ABI is provisional: they are the shape claims
//! are held to *right now*, they are compiled and tested, and they may be revised -- but a
//! revision has to show up in a diff and a failing test, not in a paragraph somewhere.
//!
//! `VIOLATION.tsare` is the counterexample.  A prelude that nothing ever fails against is
//! decoration, so the test below checks that checking against the kernel refuses it.

use std::path::{Path, PathBuf};

use tsare::check::check;
use tsare::syntax::parse;

fn prelude() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("prelude")
}

fn decls(name: &str) -> Vec<tsare::syntax::Decl> {
    let path = prelude().join(name);
    let source = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
    parse(&source).unwrap_or_else(|error| panic!("parse {}: {error:?}", path.display()))
}

#[test]
fn every_prelude_document_compiles() {
    for name in ["KERNEL.tsare", "WORLDVIEW.tsare", "PROCESS.tsare"] {
        let found = check(&decls(name));
        let errors: Vec<&str> = found
            .iter()
            .filter(|item| item.code.starts_with('E'))
            .map(|item| item.code)
            .collect();
        assert!(errors.is_empty(), "{name} has errors: {errors:?}");
    }
}

#[test]
fn the_kernel_refuses_the_document_that_violates_it() {
    let kernel = decls("KERNEL.tsare");
    let violation = decls("VIOLATION.tsare");
    let found = tsare::kernel::check(&kernel, &violation);
    assert!(
        found.iter().any(|item| item.code == "E-KERNEL-SHADOW"),
        "the prelude stopped refusing a redefined name: {found:#?}"
    );
    assert!(
        found.iter().any(|item| item.code == "W-UNPRINCIPLED"),
        "the prelude stopped asking a claim to stand on one of its laws: {found:#?}"
    );

    // and the merged program refuses it too, which is what `check --against` prints
    let mut merged = kernel;
    merged.extend(violation.clone());
    assert!(
        check(&merged).iter().any(|item| item.code.starts_with('E')),
        "the violation compiles in context"
    );
}

/// Every prelude file can only change through a diff and a red test.
///
/// Without this, `requires: ["measured_by"]` could be deleted from KERNEL.tsare and every other
/// test would still pass -- the violation document keeps producing the same two codes, because
/// `W-UNPRINCIPLED` alone already fires.  That was measured, not imagined: an agent removed the
/// line and ran the suite, and only this table can tell.  A revision is allowed; a quiet one is
/// not.
#[test]
fn the_prelude_cannot_change_without_a_red_test() {
    let pins = [
        (
            "KERNEL.tsare",
            "cbb322a95dd469c3d2c330c0e70ef574b93b0cdd26a3e1a9226144815c74f657",
        ),
        (
            "WORLDVIEW.tsare",
            "c84c4d6beff5de1642a441afb9d1747c728e96d7a961a5d7b246b1d7ae14a57e",
        ),
        (
            "PROCESS.tsare",
            "16ef2a8023bcb21b58541de6a3da1dd2e471e496c2874c4d4a9704d10bbb11a8",
        ),
        (
            "VIOLATION.tsare",
            "a3a86c658017b74459005db0dd07f618ac2035570480c3c2cea80d6fe2da586d",
        ),
    ];
    for (name, expected) in pins {
        let bytes = std::fs::read(prelude().join(name)).expect("read a prelude file");
        assert_eq!(
            tsare::hash::hex(&bytes),
            expected,
            "{name} changed: update this constant in the same commit, and say in the message \
             whether the claim it carries got weaker"
        );
    }
}

/// A witness is a witness.  `evidence: observation` is somebody having looked; it used to be graded
/// below `user` and therefore could not settle a hypothesis, which is the opposite of the intent.
#[test]
fn an_observation_can_settle_a_hypothesis_but_a_model_cannot() {
    let observed = tsare::syntax::parse(concat!(
        "fact F_SEEN { claim: \"the log shows one run\", evidence: observation }\n",
        "hypothesis H_ONE { assumes: [F_SEEN], predicts: \"p\", verdict: refuted, verdict_by: F_SEEN }\n",
    ))
    .expect("parses");
    let found = check(&observed);
    assert!(
        !found.iter().any(|item| item.code == "E-SELF-VERDICT"),
        "an observation must be able to settle a hypothesis: {found:#?}"
    );

    let guessed = tsare::syntax::parse(concat!(
        "fact F_GUESS { claim: \"the log shows one run\", evidence: model }\n",
        "hypothesis H_ONE { assumes: [F_GUESS], predicts: \"p\", verdict: refuted, verdict_by: F_GUESS }\n",
    ))
    .expect("parses");
    assert!(check(&guessed)
        .iter()
        .any(|item| item.code == "E-SELF-VERDICT"));
}

/// A declaration that supersedes itself is withdrawn from the fact set and leaves nothing behind.
#[test]
fn nothing_may_supersede_itself() {
    let source = "fact F_S { claim: \"x\", evidence: user, supersedes: [F_S] }\n";
    let found = check(&tsare::syntax::parse(source).expect("parses"));
    assert!(
        found.iter().any(|item| item.code == "E-SUPERSEDE-UNDEFINED"
            && item.message.contains("supersedes itself")),
        "{found:#?}"
    );
}
