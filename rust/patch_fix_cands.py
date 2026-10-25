from pathlib import Path

p = Path(r"D:\GitHub\engine\rust\engine-core\src\fix.rs")
t = p.read_text(encoding="utf-8")

t = t.replace(
    "pub fn suggestions(decls: &[Decl]) -> Vec<Suggestion> {",
    r'''fn code_counts(decls: &[Decl]) -> std::collections::HashMap<String, usize> {
    let mut counts = std::collections::HashMap::new();
    for item in check(decls) {
        *counts.entry(item.code.to_string()).or_insert(0) += 1;
    }
    counts
}

fn improves(before: (usize, usize), after: (usize, usize), old: &std::collections::HashMap<String, usize>, new: &std::collections::HashMap<String, usize>) -> bool {
    if after.0 > before.0 {
        return false;
    }
    old.iter().any(|(code, count)| new.get(code).copied().unwrap_or(0) < *count)
}

fn add_field(decls: &[Decl], owner: &str, key: &str, value: Val) -> Vec<Decl> {
    let mut out = Vec::new();
    for decl in decls {
        let mut copy = decl.clone();
        if decl.name == owner && !copy.fields.iter().any(|(name, _)| name == key) {
            copy.fields.push((key.to_string(), value));
        }
        out.push(copy);
    }
    out
}

fn add_warrant(decls: &[Decl], proposition: &Decl) -> Vec<Decl> {
    let premises = crate::closure::field_ids(proposition, "from");
    let mut copy = proposition.clone();
    copy.kind = "warrant".to_string();
    copy.name = format!("W_AUTO_{}", proposition.name);
    copy.fields = vec![
        ("claim".to_string(), Val::Hole),
        (
            "from".to_string(),
            Val::List(premises.into_iter().map(Val::Id).collect()),
        ),
        ("to".to_string(), Val::Id(proposition.name.clone())),
    ];
    let mut out: Vec<Decl> = decls.to_vec();
    out.push(copy);
    out
}

pub fn suggestions(decls: &[Decl]) -> Vec<Suggestion> {''',
)

t = t.replace(
    "    out.extend(from_diagnostics(decls));\n    out\n}",
    r'''    let before = score(decls);
    let old_codes = code_counts(decls);
    let warranted: std::collections::HashSet<String> = decls
        .iter()
        .filter(|decl| decl.kind == "warrant")
        .flat_map(|decl| crate::closure::field_ids(decl, "to"))
        .collect();

    for decl in decls {
        if decl.kind == "proposition"
            && !crate::closure::field_ids(decl, "from").is_empty()
            && !warranted.contains(&decl.name)
        {
            let trial = add_warrant(decls, decl);
            let after = score(&trial);
            if improves(before, after, &old_codes, &code_counts(&trial)) {
                out.push(Suggestion {
                    applicability: "HasPlaceholders",
                    message: format!("{}: add a warrant over its premises", decl.name),
                    file: decl.file.clone(),
                    line: decl.line,
                    col: decl.col,
                    end_line: decl.end_line,
                    end_col: decl.end_col,
                    replacement: "",
                });
            }
        }
        if decl.kind == "value" {
            for key in ["unit", "threshold"] {
                if decl.fields.iter().any(|(name, _)| name == key) {
                    continue;
                }
                let trial = add_field(decls, &decl.name, key, Val::Hole);
                let after = score(&trial);
                if improves(before, after, &old_codes, &code_counts(&trial)) {
                    out.push(Suggestion {
                        applicability: "HasPlaceholders",
                        message: format!("{}: add `{key}: ?`", decl.name),
                        file: decl.file.clone(),
                        line: decl.line,
                        col: decl.col,
                        end_line: decl.end_line,
                        end_col: decl.end_col,
                        replacement: "",
                    });
                }
            }
        }
    }

    out.extend(from_diagnostics(decls));
    out
}''',
)

p.write_text(t, encoding="utf-8")
print("repair candidates added")
