//! A three-way structural merge: declarations by name, fields by value, conflicts named.
//!
//! The merge is not textual: it compares a common ancestor with two edits.  A declaration changed
//! on one side takes that side; changed on both with different values is a conflict.  Nothing is
//! written when a conflict remains: the caller resolves it, the tool does not guess.

use std::collections::HashMap;

use crate::syntax::{Decl, Val};

/// One declaration in the merge view: what a merge needs, without source positions.
#[derive(Clone)]
pub struct View {
    pub kind: String,
    pub name: String,
    pub fields: Vec<(String, Val)>,
}

impl View {
    pub fn of(decl: &Decl) -> View {
        View {
            kind: decl.kind.clone(),
            name: decl.name.clone(),
            fields: decl.fields.clone(),
        }
    }
}

/// One unresolved difference: both sides changed the same field, or one removed what the other changed.
pub struct Conflict {
    pub name: String,
    pub field: String,
    pub base: Option<String>,
    pub ours: Option<String>,
    pub theirs: Option<String>,
}

/// The merged declarations and the conflicts that stop the write.
pub struct Merged {
    pub views: Vec<View>,
    pub conflicts: Vec<Conflict>,
    pub added: Vec<String>,
    pub removed: Vec<String>,
    pub changed: Vec<String>,
}

/// Merge `ours` and `theirs` against `base`.
pub fn merge(base: &[Decl], ours: &[Decl], theirs: &[Decl]) -> Merged {
    let base_map: HashMap<&str, View> = base
        .iter()
        .map(|decl| (decl.name.as_str(), View::of(decl)))
        .collect();
    let ours_map: HashMap<&str, View> = ours
        .iter()
        .map(|decl| (decl.name.as_str(), View::of(decl)))
        .collect();
    let theirs_map: HashMap<&str, View> = theirs
        .iter()
        .map(|decl| (decl.name.as_str(), View::of(decl)))
        .collect();
    let mut names: Vec<String> = ours.iter().map(|decl| decl.name.clone()).collect();
    for decl in theirs {
        if !names.contains(&decl.name) {
            names.push(decl.name.clone());
        }
    }
    let mut views = Vec::new();
    let mut conflicts = Vec::new();
    let mut added = Vec::new();
    let mut removed = Vec::new();
    let mut changed = Vec::new();
    for name in names {
        let before = base_map.get(name.as_str());
        let mine = ours_map.get(name.as_str());
        let other = theirs_map.get(name.as_str());
        match (before, mine, other) {
            (_, Some(mine), Some(other)) if same(mine, other) => views.push(mine.clone()),
            (Some(before), Some(mine), Some(other)) => {
                if same(mine, before) {
                    views.push(other.clone());
                    changed.push(name);
                } else if same(other, before) {
                    views.push(mine.clone());
                    changed.push(name);
                } else {
                    let merged = merge_three(before, mine, other, &mut conflicts);
                    if !conflicts.iter().any(|item| item.name == name) {
                        changed.push(name.clone());
                    }
                    views.push(merged);
                }
            }
            (Some(before), Some(mine), None) => {
                if same(mine, before) {
                    removed.push(name);
                } else {
                    conflicts.push(Conflict {
                        name,
                        field: "declaration".to_string(),
                        base: Some("present".to_string()),
                        ours: Some("changed".to_string()),
                        theirs: Some("removed".to_string()),
                    });
                }
            }
            (Some(before), None, Some(other)) => {
                if same(other, before) {
                    removed.push(name);
                } else {
                    conflicts.push(Conflict {
                        name,
                        field: "declaration".to_string(),
                        base: Some("present".to_string()),
                        ours: Some("removed".to_string()),
                        theirs: Some("changed".to_string()),
                    });
                }
            }
            (Some(_), None, None) => removed.push(name),
            (None, Some(mine), None) => {
                added.push(name);
                views.push(mine.clone());
            }
            (None, None, Some(other)) => {
                added.push(name);
                views.push(other.clone());
            }
            (None, Some(mine), Some(other)) => {
                if same(mine, other) {
                    views.push(mine.clone());
                } else {
                    conflicts.push(Conflict {
                        name,
                        field: "declaration".to_string(),
                        base: None,
                        ours: Some("added".to_string()),
                        theirs: Some("added".to_string()),
                    });
                    views.push(mine.clone());
                }
            }
            (None, None, None) => {}
        }
    }
    Merged {
        views,
        conflicts,
        added,
        removed,
        changed,
    }
}

fn same(left: &View, right: &View) -> bool {
    left.kind == right.kind && left.fields == right.fields
}

fn value<'a>(view: &'a View, key: &str) -> Option<&'a Val> {
    view.fields
        .iter()
        .find(|(name, _)| name == key)
        .map(|(_, value)| value)
}

fn merge_three(before: &View, mine: &View, other: &View, conflicts: &mut Vec<Conflict>) -> View {
    let mut kind = mine.kind.clone();
    if mine.kind != other.kind {
        if mine.kind == before.kind {
            kind = other.kind.clone();
        } else if other.kind == before.kind {
            kind = mine.kind.clone();
        } else {
            conflicts.push(Conflict {
                name: mine.name.clone(),
                field: "kind".to_string(),
                base: Some(before.kind.clone()),
                ours: Some(mine.kind.clone()),
                theirs: Some(other.kind.clone()),
            });
        }
    }
    let mut keys: Vec<String> = mine.fields.iter().map(|(key, _)| key.clone()).collect();
    for key in other
        .fields
        .iter()
        .chain(before.fields.iter())
        .map(|(key, _)| key.clone())
    {
        if !keys.contains(&key) {
            keys.push(key);
        }
    }
    let mut fields = Vec::new();
    for key in keys {
        let base = value(before, &key);
        let ours = value(mine, &key);
        let theirs = value(other, &key);
        let chosen = if ours == theirs {
            ours
        } else if ours == base {
            theirs
        } else if theirs == base {
            ours
        } else {
            conflicts.push(Conflict {
                name: mine.name.clone(),
                field: key.clone(),
                base: base.map(crate::format::value_of),
                ours: ours.map(crate::format::value_of),
                theirs: theirs.map(crate::format::value_of),
            });
            None
        };
        if let Some(value) = chosen {
            fields.push((key, value.clone()));
        }
    }
    View {
        kind,
        name: mine.name.clone(),
        fields,
    }
}
