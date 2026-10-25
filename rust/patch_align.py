from pathlib import Path

p = Path(r"D:\GitHub\engine\rust\engine-core\src\check.rs")
t = p.read_text(encoding="utf-8")

start = t.index("fn check_cycles(")
end = t.index("fn check_collapse(")
new_cycles = r'''fn check_cycles(decls: &[Decl], out: &mut Vec<Diagnostic>) {
    let names: Vec<String> = decls.iter().map(|decl| decl.name.clone()).collect();
    let mut by_name: std::collections::HashMap<&str, &Decl> = std::collections::HashMap::new();
    for decl in decls {
        by_name.insert(decl.name.as_str(), decl);
    }
    let mut edges: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
    for decl in decls {
        let mut targets = Vec::new();
        for (key, value) in &decl.fields {
            if REF_KEYS.contains(&key.as_str()) {
                targets.extend(ids(value));
            }
        }
        targets.retain(|target| names.contains(target));
        edges.insert(decl.name.clone(), targets);
    }

    fn visit(
        node: &str,
        edges: &std::collections::HashMap<String, Vec<String>>,
        by_name: &std::collections::HashMap<&str, &Decl>,
        color: &mut std::collections::HashMap<String, u8>,
        stack: &mut Vec<String>,
        out: &mut Vec<Diagnostic>,
    ) {
        color.insert(node.to_string(), 1);
        stack.push(node.to_string());
        if let Some(targets) = edges.get(node) {
            for target in targets.clone() {
                match color.get(&target).copied().unwrap_or(0) {
                    0 => visit(&target, edges, by_name, color, stack, out),
                    1 => {
                        let position = stack.iter().position(|item| item == &target).unwrap_or(0);
                        let mut cycle = stack[position..].to_vec();
                        cycle.push(target.clone());
                        if let Some(decl) = by_name.get(node) {
                            out.push(diag(
                                "E-CYCLE",
                                format!("dependency cycle: {}", cycle.join(" -> ")),
                                decl,
                            ));
                        }
                    }
                    _ => {}
                }
            }
        }
        stack.pop();
        color.insert(node.to_string(), 2);
    }

    let mut color = std::collections::HashMap::new();
    for decl in decls {
        if color.get(&decl.name).copied().unwrap_or(0) == 0 {
            let mut stack = Vec::new();
            visit(&decl.name, &edges, &by_name, &mut color, &mut stack, out);
        }
    }
}

'''
t = t[:start] + new_cycles + t[end:]

start = t.index("fn check_collapse(")
new_collapse = r'''fn check_collapse(decls: &[Decl], out: &mut Vec<Diagnostic>) {
    let mut seen: std::collections::HashMap<(Vec<String>, Vec<String>), String> =
        std::collections::HashMap::new();
    for decl in decls.iter().filter(|decl| decl.kind == "concept") {
        let mut positives = field_ids(decl, "grounded_by");
        let mut negatives = field_ids(decl, "against");
        positives.sort();
        negatives.sort();
        match seen.get(&(positives.clone(), negatives.clone())) {
            Some(first) => out.push(diag(
                "E-COLLAPSE",
                format!(
                    "concepts {first} and {} have the same extent, so they are one concept with two names",
                    decl.name
                ),
                decl,
            )),
            None => {
                seen.insert((positives, negatives), decl.name.clone());
            }
        }
    }
}
'''
p.write_text(t[:start] + new_collapse, encoding="utf-8")
print("aligned with Python reporting")
