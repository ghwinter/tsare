//! Commands that propose or apply a repair.

use super::*;

pub(crate) fn cmd_suggest(path: &Path, json: bool) -> i32 {
    let loaded = match load(path) {
        Ok(loaded) => loaded,
        Err(error) => return fail(error),
    };
    let items = crate::fix::suggestions(&loaded.decls);
    if !json {
        if items.is_empty() {
            println!("no verified suggestion");
            return 0;
        }
        for item in &items {
            println!("{} {}", pad(item.applicability, 18), item.message);
        }
        return 0;
    }
    let rendered: Vec<String> = items
        .iter()
        .map(|item| {
            format!(
                "{{\"applicability\": \"{}\", \"message\": \"{}\", \"file\": \"{}\", \"span\": {{\"line\": {}, \"col\": {}, \"end_line\": {}, \"end_col\": {}}}, \"replacement\": \"{}\"}}",
                item.applicability,
                escape(&item.message),
                escape(&item.file),
                item.line,
                item.col,
                item.end_line,
                item.end_col,
                escape(&item.replacement)
            )
        })
        .collect();
    println!("{{\"suggestions\": [{}]}}", rendered.join(", "));
    0
}

pub(crate) fn cmd_apply(path: &Path, dry: bool) -> i32 {
    let loaded = match load(path) {
        Ok(loaded) => loaded,
        Err(error) => return fail(error),
    };
    let items = crate::fix::suggestions(&loaded.decls);
    let (updated, applied) = crate::apply::apply_best(&loaded.text, &items);
    if !dry && updated != loaded.text {
        if let Err(error) = crate::apply::replace(path, &updated) {
            return fail(format!("{}: {error}", path.display()));
        }
    }
    let list: Vec<String> = applied
        .iter()
        .map(|item| format!("\"{}\"", escape(item)))
        .collect();
    println!("{{\"applied\": [{}]}}", list.join(", "));
    0
}

/// Codes whose repair needs a value only the author can choose.  Naming them is the honest
/// alternative to printing nothing and letting the reader assume no repair was needed.
const NO_REPAIR: &[(&str, &str)] = &[
    (
        "W-EXAMPLE-SHAPE",
        "needs an attribute the examples share; only the author knows one",
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

pub(crate) fn cmd_fix(path: &Path, apply: Option<&str>) -> i32 {
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
        fs::write(out, crate::fix::render(&current)).expect("write file");
        println!("applied {rounds} fix(es) -> {out} ({after_e} errors, {after_w} warnings)");
    }
    0
}

pub(crate) fn cmd_propose(path: &Path, proposals_path: &str, apply: bool, policy: &str) -> i32 {
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
        return 2;
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
        if let Err(error) = crate::apply::replace(path, &current) {
            return fail(format!("{}: {error}", path.display()));
        }
    }
    println!(
        "{admitted} admitted, {needs_review} review, {} rejected",
        items.len() - admitted - needs_review
    );
    0
}
