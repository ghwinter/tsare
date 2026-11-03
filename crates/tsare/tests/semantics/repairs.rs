use super::*;

#[test]
fn identical_facts_merge() {
    let source = r#"
fact F1 { claim: "x", evidence: user }
fact F2 { claim: "x", evidence: user }
"#;
    let declared = decls(source);
    let (_, groups) = merge_groups(&Program::new(&declared));
    let merged: Vec<&Vec<String>> = groups.iter().filter(|group| group.len() > 1).collect();
    assert_eq!(merged.len(), 1);
    assert_eq!(merged[0], &vec!["F1".to_string(), "F2".to_string()]);
}

#[test]
fn a_duplicate_is_a_machine_repair() {
    let text = "fact F { claim: \"a\", evidence: user }\nfact F { claim: \"b\", evidence: user }\n";
    let declared = decls(text);
    let items = suggestions(&declared);
    let drop = items
        .iter()
        .find(|item| item.message.contains("drop the duplicate"))
        .expect("a duplicate can be dropped");
    assert_eq!(drop.applicability, "MachineApplicable");
    let (updated, applied) = tsare::apply::apply_best(text, &items);
    assert_eq!(applied.len(), 1);
    assert!(
        !updated.contains("\"b\""),
        "the later duplicate is the one that goes"
    );
    let before = tsare::apply::score(text);
    let after = tsare::apply::score(&updated);
    assert!(after < before, "the applied text compiles better");
}

#[test]
fn a_repair_that_does_not_improve_is_dropped() {
    let text = "fact F { claim: \"a\", evidence: user }\n";
    let declared = decls(text);
    let noop = tsare::fix::Suggestion {
        applicability: "MachineApplicable",
        message: "no-op".to_string(),
        file: String::new(),
        line: declared[0].line,
        col: declared[0].col,
        end_line: declared[0].end_line,
        end_col: declared[0].end_col,
        replacement: String::new(),
    };
    let (updated, applied) = tsare::apply::apply_best(text, &[noop]);
    assert!(applied.is_empty());
    assert_eq!(updated, text);
}

/// A verified repair may create the declaration it needs -- adding a rule to connect a
/// proposition creates the rule, and the rule can owe something of its own.  What it may not do is
/// put a new complaint on a declaration that was already there.  A total score cannot tell those
/// two apart, which is the whole reason the judgment moved to (declaration, code) pairs.
#[test]
fn a_verified_repair_does_not_borrow_trouble() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures");
    let pairs = |list: &[Decl]| -> HashSet<(String, &'static str)> {
        check(list)
            .into_iter()
            .filter(|item| item.code != "W-HOLE")
            .filter_map(|item| item.witness.map(|name| (name, item.code)))
            .collect()
    };
    for entry in std::fs::read_dir(&root).expect("fixtures") {
        let path = entry.expect("entry").path();
        if path.extension().map(|ext| ext != "tsare").unwrap_or(true) {
            continue;
        }
        let source = std::fs::read_to_string(&path).expect("read a fixture");
        let declared = decls(&source);
        let known: HashSet<&str> = declared.iter().map(|item| item.name.as_str()).collect();
        let before = pairs(&declared);
        let (accepted, _) = tsare::fix::verified(&declared);
        for fix in accepted {
            let after = pairs(&fix.trial);
            for (declaration, code) in after.difference(&before) {
                assert!(
                    !known.contains(declaration.as_str()),
                    "{}: {} adds {code} to the existing {declaration}",
                    path.display(),
                    fix.description
                );
            }
        }
    }
}

/// The `E-KIND` repair parses its own diagnostic message; a quoted name containing ` expects `
/// put the marker before the dot and the reversed slice panicked.
#[test]
fn a_quoted_name_with_a_marker_does_not_panic_the_repair() {
    let source = concat!(
        "fact F_A { claim: \"x\", evidence: user }\n",
        "warrant \"C expects D\" { claim: \"c\", from: [F_A], to: F_A }\n",
    );
    let _ = suggestions(&decls(source));
}
