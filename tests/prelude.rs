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
