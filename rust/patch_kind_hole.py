from pathlib import Path

p = Path(r"D:\GitHub\engine\rust\engine-core\src\check.rs")
t = p.read_text(encoding="utf-8")

# W-HOLE inside check_record's field loop
t = t.replace(
    '            Some((_, ftype, _)) => {\n                if !type_ok(ftype, value) {',
    '            Some((_, ftype, _)) => {\n'
    '                if matches!(value, Val::Hole) {\n'
    '                    out.push(Diagnostic {\n'
    '                        code: "W-HOLE",\n'
    '                        message: format!("{}.{key} is a hole: it compiles, but nothing can check it yet", decl.name),\n'
    '                        line,\n'
    '                        col,\n'
    '                        file: decl.file.clone(),\n'
    '                    });\n'
    '                }\n'
    '                if !type_ok(ftype, value) {',
)

# E-KIND: reference resolves to a declaration of another kind
t = t.replace(
    '''            let mut found = Vec::new();
            refs(value, &mut found);
            for name in found {
                if LITERALS.contains(&name.as_str()) || by_name.contains_key(name.as_str()) {
                    continue;
                }''',
    '''            let want = expected_kind(decl, key);
            let mut found = Vec::new();
            refs(value, &mut found);
            for name in found {
                if LITERALS.contains(&name.as_str()) {
                    continue;
                }
                if let Some(target_kind) = by_name.get(name.as_str()) {
                    if let Some(expected) = want {
                        if expected != "any" && target_kind != &expected {
                            out.push(diag(
                                "E-KIND",
                                format!("{}.{key} expects {expected}, but {name} is {target_kind}", decl.name),
                                decl,
                            ));
                        }
                    }
                    continue;
                }''',
)

t = t.replace(
    "fn check_cycles(",
    '''fn expected_kind(decl: &Decl, key: &str) -> Option<String> {
    let fields = spec(&decl.kind)?;
    let (_, ftype, _) = fields.iter().find(|(name, _, _)| name == key)?;
    let kind = ftype.strip_prefix("list:ref:").or_else(|| ftype.strip_prefix("ref:"))?;
    Some(kind.to_string())
}

fn check_cycles(''',
)

p.write_text(t, encoding="utf-8")
print("E-KIND and W-HOLE added")
