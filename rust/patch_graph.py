from pathlib import Path

p = Path(r"D:\GitHub\engine\rust\engine-core\src\check.rs")
t = p.read_text(encoding="utf-8")

t += r'''

fn ids(value: &Val) -> Vec<String> {
    let mut out = Vec::new();
    refs(value, &mut out);
    out
}

fn field_ids(decl: &Decl, key: &str) -> Vec<String> {
    decl.fields
        .iter()
        .filter(|(name, _)| name == key)
        .flat_map(|(_, value)| ids(value))
        .collect()
}

fn check_cycles(decls: &[Decl], out: &mut Vec<Diagnostic>) {
    let names: Vec<&str> = decls.iter().map(|decl| decl.name.as_str()).collect();
    let mut edges: std::collections::HashMap<&str, Vec<&str>> = std::collections::HashMap::new();
    for decl in decls {
        let mut targets = Vec::new();
        for (key, value) in &decl.fields {
            if REF_KEYS.contains(&key.as_str()) {
                targets.extend(ids(value));
            }
        }
        edges.insert(
            decl.name.as_str(),
            targets
                .into_iter()
                .filter(|target| names.contains(&target.as_str()))
                .collect(),
        );
    }
    let mut state: std::collections::HashMap<&str, u8> = std::collections::HashMap::new();
    fn visit<'a>(
        node: &'a str,
        edges: &std::collections::HashMap<&'a str, Vec<&'a str>>,
        state: &mut std::collections::HashMap<&'a str, u8>,
    ) -> bool {
        match state.get(node) {
            Some(1) => return true,
            Some(2) => return false,
            _ => {}
        }
        state.insert(node, 1);
        let mut cyclic = false;
        if let Some(targets) = edges.get(node) {
            for target in targets.clone() {
                if visit(target, edges, state) {
                    cyclic = true;
                }
            }
        }
        state.insert(node, 2);
        cyclic
    }
    let mut cyclic_nodes = Vec::new();
    for decl in decls {
        let mut local = state.clone();
        if visit(decl.name.as_str(), &edges, &mut local) {
            cyclic_nodes.push(decl);
        }
        state.extend(local);
    }
    for decl in cyclic_nodes {
        out.push(diag("E-CYCLE", format!("{} is part of a reference cycle", decl.name), decl));
    }
}

fn check_collapse(decls: &[Decl], out: &mut Vec<Diagnostic>) {
    let mut groups: std::collections::HashMap<(Vec<String>, Vec<String>), Vec<&Decl>> =
        std::collections::HashMap::new();
    for decl in decls.iter().filter(|decl| decl.kind == "concept") {
        let mut positives = field_ids(decl, "grounded_by");
        let mut negatives = field_ids(decl, "against");
        positives.sort();
        negatives.sort();
        groups.entry((positives, negatives)).or_default().push(decl);
    }
    for ((positives, negatives), members) in groups {
        if members.len() < 2 {
            continue;
        }
        for decl in members {
            let others: Vec<&str> = decls
                .iter()
                .filter(|other| {
                    other.kind == "concept"
                        && other.name != decl.name
                        && {
                            let mut p = field_ids(other, "grounded_by");
                            let mut n = field_ids(other, "against");
                            p.sort();
                            n.sort();
                            p == positives && n == negatives
                        }
                })
                .map(|other| other.name.as_str())
                .collect();
            out.push(diag(
                "E-COLLAPSE",
                format!("{} and {} have the same extent", decl.name, others.join(", ")),
                decl,
            ));
        }
    }
}
'''

t = t.replace(
    "    let mut seen: HashSet<(String, String)> = HashSet::new();",
    "    check_cycles(decls, &mut out);\n    check_collapse(decls, &mut out);\n\n    let mut seen: HashSet<(String, String)> = HashSet::new();",
)

p.write_text(t, encoding="utf-8")
print("cycles and collapse added")
