use super::*;

#[test]
fn a_syntax_error_is_a_diagnostic_with_a_span() {
    let error =
        parse("fact F { claim: \"unterminated\nevidence: user }").expect_err("must not parse");
    assert_eq!(error.line, 1);
    assert!(error.col > 10, "the caret points at the opening quote");
    assert!(error.message.contains("unterminated"));

    let error = parse("fact F { claim: \"x\", evidence: user").expect_err("missing brace");
    assert!(!error.message.is_empty(), "the error must say something");
    assert!(error.line >= 1 && error.col >= 1);
}

#[test]
fn parsing_reports_every_error_in_one_run() {
    let source = "fact F_OK { claim: \"x\", evidence: user }\n\
                  proposition P_BROKEN { statement: \"unterminated\n\
                  fact F_AFTER { claim: \"still parses\", evidence: user }\n\
                  concept C_BAD { name: \"c\" grounded_by: [F_OK] }\n";
    let (decls, errors) = tsare::syntax::parse_recovering(source);
    assert_eq!(
        errors.len(),
        3,
        "unterminated string, the field that follows it, the missing comma: {errors:?}"
    );
    let names: Vec<&str> = decls.iter().map(|decl| decl.name.as_str()).collect();
    assert!(
        names.contains(&"F_OK"),
        "a good declaration before the error survives"
    );
    assert!(
        names.contains(&"F_AFTER"),
        "parsing resumes at the next line-initial keyword"
    );
    assert!(
        !names.contains(&"C_BAD"),
        "the broken declaration is dropped, not half-read"
    );
}

#[test]
fn a_missing_separator_is_an_error() {
    let error = parse("fact F { claim: \"x\" evidence: user }").expect_err("no comma");
    assert!(
        error.message.contains(','),
        "the message names the separator: {}",
        error.message
    );
}
