//! A content-addressed store of ledgers, with a HEAD chain: save by hash, list, restore, walk.
//!
//! The file name is the SHA-256 of the bytes, so two equal ledgers are one entry and a restore can
//! refuse content that no longer matches the name.  Each save advances `HEAD` and records the
//! previous head as the new version's parent; the store holds no clock, the chain is the order.

use std::fs;
use std::path::Path;

use crate::hash::hex;

/// Store the bytes of `path` under their hash and advance the store's HEAD; returns the hash.
pub fn save(path: &Path, store: &Path) -> Result<String, String> {
    let bytes = fs::read(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let hash = hex(&bytes);
    fs::create_dir_all(store).map_err(|error| format!("{}: {error}", store.display()))?;
    let target = store.join(format!("{hash}.tsare"));
    if !target.exists() {
        fs::write(&target, &bytes).map_err(|error| format!("{}: {error}", target.display()))?;
    }
    if let Some(previous) = head(store)? {
        if previous != hash {
            let parent = store.join(format!("{hash}.parent"));
            if !parent.exists() {
                fs::write(&parent, &previous)
                    .map_err(|error| format!("{}: {error}", parent.display()))?;
            }
        }
    }
    let head_path = store.join("HEAD");
    fs::write(&head_path, &hash).map_err(|error| format!("{}: {error}", head_path.display()))?;
    Ok(hash)
}

/// The hash the store's HEAD names, if it has one.
pub fn head(store: &Path) -> Result<Option<String>, String> {
    let path = store.join("HEAD");
    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("{}: {error}", path.display())),
    };
    let text = text.trim();
    if text.is_empty() {
        return Ok(None);
    }
    Ok(Some(text.to_string()))
}

/// The parent hash recorded for a version, if any.
pub fn parent(store: &Path, hash: &str) -> Result<Option<String>, String> {
    let path = store.join(format!("{hash}.parent"));
    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("{}: {error}", path.display())),
    };
    let text = text.trim();
    if text.is_empty() {
        Ok(None)
    } else {
        Ok(Some(text.to_string()))
    }
}

/// The content hashes in the store, sorted; an absent store is empty.
pub fn list(store: &Path) -> Result<Vec<String>, String> {
    let entries = match fs::read_dir(store) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(format!("{}: {error}", store.display())),
    };
    let mut out = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|error| format!("{}: {error}", store.display()))?;
        if !entry
            .file_type()
            .map(|kind| kind.is_file())
            .unwrap_or(false)
        {
            continue;
        }
        let name = entry.file_name();
        let name = name.to_string_lossy();
        let Some(stem) = name.strip_suffix(".tsare") else {
            continue;
        };
        if stem.len() == 64 && stem.chars().all(|c| c.is_ascii_hexdigit()) {
            out.push(stem.to_string());
        }
    }
    out.sort();
    Ok(out)
}

/// The stored bytes for `hash` as text, verified against the hash.
pub fn read(store: &Path, hash: &str) -> Result<String, String> {
    if hash.len() != 64 || !hash.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(format!("{hash}: not a content hash"));
    }
    let source = store.join(format!("{hash}.tsare"));
    let bytes = fs::read(&source).map_err(|error| format!("{}: {error}", source.display()))?;
    if hex(&bytes) != hash {
        return Err(format!("{hash}: the stored bytes do not match the hash"));
    }
    String::from_utf8(bytes).map_err(|_| format!("{hash}: the stored bytes are not UTF-8"))
}

/// Write the stored bytes for `hash` to `out`, refusing bytes that do not match the hash.
pub fn restore(store: &Path, hash: &str, out: &Path) -> Result<(), String> {
    let text = read(store, hash)?;
    fs::write(out, text.as_bytes()).map_err(|error| format!("{}: {error}", out.display()))?;
    Ok(())
}
