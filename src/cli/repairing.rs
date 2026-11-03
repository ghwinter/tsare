//! Commands that propose or apply a repair.

use super::*;

pub(crate) fn cmd_suggest(path: &Path, json: bool) -> Outcome {
    let loaded = match load(path) {
        Ok(loaded) => loaded,
        Err(error) => return fail(error),
    };
    let items = crate::fix::suggestions(&loaded.decls);
    if !json {
        if items.is_empty() {
            println!("no verified suggestion");
            return Outcome::Ok;
        }
        for item in &items {
            println!("{} {}", pad(item.applicability, 18), item.message);
        }
        return Outcome::Ok;
    }
    let rendered: Vec<String> = items
        .iter()
        .map(|item| {
            // A single-file load leaves decl.file empty; the command knows the document it read,
            // and a span without a file is not a location a caller can apply.
            let file = if item.file.is_empty() {
                path.display().to_string()
            } else {
                item.file.clone()
            };
            format!(
                "{{\"applicability\": \"{}\", \"message\": \"{}\", \"file\": \"{}\", \"span\": {{\"line\": {}, \"col\": {}, \"end_line\": {}, \"end_col\": {}}}, \"replacement\": \"{}\"}}",
                item.applicability,
                escape(&item.message),
                escape(&file),
                item.line,
                item.col,
                item.end_line,
                item.end_col,
                escape(&item.replacement)
            )
        })
        .collect();
    println!("{{\"suggestions\": [{}]}}", rendered.join(", "));
    Outcome::Ok
}

/// A conclusion written back must reach evidence: the G3 gate for in-place writes.
fn open_chain(text: &str) -> Option<String> {
    let decls = syntax::parse(text).ok()?;
    let open = crate::chain::open(&decls);
    open.first().map(|(name, _kind, hypotheses)| {
        format!(
            "{name} stands on the unbacked hypothesis {}",
            hypotheses.join(", ")
        )
    })
}

/// The check report for one side of an edit, as the case store reads it.
fn report(decls: &[crate::syntax::Decl], text: &str, display: &str) -> String {
    let mut session = crate::session::Session::new(display.to_string()).with_source(text);
    session.extend(crate::check::check(decls));
    session.json()
}

/// The process as step descriptors: what kind of edit happened, on which field, with the values
/// abstracted to their presence.  The document and the values are not part of the process.
fn describe(difference: &crate::diff::Diff) -> Vec<String> {
    let mut ops: Vec<String> = Vec::new();
    for (_, kind) in &difference.added {
        ops.push(format!("add {kind}"));
    }
    for (_, kind) in &difference.removed {
        ops.push(format!("remove {kind}"));
    }
    for change in &difference.changed {
        for field in &change.fields {
            let from = if field.before.is_some() {
                "set"
            } else {
                "unset"
            };
            let to = if field.after.is_some() {
                "set"
            } else {
                "unset"
            };
            ops.push(format!(
                "edit {} {} {}->{}",
                change.after_kind, field.key, from, to
            ));
        }
    }
    ops.sort();
    ops
}

/// The witness templates: the applied suggestion messages with their declaration names stripped.
fn templates(applied: &[String]) -> Vec<String> {
    let mut out: Vec<String> = applied
        .iter()
        .map(|message| {
            let rest = match message.split_once(": ") {
                Some((_, rest)) => rest,
                None => message.as_str(),
            };
            match rest.split_once(" at line ") {
                Some((head, _)) => head.to_string(),
                None => rest.to_string(),
            }
        })
        .collect();
    out.sort();
    out
}

pub(crate) fn cmd_apply(path: &Path, dry: bool, record: Option<&str>, action: &str) -> Outcome {
    let loaded = match load(path) {
        Ok(loaded) => loaded,
        Err(error) => return fail(error),
    };
    let items = crate::fix::suggestions(&loaded.decls);
    let (updated, applied) = crate::apply::apply_best(&loaded.text, &items);
    let refused = if !dry && updated != loaded.text {
        open_chain(&updated)
    } else {
        None
    };
    if let Some(store) = record {
        let after = crate::syntax::parse(&updated).unwrap_or_default();
        let before_report = report(&loaded.decls, &loaded.text, &path.display().to_string());
        let after_report = report(&after, &updated, &path.display().to_string());
        let process = describe(&crate::diff::diff(&loaded.decls, &after));
        let witness = templates(&applied);
        let signature = crate::cases::signature(&witness, &process);
        let lines = |items: &[String]| {
            crate::json::serialize(&crate::json::Json::Arr(
                items
                    .iter()
                    .map(|line| crate::json::Json::Str(line.clone()))
                    .collect(),
            ))
        };
        let process_json = lines(&process);
        let witness_json = lines(&witness);
        let entry = match crate::cases::entry(&crate::cases::Record {
            before: &before_report,
            after: &after_report,
            doc: &path.display().to_string(),
            action,
            checker: crate::cache::SALT,
            process: Some(&process_json),
            witness: Some(&witness_json),
            signature: Some(&signature),
            refused: refused.as_deref(),
        }) {
            Ok(entry) => entry,
            Err(error) => return fail(error),
        };
        if let Err(error) = crate::cases::append(Path::new(store), &entry) {
            return fail(format!("{store}: {error}"));
        }
    }
    if !dry && updated != loaded.text {
        if let Some(reason) = refused {
            return Outcome::Refused(format!("refused: {reason}; resolve it, or record why"));
        }
        if let Err(error) = crate::apply::replace(path, &updated) {
            return fail(format!("{}: {error}", path.display()));
        }
    }
    let list: Vec<String> = applied
        .iter()
        .map(|item| format!("\"{}\"", escape(item)))
        .collect();
    println!("{{\"applied\": [{}]}}", list.join(", "));
    Outcome::Ok
}

/// Codes whose repair needs a value only the author can choose.  Naming them is the honest
/// alternative to printing nothing and letting the reader assume no repair was needed.
const NO_REPAIR: &[(&str, &str)] = &[
    (
        "W-EXAMPLE-SHAPE",
        "needs an attribute the examples share; only the author knows one",
    ),
    (
        "W-OVERCLAIM",
        "needs `over:` naming a set the engine can enumerate; that is content",
    ),
    (
        "E-CYCLE",
        "a cycle can be cut at any edge; which edge is a judgement",
    ),
    (
        "W-UNCONFIRMED",
        "needs a person or a tool to confirm the evidence",
    ),
    (
        "E-UNFALSIFIABLE",
        "needs a test or a rival, which is content",
    ),
    ("E-VACUOUS", "needs premises or concepts, which is content"),
    (
        "W-PARTITION-UNKNOWN",
        "needs a dimension, which is the thing being asked for",
    ),
    (
        "W-PARTITION-UNLEARNED",
        "needs an attribute that separates the examples",
    ),
];

pub(crate) fn cmd_fix(path: &Path, apply: Option<&str>) -> Outcome {
    let loaded = match load(path) {
        Ok(loaded) => loaded,
        Err(error) => return fail(error),
    };
    let (accepted, (before_e, before_w)) = crate::fix::verified(&loaded.decls);
    println!("before: {before_e} errors, {before_w} warnings (holes not counted)");
    if accepted.is_empty() {
        println!("no verified candidate reduces this");
    }
    for item in &accepted {
        println!(
            "  + {}  ({:+} errors, {:+} warnings)",
            item.description, item.delta.0, item.delta.1
        );
    }
    let diagnostics = crate::check::check(&loaded.decls);
    let accepted_names: std::collections::HashSet<&str> = accepted
        .iter()
        .map(|item| item.description.as_str())
        .collect();
    let mut flat = 0;
    for (description, trial) in crate::fix::candidates(&loaded.decls) {
        if accepted_names.contains(description.as_str()) {
            continue;
        }
        if flat == 4 {
            break;
        }
        if flat == 0 {
            println!("candidates that do not pay:");
        }
        let (after_e, after_w) = crate::fix::score_without_holes(&trial);
        println!(
            "  - {description}  (no gain: {:+} errors, {:+} warnings)",
            after_e as isize - before_e as isize,
            after_w as isize - before_w as isize
        );
        flat += 1;
    }
    let blocked: Vec<&(&str, &str)> = NO_REPAIR
        .iter()
        .filter(|(code, _)| diagnostics.iter().any(|item| item.code == *code))
        .collect();
    if !blocked.is_empty() {
        println!("no machine repair for:");
        for (code, why) in blocked {
            println!("  {code}  {why}");
        }
    }
    if let Some(out) = apply {
        let mut current = loaded.decls.clone();
        let mut rounds = 0;
        while rounds < 6 {
            let (batch, _) = crate::fix::verified(&current);
            if batch.is_empty() {
                break;
            }
            current = batch
                .into_iter()
                .next()
                .map(|item| item.trial)
                .unwrap_or(current);
            rounds += 1;
        }
        let (after_e, after_w) = crate::fix::score_without_holes(&current);
        let text = crate::fix::render(&current);
        if let Some(reason) = open_chain(&text) {
            return Outcome::Refused(format!("refused: {reason}; resolve it, or record why"));
        }
        if let Err(error) = fs::write(out, &text) {
            return fail(format!("{out}: {error}"));
        }
        println!("applied {rounds} fix(es) -> {out} ({after_e} errors, {after_w} warnings)");
    }
    Outcome::Ok
}

pub(crate) fn cmd_propose(path: &Path, proposals_path: &str, apply: bool, policy: &str) -> Outcome {
    let loaded = match load(path) {
        Ok(loaded) => loaded,
        Err(error) => return fail(error),
    };
    let text = match fs::read_to_string(proposals_path) {
        Ok(text) => text,
        Err(error) => return fail(format!("{proposals_path}: {error}")),
    };
    let payload = match crate::json::parse(&text) {
        Ok(payload) => payload,
        Err(error) => return fail(format!("{proposals_path}: {error}")),
    };
    let items: Vec<crate::json::Json> = match &payload {
        crate::json::Json::Arr(items) => items.clone(),
        other => other
            .get("proposals")
            .and_then(|value| value.as_arr())
            .map(|items| items.to_vec())
            .unwrap_or_default(),
    };
    if items.is_empty() {
        println!("no proposals in {proposals_path}");
        return Outcome::Usage;
    }

    let mut current = loaded.text.clone();
    let mut admitted = 0;
    let mut needs_review = 0;
    for item in &items {
        let decl = item
            .get("decl")
            .and_then(|value| value.as_str())
            .unwrap_or("");
        let field = item
            .get("field")
            .and_then(|value| value.as_str())
            .unwrap_or("");
        let value = item
            .get("value")
            .and_then(|value| value.as_str())
            .unwrap_or("");
        let proposal = crate::propose::Proposal {
            decl: decl.to_string(),
            field: field.to_string(),
            value: value.to_string(),
        };
        let decls = match syntax::parse(&current) {
            Ok(decls) => decls,
            Err(error) => return fail(format!("the document stopped parsing: {error}")),
        };
        match crate::propose::verify(
            &current,
            &decls,
            &proposal,
            crate::propose::Policy::parse(policy),
        ) {
            crate::propose::Verdict::Admitted {
                errors,
                warnings,
                text,
            } => {
                admitted += 1;
                println!(
                    "{} admitted  ({errors:+} errors, {warnings:+} warnings)",
                    pad(&format!("{decl}.{field}"), 34)
                );
                current = text;
            }
            crate::propose::Verdict::NeedsReview { reason, text } => {
                needs_review += 1;
                println!("{} review    {reason}", pad(&format!("{decl}.{field}"), 34));
                current = text;
            }
            crate::propose::Verdict::Rejected { reason } => {
                println!("{} rejected  {reason}", pad(&format!("{decl}.{field}"), 34));
            }
        }
    }
    if apply && current != loaded.text {
        if let Some(reason) = open_chain(&current) {
            return Outcome::Refused(format!("refused: {reason}; resolve it, or record why"));
        }
        if let Err(error) = crate::apply::replace(path, &current) {
            return fail(format!("{}: {error}", path.display()));
        }
    }
    println!(
        "{admitted} admitted, {needs_review} review, {} rejected",
        items.len() - admitted - needs_review
    );
    Outcome::Ok
}
