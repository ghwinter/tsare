//! A content-addressed store of ledgers: save by hash, list, restore after checking the hash.
//!
//! The file name is the SHA-256 of the bytes, so two equal ledgers are one entry and a restore can
//! refuse content that no longer matches the name.  The store is a directory; it holds no clock and
//! no order.

use std::fs;
use std::path::Path;

use crate::hash::hex;

/// Store the bytes of `path` under their hash; returns the full hash.
pub fn save(path: &Path, store: &Path) -> Result<String, String> {
    let bytes = fs::read(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let hash = hex(&bytes);
    fs::create_dir_all(store).map_err(|error| format!("{}: {error}", store.display()))?;
    let target = store.join(format!("{hash}.tsare"));
    if !target.exists() {
        fs::write(&target, &bytes).map_err(|error| format!("{}: {error}", target.display()))?;
    }
    Ok(hash)
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

/// Write the stored bytes for `hash` to `out`, refusing bytes that do not match the hash.
pub fn restore(store: &Path, hash: &str, out: &Path) -> Result<(), String> {
    if hash.len() != 64 || !hash.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(format!("{hash}: not a content hash"));
    }
    let source = store.join(format!("{hash}.tsare"));
    let bytes = fs::read(&source).map_err(|error| format!("{}: {error}", source.display()))?;
    if hex(&bytes) != hash {
        return Err(format!("{hash}: the stored bytes do not match the hash"));
    }
    fs::write(out, &bytes).map_err(|error| format!("{}: {error}", out.display()))?;
    Ok(())
}
