from pathlib import Path

p = Path(r"D:\GitHub\engine\rust\engine-core\src\check.rs")
t = p.read_text(encoding="utf-8")

old = """    for (key, value) in &decl.fields {
        match fields.iter().find(|(name, _, _)| name == key) {
            Some((_, ftype, _)) => {
                if !type_ok(ftype, value) {
                    out.push(diag("E-TYPE", format!("{}.{key}: expected {ftype}", decl.name), decl));
                }
            }
            None => out.push(diag("E-FIELD", format!("{} has no field {key}", decl.kind), decl)),
        }
    }"""

new = """    for (index, (key, value)) in decl.fields.iter().enumerate() {
        let (line, col) = decl
            .field_pos
            .get(index)
            .copied()
            .unwrap_or((decl.line, decl.col));
        match fields.iter().find(|(name, _, _)| name == key) {
            Some((_, ftype, _)) => {
                if !type_ok(ftype, value) {
                    out.push(Diagnostic {
                        code: "E-TYPE",
                        message: format!("{}.{key}: expected {ftype}", decl.name),
                        line,
                        col,
                    });
                }
            }
            None => out.push(Diagnostic {
                code: "E-FIELD",
                message: format!("{} has no field {key}", decl.kind),
                line,
                col,
            }),
        }
    }"""

assert old in t
p.write_text(t.replace(old, new), encoding="utf-8")
print("check.rs updated")
