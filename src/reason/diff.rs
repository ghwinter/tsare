//! What changed between two ledgers: declarations added, removed, and fields changed.
//!
//! The diff is semantic: declarations are compared by name and fields by value, not by line.  The
//! document hashes are the version identity; the changes are the content.

use std::collections::HashMap;

use crate::syntax::{Decl, Val};

/// One field that changed; `None` means the field was absent on that side.
pub struct FieldChange {
    pub key: String,
    pub before: Option<String>,
    pub after: Option<String>,
}

/// One declaration present on both sides with a changed kind or field.
pub struct Change {
    pub name: String,
    pub before_kind: String,
    pub after_kind: String,
    pub fields: Vec<FieldChange>,
}

/// The three kinds of change, in document order: added, removed, changed.
pub struct Diff {
    pub added: Vec<(String, String)>,
    pub removed: Vec<(String, String)>,
    pub changed: Vec<Change>,
}

impl Diff {
    pub fn is_empty(&self) -> bool {
        self.added.is_empty() && self.removed.is_empty() && self.changed.is_empty()
    }
}

/// Compare two parsed documents by declaration name, kind and fields.
pub fn diff(before: &[Decl], after: &[Decl]) -> Diff {
    let old: HashMap<&str, &Decl> = before
        .iter()
        .map(|decl| (decl.name.as_str(), decl))
        .collect();
    let new: HashMap<&str, &Decl> = after
        .iter()
        .map(|decl| (decl.name.as_str(), decl))
        .collect();
    let added = after
        .iter()
        .filter(|decl| !old.contains_key(decl.name.as_str()))
        .map(|decl| (decl.name.clone(), decl.kind.clone()))
        .collect();
    let removed = before
        .iter()
        .filter(|decl| !new.contains_key(decl.name.as_str()))
        .map(|decl| (decl.name.clone(), decl.kind.clone()))
        .collect();
    let mut changed = Vec::new();
    for decl in after {
        let Some(before_decl) = old.get(decl.name.as_str()) else {
            continue;
        };
        let mut keys: Vec<&str> = before_decl
            .fields
            .iter()
            .map(|(key, _)| key.as_str())
            .collect();
        for (key, _) in &decl.fields {
            if !keys.contains(&key.as_str()) {
                keys.push(key.as_str());
            }
        }
        let mut fields = Vec::new();
        for key in keys {
            let before_val = before_decl
                .fields
                .iter()
                .find(|(name, _)| name == key)
                .map(|(_, value)| value);
            let after_val = decl
                .fields
                .iter()
                .find(|(name, _)| name == key)
                .map(|(_, value)| value);
            if before_val == after_val {
                continue;
            }
            fields.push(FieldChange {
                key: key.to_string(),
                before: before_val.map(render),
                after: after_val.map(render),
            });
        }
        if before_decl.kind != decl.kind || !fields.is_empty() {
            changed.push(Change {
                name: decl.name.clone(),
                before_kind: before_decl.kind.clone(),
                after_kind: decl.kind.clone(),
                fields,
            });
        }
    }
    Diff {
        added,
        removed,
        changed,
    }
}

/// A value as the document wrote it, for a diff line.
pub fn render(value: &Val) -> String {
    match value {
        Val::Id(name) => name.clone(),
        Val::Str(text) => format!("\"{text}\""),
        Val::Num(num) => num.clone(),
        Val::List(items) => format!(
            "[{}]",
            items.iter().map(render).collect::<Vec<_>>().join(", ")
        ),
        Val::Call(name, args) => format!(
            "{name}({})",
            args.iter().map(render).collect::<Vec<_>>().join(", ")
        ),
        Val::Rec(fields) => format!(
            "{{ {} }}",
            fields
                .iter()
                .map(|(key, value)| format!("{key}: {}", render(value)))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Val::Hole => "?".to_string(),
    }
}
