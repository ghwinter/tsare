//! The resolved program: names, accessors and the obligation table every analysis reads.
//!
//! `check` decides whether a declaration is well formed.  The analyses here ask what it
//! costs, what it supports, and what would fall if it were removed, so they need a name
//! index and one shared definition of what a declaration still owes.

use std::collections::HashMap;

use crate::syntax::{Decl, Val};

pub struct Program<'a> {
    pub decls: &'a [Decl],
    by_name: HashMap<&'a str, &'a Decl>,
}

impl<'a> Program<'a> {
    pub fn new(decls: &'a [Decl]) -> Self {
        let mut by_name = HashMap::new();
        for decl in decls {
            by_name.entry(decl.name.as_str()).or_insert(decl);
        }
        Program { decls, by_name }
    }

    pub fn get(&self, name: &str) -> Option<&'a Decl> {
        self.by_name.get(name).copied()
    }

    pub fn has(&self, name: &str) -> bool {
        self.by_name.contains_key(name)
    }

    pub fn of_kind(&self, kind: &str) -> Vec<&'a Decl> {
        self.decls.iter().filter(|decl| decl.kind == kind).collect()
    }

    pub fn count(&self, kind: &str) -> usize {
        self.decls.iter().filter(|decl| decl.kind == kind).count()
    }
}

pub fn field<'a>(decl: &'a Decl, key: &str) -> Option<&'a Val> {
    decl.fields
        .iter()
        .find(|(name, _)| name == key)
        .map(|(_, value)| value)
}

pub fn f_str<'a>(decl: &'a Decl, key: &str) -> Option<&'a str> {
    match field(decl, key) {
        Some(Val::Str(text)) => Some(text.as_str()),
        _ => None,
    }
}

/// The number written in a value: `16`, `16.5`, and `16ms` all read as 16 (and 16.5).
pub fn num_of(value: &Val) -> Option<f64> {
    match value {
        Val::Num(raw) => {
            let digits: String = raw
                .trim_start_matches('-')
                .chars()
                .take_while(|c| c.is_ascii_digit() || *c == '.')
                .collect();
            let sign = if raw.starts_with('-') { -1.0 } else { 1.0 };
            digits.parse::<f64>().ok().map(|number| sign * number)
        }
        _ => None,
    }
}

pub fn f_num(decl: &Decl, key: &str) -> Option<f64> {
    num_of(field(decl, key)?)
}

pub fn f_id<'a>(decl: &'a Decl, key: &str) -> Option<&'a str> {
    match field(decl, key) {
        Some(Val::Id(name)) => Some(name.as_str()),
        _ => None,
    }
}

pub fn f_ids(decl: &Decl, key: &str) -> Vec<String> {
    match field(decl, key) {
        Some(Val::List(items)) => items
            .iter()
            .filter_map(|item| match item {
                Val::Id(name) => Some(name.clone()),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

pub fn pos(decl: &Decl) -> Vec<String> {
    f_ids(decl, "grounded_by")
}

pub fn neg(decl: &Decl) -> Vec<String> {
    f_ids(decl, "against")
}

/// The command that measures a value, whether written plain or as `tool("...")`.
pub fn instrument_of<'a>(decl: &'a Decl, key: &str) -> Option<&'a str> {
    match field(decl, key) {
        Some(Val::Str(text)) => Some(text.as_str()),
        Some(Val::Call(_, args)) => match args.first() {
            Some(Val::Str(text)) => Some(text.as_str()),
            _ => None,
        },
        _ => None,
    }
}

pub fn has_field(decl: &Decl, key: &str) -> bool {
    decl.fields.iter().any(|(name, _)| name == key)
}

pub fn is_hole(decl: &Decl, key: &str) -> bool {
    matches!(field(decl, key), Some(Val::Hole))
}

pub const MEASURED: &[&str] = &["tool", "doc", "code"];

/// How well a fact is witnessed, as an order rather than four words.  Two functions used to answer
/// this question separately -- one here, one in `closure` -- and they disagreed about whether a
/// `confirmed_by` hole counts as a confirmation.  The order is what a verdict needs: a hypothesis
/// may be settled by `Confirmed` or better, never by the model's own say-so.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Grade {
    /// nothing recorded, or a witness nobody stands behind
    Asserted,
    /// a model said so and no person confirmed it
    ModelSupplied,
    /// a person stands behind it: `evidence: user`, or a model claim with `confirmed_by`
    Confirmed,
    /// a machine can produce it again: `tool(...)`, `code(...)`, `doc(...)`
    Measured,
}

impl Grade {
    pub fn label(self) -> &'static str {
        match self {
            Grade::Measured => "measured",
            Grade::Confirmed => "confirmed",
            Grade::ModelSupplied => "unconfirmed",
            Grade::Asserted => "asserted",
        }
    }
}

pub fn grade(decl: &Decl) -> Grade {
    let name = match field(decl, "evidence") {
        Some(Val::Id(name)) => Some(name.as_str()),
        Some(Val::Call(name, _)) => Some(name.as_str()),
        _ => None,
    };
    match name {
        Some(name) if MEASURED.contains(&name) => Grade::Measured,
        Some("user") | Some("observation") => Grade::Confirmed,
        Some("model") => {
            if f_str(decl, "confirmed_by").is_some() {
                Grade::Confirmed
            } else {
                Grade::ModelSupplied
            }
        }
        _ => Grade::Asserted,
    }
}

pub fn evidence_status(decl: &Decl) -> &'static str {
    grade(decl).label()
}

pub fn support_edges(prog: &Program) -> HashMap<String, Vec<String>> {
    let mut edges: HashMap<String, Vec<String>> = HashMap::new();
    for warrant in prog.of_kind("warrant") {
        if let Some(target) = f_id(warrant, "to") {
            edges
                .entry(target.to_string())
                .or_default()
                .extend(f_ids(warrant, "from"));
        }
    }
    for proposition in prog.of_kind("proposition") {
        edges
            .entry(proposition.name.clone())
            .or_default()
            .extend(f_ids(proposition, "from"));
    }
    for hypothesis in prog.of_kind("hypothesis") {
        edges
            .entry(hypothesis.name.clone())
            .or_default()
            .extend(f_ids(hypothesis, "assumes"));
    }
    edges
}

/// A step of a support walk: the fact, how it was obtained, and the path that reached it.
pub struct Step {
    pub fact: String,
    pub status: &'static str,
    pub path: Vec<String>,
}

pub fn walk(prog: &Program, start: &str, edges: &HashMap<String, Vec<String>>) -> Vec<Step> {
    let mut found: Vec<Step> = Vec::new();
    let mut seen: Vec<String> = vec![start.to_string()];
    let mut queue: std::collections::VecDeque<Vec<String>> = std::collections::VecDeque::new();
    queue.push_back(vec![start.to_string()]);
    while let Some(path) = queue.pop_front() {
        let last = path.last().cloned().unwrap_or_default();
        let Some(premises) = edges.get(&last) else {
            continue;
        };
        for premise in premises {
            if seen.iter().any(|name| name == premise) {
                continue;
            }
            seen.push(premise.clone());
            let Some(decl) = prog.get(premise) else {
                continue;
            };
            let mut step = path.clone();
            step.push(premise.clone());
            if decl.kind == "fact" {
                found.push(Step {
                    fact: premise.clone(),
                    status: evidence_status(decl),
                    path: step,
                });
            } else {
                queue.push_back(step);
            }
        }
    }
    found
}

pub const TABLE: &[(&str, &str, &str)] = &[
    (
        "fact",
        "evidence someone else can check",
        "run the tool, or ask the person who saw it",
    ),
    (
        "concept",
        "a positive and a negative example",
        "find a counterexample, or drop the concept",
    ),
    (
        "proposition",
        "a rule that connects its premises",
        "name the warrant, or mark it assumed",
    ),
    (
        "hypothesis",
        "a test or a rival",
        "design the observation that would refute it",
    ),
    (
        "question",
        "two answers that split two live hypotheses",
        "ask it of a named respondent",
    ),
    (
        "blindspot",
        "a witness pair plus the question that would close it",
        "add closes_with",
    ),
    (
        "value",
        "an instrument and a threshold",
        "name the command that measures it",
    ),
    (
        "warrant",
        "a premise and a basis",
        "say what makes the rule acceptable",
    ),
    (
        "law",
        "something that would break if it failed",
        "point checked_by at that thing",
    ),
    (
        "layer",
        "an interface that hides its internals",
        "expose exactly what others may use",
    ),
    (
        "partition",
        "a dimension, or a stated lack of one",
        "name the concept the elements share, or the question that would",
    ),
    (
        "note",
        "nothing; it is not a commitment",
        "promote it to a typed declaration to make it count",
    ),
];

/// Everything that still costs an external confirmation for this declaration.
pub fn obligations(prog: &Program, decl: &Decl) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for (key, value) in &decl.fields {
        if matches!(value, Val::Hole) {
            out.push(format!("hole: {key}"));
        }
    }
    match decl.kind.as_str() {
        "fact" => {
            if matches!(field(decl, "evidence"), Some(Val::Id(name)) if name == "user" || name == "model")
            {
                out.push(format!("evidence is {}, not measured", grade(decl).label()));
            }
        }
        "proposition" => {
            let supported = prog
                .of_kind("warrant")
                .iter()
                .any(|warrant| f_id(warrant, "to") == Some(decl.name.as_str()));
            if !supported {
                out.push("no rule supports it".to_string());
            }
        }
        "value" => {
            if instrument_of(decl, "measured_by").is_none() {
                out.push("no instrument to measure it".to_string());
            }
        }
        "hypothesis" => {
            if f_str(decl, "test").is_none() {
                out.push("no test to run".to_string());
            }
        }
        "question" => {
            if f_id(decl, "to").unwrap_or("user") == "user" {
                out.push("needs an answer from the user".to_string());
            }
        }
        "blindspot" => {
            if f_ids(decl, "closes_with").is_empty() {
                out.push("no question would close it".to_string());
            }
        }
        "warrant" => {
            if f_ids(decl, "basis").is_empty() {
                out.push("rule with no basis: why should it be acceptable?".to_string());
            }
        }
        "law" => {
            if f_ids(decl, "checked_by").is_empty() {
                out.push("no checker".to_string());
            }
        }
        "partition" => {
            let closure = f_id(decl, "closure").unwrap_or("open");
            if closure == "unknown" {
                out.push("no dimension yet: the pile has no type".to_string());
            } else if closure == "closed" && f_ids(decl, "basis").is_empty() {
                out.push("closed with no basis".to_string());
            }
        }
        _ => {}
    }
    out
}

/// Python's `str(float)`: an integral value keeps its `.0`.
pub fn repr_float(value: f64) -> String {
    if value.is_finite() && value.fract() == 0.0 && value.abs() < 1e16 {
        format!("{value:.1}")
    } else {
        format!("{value}")
    }
}

/// Python's `f"{value:g}"` for the values a document states.
pub fn fmt_g(value: f64) -> String {
    if value.is_finite() && value.fract() == 0.0 && value.abs() < 1e6 {
        format!("{}", value as i64)
    } else {
        format!("{value}")
    }
}
