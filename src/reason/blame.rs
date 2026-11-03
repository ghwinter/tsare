//! Which version on the store's HEAD chain last changed each field of a declaration.
//!
//! The store has no clock, so the chain of parents is the order.  Blame walks it from HEAD to the
//! oldest version and charges a field to the first version whose value differs from its parent.

use std::path::Path;

use crate::syntax::Decl;

/// One field and the version that last changed it.
pub struct Charge {
    pub field: String,
    pub version: String,
    pub value: Option<String>,
}

/// The blame of one declaration in the head version.
pub struct Blame {
    pub name: String,
    pub version: String,
    pub charges: Vec<Charge>,
}

/// Charge each field of `name` in the head version to the version that last changed it.
pub fn blame(store: &Path, name: &str) -> Result<Blame, String> {
    let Some(head) = crate::version::head(store)? else {
        return Err("the store has no HEAD".to_string());
    };
    let mut hashes = vec![head.clone()];
    loop {
        let parent = crate::version::parent(store, hashes.last().expect("a head"))?;
        let Some(parent) = parent else {
            break;
        };
        if hashes.contains(&parent) {
            break;
        }
        hashes.push(parent);
    }
    let mut chain: Vec<(String, Vec<Decl>)> = Vec::new();
    for hash in &hashes {
        let text = crate::version::read(store, hash)?;
        let decls = crate::syntax::parse(&text).map_err(|error| format!("{hash}: {error}"))?;
        chain.push((hash.clone(), decls));
    }
    let head_decls = &chain[0].1;
    let Some(decl) = head_decls.iter().find(|item| item.name == name) else {
        return Err(format!("{name} is not in the head version"));
    };
    let mut keys = vec!["kind".to_string()];
    keys.extend(decl.fields.iter().map(|(key, _)| key.clone()));
    let mut charges = Vec::new();
    for key in keys {
        let mut found: Option<(String, Option<String>)> = None;
        for index in 0..chain.len() {
            let new = value_at(&chain[index].1, name, &key);
            let old = if index + 1 < chain.len() {
                value_at(&chain[index + 1].1, name, &key)
            } else {
                None
            };
            if new != old {
                found = Some((chain[index].0.clone(), new));
                break;
            }
        }
        if let Some((version, value)) = found {
            charges.push(Charge {
                field: key,
                version,
                value,
            });
        }
    }
    Ok(Blame {
        name: name.to_string(),
        version: head,
        charges,
    })
}

fn value_at(decls: &[Decl], name: &str, key: &str) -> Option<String> {
    let decl = decls.iter().find(|item| item.name == name)?;
    if key == "kind" {
        return Some(decl.kind.clone());
    }
    let value = decl
        .fields
        .iter()
        .find(|(field, _)| field == key)
        .map(|(_, value)| value)?;
    Some(crate::format::value_of(value))
}
