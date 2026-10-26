use cogent::check;
use cogent::syntax::parse;

#[test]
fn parses_a_fact() {
    let decls = parse("fact F { claim: \"x\", evidence: user }\n").unwrap();
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].kind, "fact");
    assert_eq!(decls[0].name, "F");
}

#[test]
fn reports_a_duplicate() {
    let decls =
        parse("fact F { claim: \"a\", evidence: user }\nfact F { claim: \"b\", evidence: user }\n")
            .unwrap();
    let diagnostics = check::check(&decls);
    assert!(diagnostics.iter().any(|d| d.code == "E-DUP-ID"));
}

#[test]
fn reports_an_unknown_reference() {
    let decls = parse("proposition P { statement: \"p\", from: [NOPE] }\n").unwrap();
    let diagnostics = check::check(&decls);
    assert!(diagnostics.iter().any(|d| d.code == "E-UNDEFINED"));
}

#[test]
fn a_named_reference_resolves() {
    let source = "fact F { claim: \"x\", evidence: user }\nproposition P { statement: \"p\", from: [F] }\nwarrant W { claim: \"r\", from: [F], to: P, basis: [F] }\n";
    let decls = parse(source).unwrap();
    let diagnostics = check::check(&decls);
    assert!(!diagnostics.iter().any(|item| item.code.starts_with("E-")));
}
