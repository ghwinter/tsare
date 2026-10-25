from pathlib import Path

p = Path(r"D:\GitHub\engine\rust\engine-core\src\check.rs")
t = p.read_text(encoding="utf-8")

start = t.index("fn check_cycles(")
end = t.index("fn check_collapse(")

new = r'''fn check_cycles(decls: &[Decl], out: &mut Vec<Diagnostic>) {
    let names: Vec<String> = decls.iter().map(|decl| decl.name.clone()).collect();
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
        state: &mut std::collections::HashMap<String, u8>,
    ) -> bool {
        match state.get(node) {
            Some(1) => return true,
            Some(2) => return false,
            _ => {}
        }
        state.insert(node.to_string(), 1);
        let mut cyclic = false;
        if let Some(targets) = edges.get(node) {
            for target in targets.clone() {
                if visit(&target, edges, state) {
                    cyclic = true;
                }
            }
        }
        state.insert(node.to_string(), 2);
        cyclic
    }

    let mut cyclic = Vec::new();
    for decl in decls {
        let mut state = std::collections::HashMap::new();
        if visit(&decl.name, &edges, &mut state) {
            cyclic.push(decl);
        }
    }
    for decl in cyclic {
        out.push(diag("E-CYCLE", format!("{} is part of a reference cycle", decl.name), decl));
    }
}

'''

p.write_text(t[:start] + new + t[end:], encoding="utf-8")
print("cycles rewritten")
