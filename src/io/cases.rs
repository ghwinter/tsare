//! Cases: what happened, in a form that can be retrieved and audited.
//!
//! A case is a situation, an action, and an outcome:
//!
//! ```text
//!     situation  the diagnostic codes and the kinds they landed on
//!     action     mechanical repair, model rewrite, or a dropped declaration
//!     outcome    errors and confirmations before and after
//! ```
//!
//! `recall` ranks cases by how much of the situation they share with the current one.  `audit`
//! checks the store itself: a memory that cannot say which of its entries are stale is a rumour.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::cache::SALT;
use crate::json::{self, Json};

/// Where the store is when the caller does not name one.  Nothing is written there unless a
/// command was handed `--record`, so the file appearing at all is the caller's decision.
pub fn store_path() -> PathBuf {
    PathBuf::from(".tsare").join("cases.jsonl")
}

pub fn load(store: &Path) -> Vec<Json> {
    std::fs::read_to_string(store)
        .map(|text| json::parse_lines(&text))
        .unwrap_or_default()
}

/// What a report says, in the four things a case remembers about it.
struct Summary {
    errors: usize,
    warnings: usize,
    codes: Vec<String>,
}

fn summary(report: &str) -> Result<Summary, String> {
    let parsed = json::parse(report).map_err(|error| format!("not a report: {error}"))?;
    let number = |key: &str| -> usize {
        parsed
            .get(key)
            .and_then(Json::as_f64)
            .unwrap_or(0.0)
            .max(0.0) as usize
    };
    let mut codes: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for item in parsed
        .get("diagnostics")
        .and_then(Json::as_arr)
        .unwrap_or(&[])
    {
        if let Some(code) = item.get("code").and_then(Json::as_str) {
            codes.insert(code.to_string());
        }
    }
    Ok(Summary {
        errors: number("errors"),
        warnings: number("warnings"),
        codes: codes.into_iter().collect(),
    })
}

/// One recorded outcome, as a line of JSON.  Everything in it is read from the two reports or from
/// the flags; the agent that made the edit supplies only the name of its own action, because a
/// memory narrated by the party being judged is the thing the audit exists to catch.
pub fn entry(
    before: &str,
    after: &str,
    doc: &str,
    action: &str,
    checker: &str,
) -> Result<String, String> {
    let left = summary(before)?;
    let right = summary(after)?;
    let block = |item: &Summary| {
        Json::Obj(vec![
            ("errors".to_string(), Json::Num(item.errors as f64)),
            ("warnings".to_string(), Json::Num(item.warnings as f64)),
        ])
    };
    let codes = Json::Arr(
        left.codes
            .iter()
            .cloned()
            .map(Json::Str)
            .collect::<Vec<Json>>(),
    );
    Ok(json::serialize(&Json::Obj(vec![
        ("doc".to_string(), Json::Str(doc.to_string())),
        ("codes".to_string(), codes),
        ("action".to_string(), Json::Str(action.to_string())),
        ("before".to_string(), block(&left)),
        ("after".to_string(), block(&right)),
        ("checker".to_string(), Json::Str(checker.to_string())),
    ])))
}

/// Append one entry.  The store is a line-per-case file so that a crash loses at most one case and
/// a reader can skip a malformed line instead of discarding the whole memory.
pub fn append(store: &Path, line: &str) -> std::io::Result<()> {
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(store)?;
    writeln!(file, "{line}")
}

fn pair(entry: &Json, side: &str) -> (f64, f64) {
    let block = entry.get(side);
    let errors = block
        .and_then(|item| item.get("errors"))
        .and_then(|item| item.as_f64())
        .unwrap_or(0.0);
    let warnings = block
        .and_then(|item| item.get("warnings"))
        .and_then(|item| item.as_f64())
        .unwrap_or(0.0);
    (errors, warnings)
}

/// Fewer errors, then fewer warnings: the same order the repair gate uses, so that a case that the
/// engine would have accepted outranks one it would have refused.
pub fn improved(entry: &Json) -> bool {
    let before = pair(entry, "before");
    let after = pair(entry, "after");
    after.0 < before.0 || (after.0 == before.0 && after.1 < before.1)
}

fn codes_of(entry: &Json) -> Vec<String> {
    entry
        .get("codes")
        .map(|value| value.strings())
        .unwrap_or_default()
}

fn action_of(entry: &Json) -> String {
    entry
        .get("action")
        .and_then(|value| value.as_str())
        .unwrap_or("")
        .to_string()
}

pub fn overlap(left: &[String], right: &[String]) -> f64 {
    let a: HashSet<&String> = left.iter().collect();
    let b: HashSet<&String> = right.iter().collect();
    if a.is_empty() && b.is_empty() {
        return 1.0;
    }
    a.intersection(&b).count() as f64 / a.union(&b).count() as f64
}

pub struct Recalled {
    pub entry: Json,
    pub improved: bool,
}

pub fn recall(store: &Path, codes: &[String], k: usize) -> Vec<Recalled> {
    let mut scored: Vec<(f64, bool, Json)> = Vec::new();
    for entry in load(store) {
        let score = overlap(codes, &codes_of(&entry));
        if score > 0.0 {
            scored.push((score, improved(&entry), entry));
        }
    }
    scored.sort_by(|left, right| {
        // best overlap first; a case that improved outranks one that did not
        right
            .0
            .partial_cmp(&left.0)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| right.1.cmp(&left.1))
    });
    scored
        .into_iter()
        .take(k)
        .map(|(_, improved, entry)| Recalled { entry, improved })
        .collect()
}

pub struct Audit {
    pub entries: usize,
    pub expired: usize,
    pub conflicting: usize,
    /// The counts changed but did not improve: a regression, not a no-op.
    pub regressed: usize,
    pub unchanged: usize,
    pub orphans: usize,
    /// Distinct situations (the code set): the quotient the memory is addressed by.
    pub signatures: usize,
    /// Cases whose situation was seen before: the reuse count.
    pub reused: usize,
    /// Situations seen exactly once: the orphan signatures, candidates for decay.
    pub single: usize,
    /// The actions, by how many times each was taken (the tool as text).
    pub actions: Vec<(String, usize)>,
}

pub fn audit(store: &Path) -> Audit {
    let entries = load(store);
    let expired = entries
        .iter()
        .filter(|entry| entry.get("checker").and_then(|value| value.as_str()) != Some(SALT))
        .count();
    let regressed = entries
        .iter()
        .filter(|entry| !improved(entry) && pair(entry, "after") != pair(entry, "before"))
        .count();
    let unchanged = entries
        .iter()
        .filter(|entry| pair(entry, "after") == pair(entry, "before"))
        .count();
    // A store and the documents it remembers belong to the same tree, so a relative path is read
    // from the store's directory and not from wherever the reader happens to stand: an audit that
    // calls every case orphan because it was run from another shell teaches nobody anything.
    let base = store.parent().unwrap_or(Path::new("."));
    let orphans = entries
        .iter()
        .filter(|entry| {
            entry
                .get("doc")
                .and_then(|value| value.as_str())
                .map(|doc| {
                    let path = Path::new(doc);
                    let resolved = if path.is_absolute() {
                        path.to_path_buf()
                    } else {
                        base.join(path)
                    };
                    !resolved.exists()
                })
                .unwrap_or(true)
        })
        .count();

    type GroupKey = (Vec<String>, String);
    let mut groups: Vec<(GroupKey, HashSet<bool>)> = Vec::new();
    for entry in &entries {
        let key = (codes_of(entry), action_of(entry));
        let improved = improved(entry);
        match groups.iter_mut().find(|(existing, _)| existing == &key) {
            Some((_, set)) => {
                set.insert(improved);
            }
            None => {
                let mut set = HashSet::new();
                set.insert(improved);
                groups.push((key, set));
            }
        }
    }
    let conflicting = groups.iter().filter(|(_, set)| set.len() > 1).count();

    let mut signature_counts: HashMap<Vec<String>, usize> = HashMap::new();
    let mut action_counts: BTreeMap<String, usize> = BTreeMap::new();
    for entry in &entries {
        *signature_counts.entry(codes_of(entry)).or_insert(0) += 1;
        *action_counts.entry(action_of(entry)).or_insert(0) += 1;
    }
    let signatures = signature_counts.len();
    let reused = entries.len() - signatures;
    let single = signature_counts
        .values()
        .filter(|count| **count == 1)
        .count();
    let actions: Vec<(String, usize)> = action_counts.into_iter().collect();

    Audit {
        entries: entries.len(),
        expired,
        conflicting,
        regressed,
        unchanged,
        orphans,
        signatures,
        reused,
        single,
        actions,
    }
}

/// A bracketed, single-quoted list, because the store prints its code list the way it was written.
pub fn list_repr(items: &[String]) -> String {
    let quoted: Vec<String> = items
        .iter()
        .map(|item| format!("'{}'", item.replace('\'', "\\'")))
        .collect();
    format!("[{}]", quoted.join(", "))
}

impl Audit {
    /// The reuse rate: the cases whose situation was seen before, over all cases.
    pub fn reuse_rate(&self) -> f64 {
        if self.entries == 0 {
            0.0
        } else {
            self.reused as f64 / self.entries as f64
        }
    }

    /// The compression: cases per distinct situation.
    pub fn compression(&self) -> f64 {
        if self.signatures == 0 {
            0.0
        } else {
            self.entries as f64 / self.signatures as f64
        }
    }

    pub fn text(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!("cases:       {}\n", self.entries));
        out.push_str(&format!(
            "expired:     {}  (recorded under a different checker)\n",
            self.expired
        ));
        out.push_str(&format!(
            "conflicting: {}  (same situation and action, opposite outcome)\n",
            self.conflicting
        ));
        out.push_str(&format!(
            "regressed:   {}  (the counts changed, not for the better)\n",
            self.regressed
        ));
        out.push_str(&format!("unchanged:   {}\n", self.unchanged));
        out.push_str(&format!(
            "orphans:     {}  (their document is gone)\n",
            self.orphans
        ));
        out.push_str(&format!(
            "signatures:  {}  (distinct situations: the quotient the memory is addressed by)\n",
            self.signatures
        ));
        out.push_str(&format!(
            "reused:      {}  (a situation seen before)\n",
            self.reused
        ));
        out.push_str(&format!(
            "reuse rate:  {:.2}  (reused / cases; the compression ratio)\n",
            self.reuse_rate()
        ));
        out.push_str(&format!(
            "compression: {:.2}  (cases per signature)\n",
            self.compression()
        ));
        out.push_str(&format!(
            "single:      {}  (a situation seen once: the orphan signatures)\n",
            self.single
        ));
        let actions = if self.actions.is_empty() {
            "-".to_string()
        } else {
            self.actions
                .iter()
                .map(|(name, count)| format!("{name} {count}"))
                .collect::<Vec<_>>()
                .join(", ")
        };
        out.push_str(&format!("actions:     {actions}\n"));
        out
    }

    pub fn json(&self) -> Json {
        let actions: Vec<Json> = self
            .actions
            .iter()
            .map(|(name, count)| {
                Json::Obj(vec![
                    ("action".to_string(), Json::Str(name.clone())),
                    ("count".to_string(), Json::Num(*count as f64)),
                ])
            })
            .collect();
        Json::Obj(vec![
            ("cases".to_string(), Json::Num(self.entries as f64)),
            ("expired".to_string(), Json::Num(self.expired as f64)),
            (
                "conflicting".to_string(),
                Json::Num(self.conflicting as f64),
            ),
            ("regressed".to_string(), Json::Num(self.regressed as f64)),
            ("unchanged".to_string(), Json::Num(self.unchanged as f64)),
            ("orphans".to_string(), Json::Num(self.orphans as f64)),
            ("signatures".to_string(), Json::Num(self.signatures as f64)),
            ("reused".to_string(), Json::Num(self.reused as f64)),
            ("reuse_rate".to_string(), Json::Num(self.reuse_rate())),
            ("compression".to_string(), Json::Num(self.compression())),
            ("single".to_string(), Json::Num(self.single as f64)),
            ("actions".to_string(), Json::Arr(actions)),
        ])
    }
}

impl crate::out::Report for Audit {
    fn text(&self) -> String {
        Audit::text(self)
    }

    fn json(&self) -> Json {
        Audit::json(self)
    }
}
