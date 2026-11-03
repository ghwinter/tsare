use super::*;

pub(crate) fn cmd_delta(
    before: &Path,
    after: &Path,
    json: bool,
    record: Option<&str>,
    doc: Option<&str>,
    action: Option<&str>,
) -> i32 {
    let read = |path: &Path| -> Result<String, String> {
        fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))
    };
    let (left, right) = match (read(before), read(after)) {
        (Ok(left), Ok(right)) => (left, right),
        (Err(error), _) | (_, Err(error)) => return fail(error),
    };
    // What was done is a small closed set, not prose: the store is queried by it, and a sentence
    // written by the agent that made the edit is the one thing the audit cannot check.
    let action = match action {
        None => "unspecified",
        Some("mechanical") => "mechanical",
        Some("model") => "model",
        Some("manual") => "manual",
        Some(other) => {
            println!("--action is mechanical, model or manual; got {other}");
            return 2;
        }
    };
    let difference = match crate::delta::compare(&left, &right) {
        Ok(difference) => difference,
        Err(error) => return fail(error),
    };
    let mut recorded: Option<String> = None;
    if let Some(store) = record {
        let Some(doc) = doc else {
            println!("usage: tsare delta BEFORE AFTER --record STORE --doc DOC [--action A]");
            return 2;
        };
        let entry = match crate::cases::entry(&left, &right, doc, action, crate::cache::SALT) {
            Ok(entry) => entry,
            Err(error) => return fail(error),
        };
        if let Err(error) = crate::cases::append(Path::new(store), &entry) {
            return fail(format!("{store}: {error}"));
        }
        recorded = Some(store.to_string());
    }
    if json {
        println!(
            "{}",
            crate::delta::to_json(&difference, recorded.as_deref())
        );
        return 0;
    }
    print!("{}", crate::delta::text(&difference));
    if let Some(store) = recorded {
        println!("recorded: {store} (action {action})");
    }
    0
}

/// The semantic diff between two ledgers: declarations added, removed, and fields changed.
pub(crate) fn cmd_diff(before: &Path, after: &Path, json: bool) -> i32 {
    let read = |path: &Path| -> Result<String, String> {
        fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))
    };
    let (left, right) = match (read(before), read(after)) {
        (Ok(left), Ok(right)) => (left, right),
        (Err(error), _) | (_, Err(error)) => {
            println!("{error}");
            return 2;
        }
    };
    let old = match crate::syntax::parse(&left) {
        Ok(decls) => decls,
        Err(error) => {
            println!("{}: {error}", before.display());
            return 2;
        }
    };
    let new = match crate::syntax::parse(&right) {
        Ok(decls) => decls,
        Err(error) => {
            println!("{}: {error}", after.display());
            return 2;
        }
    };
    let difference = crate::diff::diff(&old, &new);
    let old_hash: String = crate::hash::hex(left.as_bytes()).chars().take(12).collect();
    let new_hash: String = crate::hash::hex(right.as_bytes())
        .chars()
        .take(12)
        .collect();
    let code = if difference.is_empty() { 0 } else { 1 };
    if json {
        let added: Vec<J> = difference
            .added
            .iter()
            .map(|(name, kind)| {
                obj(vec![
                    ("name", J::Str(name.clone())),
                    ("kind", J::Str(kind.clone())),
                ])
            })
            .collect();
        let removed: Vec<J> = difference
            .removed
            .iter()
            .map(|(name, kind)| {
                obj(vec![
                    ("name", J::Str(name.clone())),
                    ("kind", J::Str(kind.clone())),
                ])
            })
            .collect();
        let changed: Vec<J> = difference
            .changed
            .iter()
            .map(|change| {
                let fields: Vec<J> = change
                    .fields
                    .iter()
                    .map(|field| {
                        obj(vec![
                            ("key", J::Str(field.key.clone())),
                            (
                                "before",
                                match &field.before {
                                    Some(text) => J::Str(text.clone()),
                                    None => J::Null,
                                },
                            ),
                            (
                                "after",
                                match &field.after {
                                    Some(text) => J::Str(text.clone()),
                                    None => J::Null,
                                },
                            ),
                        ])
                    })
                    .collect();
                obj(vec![
                    ("name", J::Str(change.name.clone())),
                    ("before_kind", J::Str(change.before_kind.clone())),
                    ("after_kind", J::Str(change.after_kind.clone())),
                    ("fields", J::Arr(fields)),
                ])
            })
            .collect();
        emit(obj(vec![
            ("old", J::Str(old_hash)),
            ("new", J::Str(new_hash)),
            ("added", J::Arr(added)),
            ("removed", J::Arr(removed)),
            ("changed", J::Arr(changed)),
        ]));
        return code;
    }
    println!("diff {old_hash} -> {new_hash}");
    if difference.is_empty() {
        println!("no semantic change");
        return 0;
    }
    for (name, kind) in &difference.added {
        println!("+ {name} {kind}");
    }
    for (name, kind) in &difference.removed {
        println!("- {name} {kind}");
    }
    for change in &difference.changed {
        if change.before_kind != change.after_kind {
            println!(
                "~ {}: kind {} -> {}",
                change.name, change.before_kind, change.after_kind
            );
        } else {
            println!("~ {} {}", change.name, change.after_kind);
        }
        for field in &change.fields {
            println!(
                "    {}: {} -> {}",
                field.key,
                field.before.as_deref().unwrap_or("-"),
                field.after.as_deref().unwrap_or("-")
            );
        }
    }
    code
}

/// A three-way structural merge; writes only when nothing conflicts.
pub(crate) fn cmd_merge(
    base: &Path,
    ours: &Path,
    theirs: &Path,
    out: Option<&str>,
    json: bool,
) -> i32 {
    let parse = |path: &Path| -> Result<Vec<crate::syntax::Decl>, String> {
        let text =
            fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))?;
        crate::syntax::parse(&text).map_err(|error| format!("{}: {error}", path.display()))
    };
    let (before, mine, other) = match (parse(base), parse(ours), parse(theirs)) {
        (Ok(before), Ok(mine), Ok(other)) => (before, mine, other),
        (Err(error), _, _) | (_, Err(error), _) | (_, _, Err(error)) => {
            println!("{error}");
            return 2;
        }
    };
    let merged = crate::merge::merge(&before, &mine, &other);
    let Some(out) = out else {
        println!("usage: tsare merge BASE OURS THEIRS --out FILE [--json]");
        return 2;
    };
    let text = merge_text(&merged);
    if json {
        let conflicts: Vec<J> = merged
            .conflicts
            .iter()
            .map(|item| {
                obj(vec![
                    ("name", J::Str(item.name.clone())),
                    ("field", J::Str(item.field.clone())),
                    ("base", optional(item.base.clone())),
                    ("ours", optional(item.ours.clone())),
                    ("theirs", optional(item.theirs.clone())),
                ])
            })
            .collect();
        let names = |list: &[String]| -> J {
            J::Arr(list.iter().map(|name| J::Str(name.clone())).collect())
        };
        let empty = merged.conflicts.is_empty();
        emit(obj(vec![
            ("merged", J::Bool(empty)),
            (
                "out",
                if empty {
                    J::Str(out.to_string())
                } else {
                    J::Null
                },
            ),
            ("conflicts", J::Arr(conflicts)),
            ("added", names(&merged.added)),
            ("removed", names(&merged.removed)),
            ("changed", names(&merged.changed)),
        ]));
    }
    if !merged.conflicts.is_empty() {
        if !json {
            for item in &merged.conflicts {
                println!(
                    "conflict {} {}: base {}, ours {}, theirs {}",
                    item.name,
                    item.field,
                    item.base.as_deref().unwrap_or("-"),
                    item.ours.as_deref().unwrap_or("-"),
                    item.theirs.as_deref().unwrap_or("-")
                );
            }
            println!(
                "merge stopped: {} conflict(s); nothing written",
                merged.conflicts.len()
            );
        }
        return 1;
    }
    if let Err(error) = fs::write(out, text) {
        return fail(format!("{out}: {error}"));
    }
    if !json {
        println!("merged {} declaration(s) -> {out}", merged.views.len());
    }
    0
}

/// Which version on the store's HEAD chain last changed each field of a declaration.
pub(crate) fn cmd_blame(store: &Path, name: &str, json: bool) -> i32 {
    let blame = match crate::blame::blame(store, name) {
        Ok(blame) => blame,
        Err(error) => return fail(error),
    };
    let short = |hash: &str| -> String { hash.chars().take(12).collect() };
    if json {
        let charges: Vec<J> = blame
            .charges
            .iter()
            .map(|charge| {
                obj(vec![
                    ("field", J::Str(charge.field.clone())),
                    ("version", J::Str(charge.version.clone())),
                    ("value", optional(charge.value.clone())),
                ])
            })
            .collect();
        emit(obj(vec![
            ("name", J::Str(blame.name.clone())),
            ("version", J::Str(blame.version.clone())),
            ("charges", J::Arr(charges)),
        ]));
        return 0;
    }
    println!("blame {} at {}", blame.name, short(&blame.version));
    for charge in &blame.charges {
        println!(
            "  {:<12} {:<14} {}",
            charge.field,
            short(&charge.version),
            charge.value.as_deref().unwrap_or("-")
        );
    }
    0
}

/// The support path behind a declaration, from the claim to the evidence.
pub(crate) fn cmd_chain(path: &Path, name: &str, json: bool) -> i32 {
    let loaded = match load(path) {
        Ok(loaded) => loaded,
        Err(error) => return fail(error),
    };
    let link = match crate::chain::chain(&loaded.decls, name) {
        Ok(link) => link,
        Err(error) => return fail(error),
    };
    if json {
        emit(link_json(&link));
    } else {
        print_link(&link, 0);
    }
    0
}

/// Every conclusion whose support path rests on an unbacked hypothesis.
pub(crate) fn cmd_chains(path: &Path, json: bool) -> i32 {
    let loaded = match load(path) {
        Ok(loaded) => loaded,
        Err(error) => return fail(error),
    };
    let mut rows = Vec::new();
    for decl in &loaded.decls {
        if !matches!(decl.kind.as_str(), "proposition" | "warrant") {
            continue;
        }
        if let Ok(link) = crate::chain::chain(&loaded.decls, &decl.name) {
            let undecided = crate::chain::undecided(&link);
            if !undecided.is_empty() {
                rows.push((decl.name.clone(), decl.kind.clone(), undecided));
            }
        }
    }
    if json {
        let items: Vec<J> = rows
            .iter()
            .map(|(name, kind, hypotheses)| {
                obj(vec![
                    ("name", J::Str(name.clone())),
                    ("kind", J::Str(kind.clone())),
                    (
                        "hypotheses",
                        J::Arr(hypotheses.iter().map(|h| J::Str(h.clone())).collect()),
                    ),
                ])
            })
            .collect();
        emit(obj(vec![
            ("count", J::Num(rows.len() as f64)),
            ("open", J::Arr(items)),
        ]));
    } else if rows.is_empty() {
        println!("no open chains");
    } else {
        for (name, kind, hypotheses) in &rows {
            println!(
                "{name} {kind} stands on the unbacked hypothesis {}",
                hypotheses.join(", ")
            );
        }
    }
    if rows.is_empty() {
        0
    } else {
        1
    }
}
