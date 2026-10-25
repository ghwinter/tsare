from pathlib import Path

p = Path(r"D:\GitHub\engine\rust\engine-core\src\closure.rs")
t = p.read_text(encoding="utf-8")

t += r'''

pub struct EngineResult {
    pub held: Vec<String>,
    pub conditional: Vec<(String, Vec<String>)>,
    pub blocked: Vec<(String, Vec<String>)>,
    pub chains: Vec<(String, String)>,
    pub gates: Vec<(String, Vec<String>)>,
}

fn evidence_status(decl: &Decl) -> String {
    for (name, value) in &decl.fields {
        if name != "evidence" {
            continue;
        }
        match value {
            Val::Id(token) => {
                return match token.as_str() {
                    "model" => {
                        if decl.fields.iter().any(|(key, _)| key == "confirmed_by") {
                            "confirmed".to_string()
                        } else {
                            "unconfirmed".to_string()
                        }
                    }
                    other => other.to_string(),
                }
            }
            Val::Call(call, _) => {
                if call == "tool" || call == "doc" || call == "code" {
                    return "measured".to_string();
                }
            }
            _ => {}
        }
    }
    "asserted".to_string()
}

fn support_edges(decls: &[Decl]) -> HashMap<String, Vec<String>> {
    let mut edges: HashMap<String, Vec<String>> = HashMap::new();
    for decl in decls {
        match decl.kind.as_str() {
            "warrant" => {
                if let Some(target) = id_field(decl, "to") {
                    edges.entry(target.to_string()).or_default().extend(field_ids(decl, "from"));
                }
            }
            "proposition" => {
                edges.entry(decl.name.clone()).or_default().extend(field_ids(decl, "from"));
            }
            "hypothesis" => {
                edges.entry(decl.name.clone()).or_default().extend(field_ids(decl, "assumes"));
            }
            _ => {}
        }
    }
    edges
}

pub fn engine(decls: &[Decl]) -> EngineResult {
    let inside = grounded(decls);
    let facts: HashSet<String> =
        decls.iter().filter(|decl| decl.kind == "fact").map(|decl| decl.name.clone()).collect();
    let rules: Vec<Rule> = applications(decls)
        .into_iter()
        .filter(|rule| inside.contains(&rule.warrant))
        .collect();

    let mut held = facts.clone();
    let mut parent: HashMap<String, (String, Vec<String>)> = HashMap::new();
    let mut applied: HashSet<(String, String)> = HashSet::new();
    let mut changed = true;
    while changed {
        changed = false;
        for rule in &rules {
            if applied.contains(&(rule.warrant.clone(), rule.target.clone())) {
                continue;
            }
            if rule.premises.is_empty() {
                continue;
            }
            if rule.premises.iter().all(|premise| held.contains(premise)) {
                applied.insert((rule.warrant.clone(), rule.target.clone()));
                if !held.contains(&rule.target) {
                    held.insert(rule.target.clone());
                    parent.insert(rule.target.clone(), (rule.warrant.clone(), rule.premises.clone()));
                    changed = true;
                }
            }
        }
    }

    let state = statuses(decls, &inside);
    let mut conditional = Vec::new();
    for (name, status) in &state {
        if status != "supported" || held.contains(name) {
            continue;
        }
        let mut reasons = Vec::new();
        for rule in rules.iter().filter(|rule| &rule.target == name) {
            for premise in &rule.premises {
                if !held.contains(premise) {
                    let premise_state = state.get(premise).cloned().unwrap_or_else(|| "fact".to_string());
                    let label = if premise_state == "defeated" { "defeated".to_string() } else { premise_state };
                    reasons.push(format!("{} needs {premise} ({label})", rule.warrant));
                }
            }
        }
        if reasons.is_empty() {
            reasons.push("no live rule reached a fact".to_string());
        }
        conditional.push((name.clone(), reasons));
    }

    let mut blocked = Vec::new();
    for rule in &rules {
        if held.contains(&rule.target) {
            continue;
        }
        let missing: Vec<String> =
            rule.premises.iter().filter(|premise| !held.contains(*premise)).cloned().collect();
        if !missing.is_empty() {
            blocked.push((rule.warrant.clone(), missing));
        }
    }

    fn chain(
        name: &str,
        facts: &HashSet<String>,
        parent: &HashMap<String, (String, Vec<String>)>,
        depth: usize,
    ) -> String {
        if facts.contains(name) || depth > 12 {
            return name.to_string();
        }
        match parent.get(name) {
            Some((warrant, premises)) if !premises.is_empty() => {
                let parts: Vec<String> =
                    premises.iter().map(|premise| chain(premise, facts, parent, depth + 1)).collect();
                format!("{name} <- {warrant} <- {}", parts.join(" & "))
            }
            _ => name.to_string(),
        }
    }

    let mut chains = Vec::new();
    let mut held_names: Vec<String> = held.iter().filter(|name| !facts.contains(*name)).cloned().collect();
    held_names.sort();
    for name in &held_names {
        chains.push((name.clone(), chain(name, &facts, &parent, 0)));
    }

    let edges = support_edges(decls);
    let by_name: HashMap<&str, &Decl> = decls.iter().map(|decl| (decl.name.as_str(), decl)).collect();
    let mut gates = Vec::new();
    for name in &held_names {
        let mut seen: HashSet<String> = HashSet::new();
        let mut queue: Vec<String> = vec![name.clone()];
        let mut weak = Vec::new();
        while let Some(current) = queue.pop() {
            for premise in edges.get(&current).cloned().unwrap_or_default() {
                if !seen.insert(premise.clone()) {
                    continue;
                }
                match by_name.get(premise.as_str()) {
                    Some(decl) if decl.kind == "fact" => {
                        let status = evidence_status(decl);
                        if status != "measured" {
                            weak.push(format!("{premise} ({status})"));
                        }
                    }
                    Some(_) => queue.push(premise),
                    None => {}
                }
            }
        }
        gates.push((name.clone(), weak));
    }

    EngineResult { held: held_names, conditional, blocked, chains, gates }
}
'''

p.write_text(t, encoding="utf-8")
print("engine result added")
