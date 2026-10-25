from pathlib import Path

p = Path(r"D:\GitHub\engine\rust\engine-core\src\fix.rs")
t = p.read_text(encoding="utf-8")

start = t.index('        if decl.kind == "value".to_string() {')
end = t.index("    out.extend(from_diagnostics(decls));")

new = r'''        if let Some(fields) = crate::specs::spec(&decl.kind) {
            let missing: Vec<&str> = fields
                .iter()
                .filter(|(name, _, required)| {
                    *required
                        && !["claim", "evidence", "name", "to"].contains(name)
                        && !decl.fields.iter().any(|(key, _)| key == name)
                })
                .map(|(name, _, _)| *name)
                .collect();
            if !missing.is_empty() {
                let mut trial = decls.to_vec();
                for key in &missing {
                    trial = add_field(&trial, &decl.name, key, Val::Hole);
                }
                let after = score(&trial);
                if improves(before, after, &old_codes, &code_counts(&trial)) {
                    let listed = missing
                        .iter()
                        .map(|key| format!("`{key}: ?`"))
                        .collect::<Vec<_>>()
                        .join(", ");
                    out.push(Suggestion {
                        applicability: "HasPlaceholders",
                        message: format!("{}: add {listed}", decl.name),
                        file: decl.file.clone(),
                        line: decl.line,
                        col: decl.col,
                        end_line: decl.end_line,
                        end_col: decl.end_col,
                        replacement: String::new(),
                    });
                }
            }
        }
    }

'''

t = t[:start] + new + t[end:]
p.write_text(t, encoding="utf-8")
print("missing-field suggestions unified")
