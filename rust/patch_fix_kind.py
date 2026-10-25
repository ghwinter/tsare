from pathlib import Path

p = Path(r"D:\GitHub\engine\rust\engine-core\src\fix.rs")
t = p.read_text(encoding="utf-8")

t = t.replace(
    "pub fn suggestions(decls: &[Decl]) -> Vec<Suggestion> {",
    r'''fn clone_with_list(decls: &[Decl], owner: &str, key: &str, old: &str, new: &str) -> Option<Vec<Decl>> {
    let mut out = Vec::new();
    let mut touched = false;
    for decl in decls {
        let mut copy = decl.clone();
        if decl.name == owner {
            for (name, value) in copy.fields.iter_mut() {
                if name != key {
                    continue;
                }
                if let Val::List(items) = value {
                    let mut replaced = false;
                    let mapped: Vec<Val> = items
                        .iter()
                        .map(|item| match item {
                            Val::Id(token) if token == old => {
                                replaced = true;
                                Val::Id(new.to_string())
                            }
                            other => other.clone(),
                        })
                        .collect();
                    if replaced {
                        *value = Val::List(mapped);
                        touched = true;
                    }
                }
            }
        }
        out.push(copy);
    }
    if touched { Some(out) } else { None }
}

fn kind_map(decls: &[Decl]) -> std::collections::HashMap<&str, &str> {
    decls.iter().map(|decl| (decl.name.as_str(), decl.kind.as_str())).collect()
}

pub fn from_diagnostics(decls: &[Decl]) -> Vec<Suggestion> {
    let before = score(decls);
    let diagnostics = check(decls);
    let kinds = kind_map(decls);
    let mut out = Vec::new();
    for item in &diagnostics {
        if item.code != "E-KIND" {
            continue;
        }
        let message = &item.message;
        let Some(dot) = message.find('.') else { continue };
        let Some(space) = message.find(" expects ") else { continue };
        let owner = &message[..dot];
        let key = &message[dot + 1..space];
        let rest = &message[space + " expects ".len()..];
        let Some(comma) = rest.find(", but ") else { continue };
        let want = &rest[..comma];
        let tail = &rest[comma + ", but ".len()..];
        let Some(space2) = tail.find(" is ") else { continue };
        let target = &tail[..space2];
        let Some(replacement) = decls.iter().find(|decl| decl.kind == want && decl.name != target) else {
            continue;
        };
        let Some(trial) = clone_with_list(decls, owner, key, target, &replacement.name) else {
            continue;
        };
        let after = score(&trial);
        if after.0 >= before.0 {
            continue;
        }
        out.push(Suggestion {
            applicability: "MaybeIncorrect",
            message: format!("{owner}: replace {target} in {key}"),
            file: item.file.clone(),
            line: item.line,
            col: item.col,
            end_line: item.line,
            end_col: item.col,
            replacement: format!("{owner}: replace {target} in {key}"),
        });
    }
    out
}

pub fn suggestions(decls: &[Decl]) -> Vec<Suggestion> {''',
)

t = t.replace(
    "    out\n}\n",
    "    out.extend(from_diagnostics(decls));\n    out\n}\n",
)

p.write_text(t, encoding="utf-8")
print("kind repairs added")
