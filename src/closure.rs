//! Grounded semantics: which rules survive attack, and which conclusions stand on facts.

use std::collections::{HashMap, HashSet};

use crate::syntax::{Decl, Val};

fn ids(value: &Val, out: &mut Vec<String>) {
    match value {
        Val::Id(name) => out.push(name.clone()),
        Val::List(items) => items.iter().for_each(|item| ids(item, out)),
        Val::Call(_, args) => args.iter().for_each(|item| ids(item, out)),
        Val::Rec(fields) => fields.iter().for_each(|(_, item)| ids(item, out)),
        _ => {}
    }
}

pub fn field_ids(decl: &Decl, key: &str) -> Vec<String> {
    let mut out = Vec::new();
    for (name, value) in &decl.fields {
        if name == key {
            ids(value, &mut out);
        }
    }
    out
}

pub fn id_field<'a>(decl: &'a Decl, key: &str) -> Option<&'a str> {
    for (name, value) in &decl.fields {
        if name == key {
            if let Val::Id(text) = value {
                return Some(text.as_str());
            }
        }
    }
    None
}

fn holds(value: &Val, pattern: &Val) -> bool {
    match pattern {
        Val::List(items) => items.iter().all(|item| holds(value, item)),
        _ => match value {
            Val::List(items) => items.iter().any(|item| holds(item, pattern)),
            Val::Id(left) => matches!(pattern, Val::Id(right) if left == right),
            Val::Str(left) => matches!(pattern, Val::Str(right) if left == right),
            Val::Num(left) => matches!(pattern, Val::Num(right) if left == right),
            _ => false,
        },
    }
}

fn matches(decl: &Decl, record: &[(String, Val)]) -> bool {
    record.iter().all(|(key, pattern)| {
        decl.fields
            .iter()
            .find(|(name, _)| name == key)
            .map(|(_, value)| holds(value, pattern))
            .unwrap_or(false)
    })
}

pub struct Rule {
    pub warrant: String,
    pub premises: Vec<String>,
    pub target: String,
}

pub fn applications(decls: &[Decl]) -> Vec<Rule> {
    let mut out = Vec::new();
    for decl in decls.iter().filter(|decl| decl.kind == "warrant") {
        let premises = field_ids(decl, "from");
        if let Some(target) = id_field(decl, "to") {
            out.push(Rule {
                warrant: decl.name.clone(),
                premises,
                target: target.to_string(),
            });
            continue;
        }
        let record = decl.fields.iter().find(|(name, _)| name == "concludes");
        if let Some((_, Val::Rec(fields))) = record {
            for proposition in decls.iter().filter(|decl| decl.kind == "proposition") {
                if matches(proposition, fields) {
                    out.push(Rule {
                        warrant: decl.name.clone(),
                        premises: premises.clone(),
                        target: proposition.name.clone(),
                    });
                }
            }
        }
    }
    out
}

pub fn grounded(decls: &[Decl]) -> HashSet<String> {
    grounded_sets(decls).0
}

/// Names withdrawn by some `supersedes:` field.  A retired declaration stays in the document --
/// references to it still resolve -- but it counts for nothing: it grounds nothing, supports
/// nothing, attacks nothing.  That is what makes withdrawal an append instead of an edit, and
/// what lets a reader see what used to be believed.
pub fn retired(decls: &[Decl]) -> HashSet<String> {
    decls
        .iter()
        .flat_map(|decl| field_ids(decl, "supersedes"))
        .collect()
}

/// The grounded extension, plus the rules it defeats.  `undecided` is the rest.
pub fn grounded_sets(decls: &[Decl]) -> (HashSet<String>, HashSet<String>) {
    let warrants: Vec<&Decl> = decls.iter().filter(|decl| decl.kind == "warrant").collect();
    let gone = retired(decls);
    let invalid: HashSet<String> = warrants
        .iter()
        .filter(|decl| field_ids(decl, "from").is_empty())
        .map(|decl| decl.name.clone())
        // a retired rule is still declared and no longer holds a position
        .chain(gone.iter().cloned())
        .collect();

    let mut attackers: HashMap<String, Vec<String>> = HashMap::new();
    for decl in &warrants {
        if invalid.contains(&decl.name) {
            continue;
        }
        for target in field_ids(decl, "attacks") {
            attackers.entry(target).or_default().push(decl.name.clone());
        }
    }

    let names: Vec<String> = warrants.iter().map(|decl| decl.name.clone()).collect();
    let mut outside: HashSet<String> = invalid.clone();
    let mut inside: HashSet<String> = HashSet::new();
    loop {
        let new_inside: HashSet<String> = names
            .iter()
            .filter(|name| {
                !invalid.contains(*name)
                    && attackers
                        .get(*name)
                        .map(|list| list.iter().all(|attacker| outside.contains(attacker)))
                        .unwrap_or(true)
            })
            .cloned()
            .collect();
        let new_outside: HashSet<String> = invalid
            .iter()
            .cloned()
            .chain(
                names
                    .iter()
                    .filter(|name| {
                        attackers
                            .get(*name)
                            .map(|list| list.iter().any(|attacker| new_inside.contains(attacker)))
                            .unwrap_or(false)
                    })
                    .cloned(),
            )
            .collect();
        if new_inside == inside && new_outside == outside {
            break;
        }
        inside = new_inside;
        outside = new_outside;
    }
    (inside, outside)
}

pub fn statuses(decls: &[Decl], inside: &HashSet<String>) -> HashMap<String, String> {
    let mut by_conclusion: HashMap<String, Vec<String>> = HashMap::new();
    for decl in decls.iter().filter(|decl| decl.kind == "warrant") {
        if let Some(target) = id_field(decl, "to") {
            by_conclusion
                .entry(target.to_string())
                .or_default()
                .push(decl.name.clone());
        }
    }
    let mut out = HashMap::new();
    for decl in decls.iter().filter(|decl| decl.kind == "proposition") {
        let state = match by_conclusion.get(&decl.name) {
            None => "asserted",
            Some(rules) if rules.iter().any(|rule| inside.contains(rule)) => "supported",
            Some(rules) if rules.iter().all(|rule| !inside.contains(rule)) => "defeated",
            Some(_) => "contested",
        };
        out.insert(decl.name.clone(), state.to_string());
    }
    out
}

pub struct Closure {
    pub facts: HashSet<String>,
    pub held: HashSet<String>,
    pub conditional: Vec<(String, String)>,
}

pub fn closure(decls: &[Decl]) -> Closure {
    let inside = grounded(decls);
    let gone = retired(decls);
    let facts: HashSet<String> = decls
        .iter()
        .filter(|decl| decl.kind == "fact" && !gone.contains(&decl.name))
        .map(|decl| decl.name.clone())
        .collect();
    let mut held = facts.clone();
    let rules: Vec<Rule> = applications(decls)
        .into_iter()
        .filter(|rule| inside.contains(&rule.warrant) && !gone.contains(&rule.warrant))
        .collect();
    let mut changed = true;
    while changed {
        changed = false;
        for rule in &rules {
            if held.contains(&rule.target) {
                continue;
            }
            if !rule.premises.is_empty()
                && rule.premises.iter().all(|premise| held.contains(premise))
            {
                held.insert(rule.target.clone());
                changed = true;
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
                    let premise_state = if gone.contains(premise) {
                        "retired".to_string()
                    } else {
                        state
                            .get(premise)
                            .cloned()
                            .unwrap_or_else(|| "fact".to_string())
                    };
                    let label = if premise_state == "defeated" {
                        "defeated".to_string()
                    } else {
                        premise_state
                    };
                    reasons.push(format!("{} needs {premise} ({label})", rule.warrant));
                }
            }
        }
        if reasons.is_empty() {
            reasons.push("no live rule reached a fact".to_string());
        }
        reasons.truncate(2);
        conditional.push((name.clone(), reasons.join("; ")));
    }
    Closure {
        facts,
        held,
        conditional,
    }
}

pub struct EngineResult {
    pub held: Vec<String>,
    pub conditional: Vec<(String, Vec<String>)>,
    pub blocked: Vec<(String, Vec<String>)>,
    pub chains: Vec<(String, String)>,
    pub gates: Vec<(String, Vec<String>)>,
}

fn support_edges(decls: &[Decl]) -> HashMap<String, Vec<String>> {
    let mut edges: HashMap<String, Vec<String>> = HashMap::new();
    for decl in decls {
        match decl.kind.as_str() {
            "warrant" => {
                if let Some(target) = id_field(decl, "to") {
                    edges
                        .entry(target.to_string())
                        .or_default()
                        .extend(field_ids(decl, "from"));
                }
            }
            "proposition" => {
                edges
                    .entry(decl.name.clone())
                    .or_default()
                    .extend(field_ids(decl, "from"));
            }
            "hypothesis" => {
                edges
                    .entry(decl.name.clone())
                    .or_default()
                    .extend(field_ids(decl, "assumes"));
            }
            _ => {}
        }
    }
    edges
}

pub fn engine(decls: &[Decl]) -> EngineResult {
    let inside = grounded(decls);
    let gone = retired(decls);
    let facts: HashSet<String> = decls
        .iter()
        .filter(|decl| decl.kind == "fact" && !gone.contains(&decl.name))
        .map(|decl| decl.name.clone())
        .collect();
    let rules: Vec<Rule> = applications(decls)
        .into_iter()
        .filter(|rule| inside.contains(&rule.warrant) && !gone.contains(&rule.warrant))
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
                    parent.insert(
                        rule.target.clone(),
                        (rule.warrant.clone(), rule.premises.clone()),
                    );
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
                    let premise_state = if gone.contains(premise) {
                        "retired".to_string()
                    } else {
                        state
                            .get(premise)
                            .cloned()
                            .unwrap_or_else(|| "fact".to_string())
                    };
                    let label = if premise_state == "defeated" {
                        "defeated".to_string()
                    } else {
                        premise_state
                    };
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
        let missing: Vec<String> = rule
            .premises
            .iter()
            .filter(|premise| !held.contains(*premise))
            .cloned()
            .collect();
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
                let parts: Vec<String> = premises
                    .iter()
                    .map(|premise| chain(premise, facts, parent, depth + 1))
                    .collect();
                format!("{name} <- {warrant} <- {}", parts.join(" & "))
            }
            _ => name.to_string(),
        }
    }

    let mut chains = Vec::new();
    let mut held_names: Vec<String> = held
        .iter()
        .filter(|name| !facts.contains(*name))
        .cloned()
        .collect();
    held_names.sort();
    for name in &held_names {
        chains.push((name.clone(), chain(name, &facts, &parent, 0)));
    }

    let edges = support_edges(decls);
    let by_name: HashMap<&str, &Decl> = decls
        .iter()
        .map(|decl| (decl.name.as_str(), decl))
        .collect();
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
                        let grade = crate::program::grade(decl);
                        if grade != crate::program::Grade::Measured {
                            weak.push(format!("{premise} ({})", grade.label()));
                        }
                    }
                    Some(_) => queue.push(premise),
                    None => {}
                }
            }
        }
        gates.push((name.clone(), weak));
    }

    EngineResult {
        held: held_names,
        conditional,
        blocked,
        chains,
        gates,
    }
}
