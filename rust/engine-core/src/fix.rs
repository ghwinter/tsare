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
    out.push_str("}\n");
    out
}

pub fn render(decls: &[Decl]) -> String {
    decls.iter().map(render_decl).collect()
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
                end_line: decl.line,
                end_col: decl.col,
                replacement: String::new(),
            });
        }
    }
    out.extend(from_diagnostics(decls));
    out
}
