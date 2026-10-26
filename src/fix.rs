//! Repair candidates, rendered as suggestions with an applicability.

use std::collections::HashSet;

use crate::check::{check, Diagnostic};
use crate::syntax::{Decl, Val};

pub fn render_value(value: &Val) -> String {
    match value {
        Val::Id(name) => name.clone(),
        Val::Str(text) => format!("\"{}\"", text.replace('\\', "\\\\").replace('"', "\\\"")),
        Val::Num(text) => text.clone(),
        Val::Hole => "?".to_string(),
        Val::List(items) => {
            let parts: Vec<String> = items.iter().map(render_value).collect();
            format!("[{}]", parts.join(", "))
        }
        Val::Call(name, args) => {
            let parts: Vec<String> = args.iter().map(render_value).collect();
            format!("{}({})", name, parts.join(", "))
        }
        Val::Rec(fields) => {
            let parts: Vec<String> =
                fields.iter().map(|(key, item)| format!("{key}: {}", render_value(item))).collect();
            format!("{{ {} }}", parts.join(", "))
        }
    }
}

pub fn render_decl(decl: &Decl) -> String {
    let mut out = format!("{} {} {{\n", decl.kind, decl.name);
    for (key, value) in &decl.fields {
        out.push_str(&format!("  {key}: {},\n", render_value(value)));
    }
    out.push('}');
    out
}

pub fn render(decls: &[Decl]) -> String {
    let chunks: Vec<String> = decls.iter().map(render_decl).collect();
    format!("{}\n", chunks.join("\n\n"))
}

#[derive(Debug, Clone, PartialEq)]
pub struct Suggestion {
    pub applicability: &'static str,
    pub message: String,
    pub file: String,
    pub line: usize,
    pub col: usize,
    pub end_line: usize,
    pub end_col: usize,
    pub replacement: String,
}

fn score(decls: &[Decl]) -> (usize, usize) {
    let diagnostics: Vec<Diagnostic> = check(decls);
    let errors = diagnostics.iter().filter(|item| item.code.starts_with("E-")).count();
    let warnings = diagnostics.iter().filter(|item| item.code.starts_with("W-")).count();
    (errors, warnings)
}

fn without(decls: &[Decl], target: &Decl) -> Vec<Decl> {
    decls.iter().filter(|decl| !std::ptr::eq(*decl, target)).cloned().collect()
}

fn classify(before: (usize, usize), after: (usize, usize), text: &str, original: &str) -> &'static str {
    if after >= before {
        return "Unspecified";
    }
    if text.matches('?').count() > original.matches('?').count() {
        return "HasPlaceholders";
    }
    "MachineApplicable"
}

fn clone_with_list(decls: &[Decl], owner: &str, key: &str, old: &str, new: &str) -> Option<Vec<Decl>> {
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

pub fn from_diagnostics(decls: &[Decl]) -> Vec<Suggestion> {
    let before = score(decls);
    let diagnostics = check(decls);
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

fn code_counts(decls: &[Decl]) -> std::collections::HashMap<String, usize> {
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
            copy.fields.push((key.to_string(), value.clone()));
        }
        out.push(copy);
    }
    out
}

/// Warnings a candidate should reduce.  A hole is a progress marker, not noise.
pub fn score_without_holes(decls: &[Decl]) -> (usize, usize) {
    let diagnostics = check(decls);
    let errors = diagnostics.iter().filter(|item| item.code.starts_with("E-")).count();
    let warnings = diagnostics
        .iter()
        .filter(|item| item.code.starts_with("W-") && item.code != "W-HOLE")
        .count();
    (errors, warnings)
}

/// Every repair candidate: a description and the declaration list it would produce.
pub fn candidates(decls: &[Decl]) -> Vec<(String, Vec<Decl>)> {
    let mut out: Vec<(String, Vec<Decl>)> = Vec::new();
    let warranted: HashSet<String> = decls
        .iter()
        .filter(|decl| decl.kind == "warrant")
        .flat_map(|decl| crate::closure::field_ids(decl, "to"))
        .collect();

    // a duplicate name is the one repair a machine can make without guessing meaning:
    // keep the first declaration, drop the later one; references still resolve
    let mut seen_names: HashSet<&str> = HashSet::new();
    for (index, decl) in decls.iter().enumerate() {
        if !seen_names.insert(decl.name.as_str()) {
            let trial: Vec<Decl> =
                decls.iter().enumerate().filter(|(position, _)| *position != index).map(|(_, item)| item.clone()).collect();
            out.push((format!("{}: drop the duplicate at line {}", decl.name, decl.line), trial));
        }
    }

    for decl in decls {
        let missing: Vec<String> = crate::specs::spec(&decl.kind)
            .map(|fields| {
                fields
                    .iter()
                    .filter(|(name, _, required)| {
                        *required
                            && !["claim", "evidence", "name", "to"].contains(name)
                            && !decl.fields.iter().any(|(key, _)| key == *name)
                    })
                    .map(|(name, _, _)| name.to_string())
                    .collect()
            })
            .unwrap_or_default();
        if !missing.is_empty() {
            let mut trial = decls.to_vec();
            for key in &missing {
                trial = add_field(&trial, &decl.name, key, Val::Hole);
            }
            let listed =
                missing.iter().map(|key| format!("`{key}: ?`")).collect::<Vec<_>>().join(", ");
            out.push((format!("{}: add {listed}", decl.name), trial));
        }
        if decl.kind == "value" {
            let absent: Vec<&str> = ["unit", "threshold"]
                .into_iter()
                .filter(|key| !decl.fields.iter().any(|(name, _)| name == key))
                .collect();
            if !absent.is_empty() {
                let mut trial = decls.to_vec();
                for key in &absent {
                    trial = add_field(&trial, &decl.name, key, Val::Hole);
                }
                let listed =
                    absent.iter().map(|key| format!("`{key}: ?`")).collect::<Vec<_>>().join(", ");
                out.push((format!("{}: add {listed}", decl.name), trial));
            }
        }
        if decl.kind == "proposition"
            && !crate::closure::field_ids(decl, "from").is_empty()
            && !warranted.contains(&decl.name)
        {
            out.push((
                format!("{}: add a warrant over its premises", decl.name),
                add_warrant(decls, decl),
            ));
        }
        if decl.kind == "fact" {
            let claim = crate::program::f_str(decl, "claim").unwrap_or("");
            let universal = ["所有", "都", "全部", "always", "every", "all "];
            if universal.iter().any(|word| claim.contains(word))
                && !decl.fields.iter().any(|(key, _)| key == "scope")
            {
                out.push((
                    format!("{}: add `scope: ?`", decl.name),
                    add_field(decls, &decl.name, "scope", Val::Hole),
                ));
            }
        }
    }
    out
}

pub struct Verified {
    pub description: String,
    pub trial: Vec<Decl>,
    pub delta: (isize, isize),
}

/// Candidates that a full re-check accepts: fewer errors, or fewer warnings without new ones.
pub fn verified(decls: &[Decl]) -> (Vec<Verified>, (usize, usize)) {
    let (before_e, before_w) = score_without_holes(decls);
    let mut accepted = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    for (description, trial) in candidates(decls) {
        if !seen.insert(description.clone()) {
            continue;
        }
        let (after_e, after_w) = score_without_holes(&trial);
        let better = after_e < before_e || (after_e == before_e && after_w < before_w);
        if better {
            accepted.push(Verified {
                description,
                trial,
                delta: (after_e as isize - before_e as isize, after_w as isize - before_w as isize),
            });
        }
    }
    (accepted, (before_e, before_w))
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

pub fn suggestions(decls: &[Decl]) -> Vec<Suggestion> {
    let mut out = Vec::new();
    let mut seen: HashSet<&str> = HashSet::new();
    for decl in decls {
        if seen.insert(decl.name.as_str()) {
            continue;
        }
        let trial = without(decls, decl);
        let before = score(decls);
        let after = score(&trial);
        if after < before {
            out.push(Suggestion {
                applicability: classify(before, after, "", ""),
                message: format!("{}: drop the duplicate at line {}", decl.name, decl.line),
                file: decl.file.clone(),
                line: decl.line,
                col: decl.col,
                end_line: decl.end_line,
                end_col: decl.end_col,
                replacement: String::new(),
            });
        }
    }
    let before = score(decls);
    let old_codes = code_counts(decls);
    let warranted: std::collections::HashSet<String> = decls
        .iter()
        .filter(|decl| decl.kind == "warrant")
        .flat_map(|decl| crate::closure::field_ids(decl, "to"))
        .collect();

    for decl in decls {
        if decl.kind == "proposition".to_string()
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
                    replacement: String::new(),
                });
            }
        }
        if let Some(fields) = crate::specs::spec(&decl.kind) {
            let missing: Vec<&str> = fields
                .iter()
                .filter(|(name, _, required)| {
                    *required
                        && !["claim", "evidence", "name", "to"].contains(name)
                        && !decl.fields.iter().any(|(key, _)| key == name)
                })
                .map(|(name, _, _)| *name)
                .chain(
                    if decl.kind == "value" {
                        ["unit", "threshold"]
                            .into_iter()
                            .filter(|key| !decl.fields.iter().any(|(name, _)| name == *key))
                            .collect::<Vec<_>>()
                    } else {
                        Vec::new()
                    },
                )
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

    out.extend(from_diagnostics(decls));
    out
}
