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
use tsare_lang::json::{self, Json};

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

/// Everything a recorded case carries.  The process and the witness are the R5 addition: the steps
/// taken and the rule that justified them, so the memory can be addressed by the process rather
/// than by the situation.
pub struct Record<'a> {
    pub before: &'a str,
    pub after: &'a str,
    pub doc: &'a str,
    pub action: &'a str,
    pub checker: &'a str,
    pub process: Option<&'a str>,
    pub witness: Option<&'a str>,
    pub signature: Option<&'a str>,
    pub abstracts: Option<&'a str>,
    pub refused: Option<&'a str>,
}

/// The canonical signature of a process: the sorted witness templates and step descriptors, hashed.
pub fn signature(witness: &[String], process: &[String]) -> String {
    let mut lines: Vec<&str> = Vec::new();
    lines.extend(witness.iter().map(|line| line.as_str()));
    lines.extend(process.iter().map(|line| line.as_str()));
    lines.sort();
    tsare_lang::hash::hex(lines.join("\n").as_bytes())
}

/// One recorded outcome, as a line of JSON.  Everything in it is read from the two reports or from
/// the flags; the agent that made the edit supplies only the name of its own action, because a
/// memory narrated by the party being judged is the thing the audit exists to catch.
pub fn entry(record: &Record) -> Result<String, String> {
    let left = summary(record.before)?;
    let right = summary(record.after)?;
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
    let parsed = |text: Option<&str>| -> Result<Json, String> {
        match text {
            Some(text) => json::parse(text),
            None => Ok(Json::Null),
        }
    };
    let process = parsed(record.process).map_err(|error| format!("not a process: {error}"))?;
    let witness = parsed(record.witness).map_err(|error| format!("not a witness: {error}"))?;
    let abstracts = parsed(record.abstracts).map_err(|error| format!("not abstracts: {error}"))?;
    Ok(json::serialize(&Json::Obj(vec![
        ("doc".to_string(), Json::Str(record.doc.to_string())),
        ("codes".to_string(), codes),
        ("process".to_string(), process),
        ("witness".to_string(), witness),
        (
            "signature".to_string(),
            match record.signature {
                Some(signature) => Json::Str(signature.to_string()),
                None => Json::Null,
            },
        ),
        ("abstracts".to_string(), abstracts),
        (
            "refused".to_string(),
            match record.refused {
                Some(reason) => Json::Str(reason.to_string()),
                None => Json::Null,
            },
        ),
        ("action".to_string(), Json::Str(record.action.to_string())),
        ("before".to_string(), block(&left)),
        ("after".to_string(), block(&right)),
        ("checker".to_string(), Json::Str(record.checker.to_string())),
    ])))
}

/// Recall by the process signature: the same steps over a different document.
pub fn recall_by_signature(store: &Path, signature: &str, k: usize) -> Vec<Recalled> {
    load(store)
        .into_iter()
        .filter(|entry| entry.get("signature").and_then(|value| value.as_str()) == Some(signature))
        .take(k)
        .map(|entry| Recalled {
            improved: improved(&entry),
            entry,
        })
        .collect()
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
    /// Distinct situations (the code set): the first quotient of the memory.
    pub situations: usize,
    /// Cases whose situation was seen before.
    pub situation_reuse: usize,
    /// Situations seen exactly once: the orphan signatures, candidates for decay.
    pub situation_single: usize,
    /// The actions, by how many times each was taken (the tool as text).
    pub actions: Vec<(String, usize)>,
    /// Distinct process signatures: the same steps over different documents.
    pub processes: usize,
    /// Cases whose process signature was seen before.
    pub process_reuse: usize,
    /// Process signatures over more than one document: the cross-task reuse.
    pub cross_task: usize,
    /// Cases recorded as refused by a gate.
    pub refused: usize,
    /// Process signatures that carry a refusal: the vetoed processes.
    pub vetoed: usize,
    /// How many cases instantiated each library abstract.
    pub abstracts: Vec<(String, usize)>,
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
    let situations = signature_counts.len();
    let situation_reuse = entries.len() - situations;
    let situation_single = signature_counts
        .values()
        .filter(|count| **count == 1)
        .count();
    let actions: Vec<(String, usize)> = action_counts.into_iter().collect();
    let mut process_docs: HashMap<String, Vec<&str>> = HashMap::new();
    let mut refused = 0;
    for entry in &entries {
        if let Some(signature) = entry.get("signature").and_then(|value| value.as_str()) {
            let doc = entry
                .get("doc")
                .and_then(|value| value.as_str())
                .unwrap_or("");
            process_docs
                .entry(signature.to_string())
                .or_default()
                .push(doc);
        }
        if entry
            .get("refused")
            .and_then(|value| value.as_str())
            .is_some()
        {
            refused += 1;
        }
    }
    let processes = process_docs.len();
    let process_cases = entries
        .iter()
        .filter(|entry| {
            entry
                .get("signature")
                .and_then(|value| value.as_str())
                .is_some()
        })
        .count();
    let process_reuse = process_cases - processes;
    let cross_task = process_docs
        .values()
        .filter(|docs| docs.iter().collect::<HashSet<_>>().len() > 1)
        .count();
    let mut abstract_uses: BTreeMap<String, usize> = BTreeMap::new();
    for entry in &entries {
        for name in entry
            .get("abstracts")
            .map(|value| value.strings())
            .unwrap_or_default()
        {
            *abstract_uses.entry(name).or_insert(0) += 1;
        }
    }
    let abstracts: Vec<(String, usize)> = abstract_uses.into_iter().collect();
    let vetoed = process_docs
        .iter()
        .filter(|(signature, _)| {
            entries.iter().any(|entry| {
                entry.get("signature").and_then(|value| value.as_str()) == Some(signature.as_str())
                    && entry
                        .get("refused")
                        .and_then(|value| value.as_str())
                        .is_some()
            })
        })
        .count();

    Audit {
        entries: entries.len(),
        expired,
        conflicting,
        regressed,
        unchanged,
        orphans,
        situations,
        situation_reuse,
        situation_single,
        actions,
        processes,
        process_reuse,
        cross_task,
        refused,
        vetoed,
        abstracts,
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
            self.situation_reuse as f64 / self.entries as f64
        }
    }

    /// The compression: cases per distinct situation.
    pub fn compression(&self) -> f64 {
        if self.situations == 0 {
            0.0
        } else {
            self.entries as f64 / self.situations as f64
        }
    }

    /// The process rate: the cases whose process signature was seen before, over process cases.
    pub fn process_rate(&self) -> f64 {
        let cases = self.process_reuse + self.processes;
        if cases == 0 {
            0.0
        } else {
            self.process_reuse as f64 / cases as f64
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
            "situations:  {}  (distinct diagnostic-code sets)\n",
            self.situations
        ));
        out.push_str(&format!(
            "situation reuse: {}  (a situation seen before)\n",
            self.situation_reuse
        ));
        out.push_str(&format!(
            "reuse rate:  {:.2}  (situation reuse / cases)\n",
            self.reuse_rate()
        ));
        out.push_str(&format!(
            "compression: {:.2}  (cases per situation)\n",
            self.compression()
        ));
        out.push_str(&format!(
            "situation single: {}  (a situation seen once)\n",
            self.situation_single
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
        out.push_str(&format!(
            "processes:   {}  (distinct process signatures: the same steps over different documents)\n",
            self.processes
        ));
        out.push_str(&format!(
            "process reuse: {}  (a process seen before)\n",
            self.process_reuse
        ));
        out.push_str(&format!(
            "process rate: {:.2}  (process reuse / process cases)\n",
            self.process_rate()
        ));
        out.push_str(&format!(
            "cross task:  {}  (a process over more than one document)\n",
            self.cross_task
        ));
        out.push_str(&format!(
            "refused:     {}  (a gate refused the process)\n",
            self.refused
        ));
        out.push_str(&format!(
            "vetoed:      {}  (a process signature with a refusal)\n",
            self.vetoed
        ));
        let abstracts = if self.abstracts.is_empty() {
            "-".to_string()
        } else {
            self.abstracts
                .iter()
                .map(|(name, count)| format!("{name} {count}"))
                .collect::<Vec<_>>()
                .join(", ")
        };
        out.push_str(&format!("abstracts:   {abstracts}\n"));
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
            ("situations".to_string(), Json::Num(self.situations as f64)),
            (
                "situation_reuse".to_string(),
                Json::Num(self.situation_reuse as f64),
            ),
            (
                "situation_single".to_string(),
                Json::Num(self.situation_single as f64),
            ),
            ("reuse_rate".to_string(), Json::Num(self.reuse_rate())),
            ("compression".to_string(), Json::Num(self.compression())),
            ("processes".to_string(), Json::Num(self.processes as f64)),
            (
                "process_reuse".to_string(),
                Json::Num(self.process_reuse as f64),
            ),
            ("process_rate".to_string(), Json::Num(self.process_rate())),
            ("cross_task".to_string(), Json::Num(self.cross_task as f64)),
            ("refused".to_string(), Json::Num(self.refused as f64)),
            ("vetoed".to_string(), Json::Num(self.vetoed as f64)),
            (
                "abstracts".to_string(),
                Json::Arr(
                    self.abstracts
                        .iter()
                        .map(|(name, count)| {
                            Json::Obj(vec![
                                ("name".to_string(), Json::Str(name.clone())),
                                ("count".to_string(), Json::Num(*count as f64)),
                            ])
                        })
                        .collect(),
                ),
            ),
            ("actions".to_string(), Json::Arr(actions)),
        ])
    }
}

impl tsare_core::out::Report for Audit {
    fn text(&self) -> String {
        Audit::text(self)
    }

    fn json(&self) -> Json {
        Audit::json(self)
    }
}
