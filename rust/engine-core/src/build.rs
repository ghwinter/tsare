//! What a checked document emits: obligations, learned clauses, open questions, notes.
//!
//! The artifacts are the point of the compiler for a person: they say what still has to be
//! discharged, what was learned from the examples, and what the document cannot express yet.

use std::collections::HashMap;

use crate::induce::{attrs_of, clause_text, induce_concept, Learning, Rhs};
use crate::patterns::{matches, render as render_pattern, Pattern, Target};
use crate::program::{f_id, f_ids, f_num, f_str, fmt_g, pos, Program};
use crate::syntax::{Decl, Val};

pub const ACTIONS: &[(&str, &str)] = &[
    ("fact", "check"),
    ("concept", "induce"),
    ("proposition", "prove"),
    ("hypothesis", "test"),
    ("question", "ask"),
    ("blindspot", "extend"),
    ("value", "measure"),
    ("partition", "close"),
    ("warrant", "justify"),
    ("law", "watch"),
    ("layer", "expose"),
    ("note", "promote"),
];

pub fn action_of(kind: &str) -> &'static str {
    ACTIONS.iter().find(|(name, _)| *name == kind).map(|(_, action)| *action).unwrap_or("?")
}

fn evidence_name(decl: &Decl) -> Option<String> {
    match decl.fields.iter().find(|(key, _)| key == "evidence").map(|(_, value)| value) {
        Some(Val::Id(name)) => Some(name.clone()),
        Some(Val::Call(name, _)) => Some(name.clone()),
        _ => None,
    }
}

pub struct Obligation {
    pub action: &'static str,
    pub state: &'static str,
    pub note: String,
}

pub fn obligation(decl: &Decl, _prog: &Program, learnings: &HashMap<String, Learning>) -> Obligation {
    let action = action_of(&decl.kind);
    match decl.kind.as_str() {
        "fact" => {
            let confirmed = f_str(decl, "confirmed_by");
            let name = evidence_name(decl);
            if name.as_deref() == Some("model") && confirmed.is_none() {
                return Obligation {
                    action,
                    state: "blocked",
                    note: "model-supplied, not confirmed".to_string(),
                };
            }
            if name.as_deref() == Some("model") {
                return Obligation {
                    action,
                    state: "ready",
                    note: "model evidence, confirmed by user".to_string(),
                };
            }
            Obligation {
                action,
                state: "ready",
                note: format!("evidence: {}", name.unwrap_or_else(|| "tool".to_string())),
            }
        }
        "concept" => {
            if pos(decl).is_empty() {
                return Obligation { action, state: "blocked", note: "no positive example".to_string() };
            }
            if crate::program::neg(decl).is_empty() {
                return Obligation { action, state: "blocked", note: "no counterexample".to_string() };
            }
            match learnings.get(&decl.name) {
                Some(learning) if learning.clause.is_some() => Obligation {
                    action,
                    state: "ready",
                    note: format!("clause: {}", clause_text(learning.clause.as_ref().unwrap())),
                },
                _ => Obligation {
                    action,
                    state: "blocked",
                    note: "no clause found; see OPEN.md".to_string(),
                },
            }
        }
        "proposition" => {
            if !f_ids(decl, "from").is_empty() {
                Obligation { action, state: "ready", note: "premises given".to_string() }
            } else {
                Obligation {
                    action,
                    state: "assumed",
                    note: "no premises; treated as an assumption".to_string(),
                }
            }
        }
        "hypothesis" => {
            let verdict = f_id(decl, "verdict").unwrap_or("");
            if verdict == "supported" || verdict == "refuted" {
                return Obligation {
                    action,
                    state: "done",
                    note: format!("{verdict} by {}", f_id(decl, "verdict_by").unwrap_or("")),
                };
            }
            match f_str(decl, "test") {
                Some(test) if !test.is_empty() => Obligation {
                    action,
                    state: "ready",
                    note: format!(
                        "test: {}",
                        if test.chars().count() > 44 {
                            format!("{}...", test.chars().take(44).collect::<String>())
                        } else {
                            test.to_string()
                        }
                    ),
                },
                _ => Obligation {
                    action,
                    state: "blocked",
                    note: "no test; only a rival can refute it".to_string(),
                },
            }
        }
        "question" => Obligation {
            action,
            state: "waiting",
            note: format!("to {}", f_id(decl, "to").unwrap_or("user")),
        },
        "blindspot" => Obligation {
            action,
            state: "waiting",
            note: "signature must grow".to_string(),
        },
        "value" => {
            let unit = f_str(decl, "unit");
            let threshold = f_num(decl, "threshold");
            match (unit, threshold) {
                (Some(unit), Some(threshold)) if !unit.is_empty() => Obligation {
                    action,
                    state: "ready",
                    note: format!(
                        "{} {} {}",
                        f_str(decl, "metric").unwrap_or(""),
                        f_id(decl, "direction").unwrap_or(""),
                        fmt_g(threshold)
                    ),
                },
                _ => Obligation {
                    action,
                    state: "blocked",
                    note: "no unit or threshold".to_string(),
                },
            }
        }
        "partition" => {
            let closure = f_id(decl, "closure").unwrap_or("open");
            let basis = f_ids(decl, "basis");
            if closure == "unknown" {
                Obligation {
                    action,
                    state: "blocked",
                    note: "no dimension yet, so the list is a pile and cannot be used".to_string(),
                }
            } else if closure == "closed" && !basis.is_empty() {
                Obligation {
                    action,
                    state: "ready",
                    note: format!("closed by {}", basis.join(", ")),
                }
            } else if closure == "closed" {
                Obligation {
                    action,
                    state: "blocked",
                    note: "closed with no reason, so the list asserts more than it shows".to_string(),
                }
            } else {
                Obligation {
                    action,
                    state: "waiting",
                    note: "open: the list is not exhaustive, and that has to be said".to_string(),
                }
            }
        }
        "warrant" => {
            let basis = f_ids(decl, "basis");
            if basis.is_empty() {
                Obligation {
                    action,
                    state: "blocked",
                    note: "rule with no basis: why should it be acceptable?".to_string(),
                }
            } else {
                Obligation { action, state: "ready", note: format!("basis: {}", basis.join(", ")) }
            }
        }
        "law" => {
            let checked = f_ids(decl, "checked_by");
            if checked.is_empty() {
                Obligation {
                    action,
                    state: "blocked",
                    note: "nothing would break if it were false".to_string(),
                }
            } else {
                Obligation { action, state: "ready", note: format!("checked by {}", checked.join(", ")) }
            }
        }
        "layer" => {
            let exposes = f_ids(decl, "exposes");
            if exposes.is_empty() {
                Obligation {
                    action,
                    state: "blocked",
                    note: "exposes nothing, so it hides nothing".to_string(),
                }
            } else {
                Obligation {
                    action,
                    state: "ready",
                    note: format!("exposes {} name(s)", exposes.len()),
                }
            }
        }
        "note" => Obligation {
            action,
            state: "none",
            note: "not a commitment".to_string(),
        },
        _ => Obligation { action, state: "?", note: String::new() },
    }
}

pub fn render_obligations(prog: &Program, learnings: &HashMap<String, Learning>) -> String {
    let mut lines = vec![
        "# Obligations".to_string(),
        String::new(),
        "What must be discharged before the description counts as checked.".to_string(),
        String::new(),
        "| id | kind | action | status | note |".to_string(),
        "| --- | --- | --- | --- | --- |".to_string(),
    ];
    let mut ready = 0;
    for decl in prog.decls {
        let row = obligation(decl, prog, learnings);
        if row.state == "ready" {
            ready += 1;
        }
        lines.push(format!(
            "| {} | {} | {} | {} | {} |",
            decl.name, decl.kind, row.action, row.state, row.note
        ));
    }
    lines.push(String::new());
    lines.push(format!("{} obligations, {ready} ready.", prog.decls.len()));
    lines.push(String::new());
    lines.join("\n")
}

pub fn render_learned(learnings: &[Learning], structurals: &[(String, Option<Pattern>)]) -> String {
    let mut lines = vec![
        "# Learned clauses".to_string(),
        String::new(),
        "Least generalisation over recorded attributes.".to_string(),
        String::new(),
        "| concept | positives | negatives | clause |".to_string(),
        "| --- | --- | --- | --- |".to_string(),
    ];
    for learning in learnings {
        let clause = match &learning.clause {
            Some(clause) => clause_text(clause),
            None => "_failed_".to_string(),
        };
        lines.push(format!(
            "| {} | {} | {} | {} |",
            learning.concept,
            learning.positives.len(),
            learning.negatives.len(),
            clause
        ));
    }
    lines.push(String::new());
    let structural: Vec<&(String, Option<Pattern>)> =
        structurals.iter().filter(|(_, rule)| rule.is_some()).collect();
    if !structural.is_empty() {
        lines.push("## Structural clauses".to_string());
        lines.push(String::new());
        lines.push("Learned from graph position, not attributes.".to_string());
        lines.push(String::new());
        lines.push("| concept | structure |".to_string());
        lines.push("| --- | --- |".to_string());
        for (name, rule) in structural {
            lines.push(format!("| {name} | {} |", render_pattern(rule.as_ref().unwrap())));
        }
        lines.push(String::new());
    }
    let failed: Vec<&Learning> = learnings.iter().filter(|item| item.clause.is_none()).collect();
    if !failed.is_empty() {
        lines.push("## Failures".to_string());
        lines.push(String::new());
        for learning in failed {
            lines.push(format!("- **{}**: {}", learning.concept, learning.reason));
            if let Some((left, right)) = &learning.witness {
                lines.push(format!("  - witness: `{left}` vs `{right}`"));
            }
        }
        lines.push(String::new());
    }
    lines.join("\n")
}

pub fn render_notes(prog: &Program) -> String {
    let notes = prog.of_kind("note");
    let mut lines = vec![
        "# Notes".to_string(),
        String::new(),
        "Not commitments. Nothing may depend on them; they exist so a thought can be written".to_string(),
        "before it is judged. Promote one to a typed declaration when it starts doing work.".to_string(),
        String::new(),
    ];
    if notes.is_empty() {
        lines.push("_none_".to_string());
    }
    for decl in &notes {
        let tag = f_id(decl, "tag").unwrap_or("note");
        let text = f_str(decl, "text").unwrap_or("");
        let about = f_ids(decl, "about").join(", ");
        let suffix = if about.is_empty() { String::new() } else { format!(" (about: {about})") };
        lines.push(format!("- **{tag}** `{}`: {text}{suffix}", decl.name));
    }
    lines.join("\n")
}

pub fn render_open(
    prog: &Program,
    learnings: &[Learning],
    holes: &[(String, String)],
) -> String {
    let mut lines = vec!["# Open".to_string(), String::new()];
    if !holes.is_empty() {
        lines.push("## Holes".to_string());
        lines.push(String::new());
        lines.push("Values marked `?`: known unknowns, not errors.".to_string());
        lines.push(String::new());
        for (name, key) in holes {
            lines.push(format!("- **{name}.{key}**"));
        }
        lines.push(String::new());
    }
    lines.push("## Declared questions".to_string());
    lines.push(String::new());
    let questions = prog.of_kind("question");
    if questions.is_empty() {
        lines.push("_none_".to_string());
    }
    for decl in &questions {
        let asks = f_str(decl, "asks").unwrap_or("");
        let answers: Vec<String> = match decl.fields.iter().find(|(key, _)| key == "answers").map(|(_, value)| value) {
            Some(Val::List(items)) => items
                .iter()
                .filter_map(|item| match item {
                    Val::Str(text) => Some(text.clone()),
                    _ => None,
                })
                .collect(),
            _ => Vec::new(),
        };
        lines.push(format!("**{}** (to {}): {asks}", decl.name, f_id(decl, "to").unwrap_or("user")));
        lines.push(String::new());
        for (index, answer) in answers.iter().enumerate() {
            lines.push(format!("{}. {answer}", index + 1));
        }
        lines.push(String::new());
    }

    lines.push("## Blind spots".to_string());
    lines.push(String::new());
    let spots = prog.of_kind("blindspot");
    if spots.is_empty() {
        lines.push("_none_".to_string());
    }
    for decl in &spots {
        let witness = f_ids(decl, "witness");
        lines.push(format!("**{}**: {}", decl.name, f_str(decl, "cannot_express").unwrap_or("")));
        lines.push(String::new());
        lines.push(if witness.len() == 2 {
            format!("- witness: `{}` vs `{}`", witness[0], witness[1])
        } else {
            "- witness: ?".to_string()
        });
        lines.push("- needs: a predicate that separates them".to_string());
        lines.push(String::new());
    }

    let failures: Vec<&Learning> = learnings.iter().filter(|item| item.clause.is_none()).collect();
    let blocked: Vec<&&Learning> = failures.iter().filter(|item| item.witness.is_none()).collect();
    let collisions: Vec<&&Learning> = failures.iter().filter(|item| item.witness.is_some()).collect();
    lines.push("## Blocked concepts".to_string());
    lines.push(String::new());
    if blocked.is_empty() {
        lines.push("_none_".to_string());
    }
    for learning in blocked {
        lines.push(format!("- **{}**: {}", learning.concept, learning.reason));
    }
    lines.push(String::new());
    lines.push("## Derived questions".to_string());
    lines.push(String::new());
    lines.push("Produced by induction failure, not written by hand.".to_string());
    lines.push(String::new());
    if collisions.is_empty() {
        lines.push("_none_".to_string());
    }
    for learning in collisions {
        let (asks, answers) = crate::induce::auto_question(learning);
        lines.push(format!("**{}**: {asks}", learning.concept));
        lines.push(String::new());
        for (index, answer) in answers.iter().enumerate() {
            lines.push(format!("{}. {answer}", index + 1));
        }
        lines.push(String::new());
    }
    lines.join("\n")
}

pub fn render_coverage(goals: &[crate::coverage::Goal]) -> String {
    let mut lines = vec![
        "# Coverage".to_string(),
        String::new(),
        "Which line of the brief is answered by a commitment rather than an observation.".to_string(),
        String::new(),
        "| goal | addressed by |".to_string(),
        "| --- | --- |".to_string(),
    ];
    for goal in goals {
        let hits = if goal.hits.is_empty() {
            "_nothing_".to_string()
        } else {
            goal.hits.join(", ")
        };
        lines.push(format!("| {} | {hits} |", goal.name));
    }
    let covered = goals.iter().filter(|goal| goal.covered()).count();
    lines.push(String::new());
    lines.push(format!("{covered}/{} goals addressed.", goals.len()));
    lines.push(String::new());
    lines.join("\n")
}

const PRED_HEADER: &str = r#""""Generated by mf7 from {source}. Do not edit.

One function per concept, each the clause learned from that concept's positive and
negative examples.  A definition in the document is a function here.
"""


def _eq(attrs, key, value):
    return attrs.get(key) == value


def _le(attrs, key, bound):
    value = attrs.get(key)
    return isinstance(value, (int, float)) and value <= bound


def _ge(attrs, key, bound):
    value = attrs.get(key)
    return isinstance(value, (int, float)) and value >= bound
"#;

fn python_text(value: &str) -> String {
    let escaped = value.replace('\\', "\\\\").replace('\'', "\\'");
    format!("'{escaped}'")
}

fn python_repr_pattern(pattern: &Pattern) -> String {
    let branches: Vec<String> = pattern
        .iter()
        .map(|(labels, target)| {
            let labels: Vec<String> = labels.iter().map(|label| python_text(label)).collect();
            let target = match target {
                Target::Var(name) => format!("('var', {})", python_text(name)),
                Target::Const(name) => format!("('const', {})", python_text(name)),
            };
            let tuple = if labels.len() == 1 {
                format!("({},)", labels[0])
            } else {
                format!("({})", labels.join(", "))
            };
            format!("({tuple}, {target})")
        })
        .collect();
    // the learned rule is a tuple of branches, so a one-branch rule keeps its comma
    if branches.len() == 1 {
        format!("({},)", branches[0])
    } else {
        format!("({})", branches.join(", "))
    }
}

pub fn predicates_source(
    prog: &Program,
    learnings: &HashMap<String, Learning>,
    structurals: &[(String, Option<Pattern>)],
    filename: &str,
) -> String {
    let mut parts: Vec<String> = Vec::new();
    for decl in prog.of_kind("concept") {
        let Some(learning) = learnings.get(&decl.name) else { continue };
        let Some(clause) = &learning.clause else { continue };
        let mut terms: Vec<String> = Vec::new();
        for lit in clause {
            let rhs = match (&lit.rhs, lit.op) {
                (Rhs::Text(text), "=") => python_text(text),
                (Rhs::Num(value), _) => crate::program::repr_float(*value),
                (Rhs::Text(text), _) => python_text(text),
            };
            let call = match lit.op {
                "=" => "_eq",
                "<=" => "_le",
                _ => "_ge",
            };
            terms.push(format!("{call}(attrs, {}, {rhs})", python_text(&lit.key)));
        }
        let doc = format!(
            "positives: {}; negatives: {}",
            learning.positives.join(", "),
            learning.negatives.join(", ")
        );
        parts.push(format!(
            "def {}(attrs):\n    \"\"\"{doc}\"\"\"\n    return {}\n",
            decl.name,
            terms.join(" and ")
        ));
    }
    for decl in prog.of_kind("concept") {
        let Some(rule) = structurals.iter().find(|(name, _)| name == &decl.name).and_then(|(_, rule)| rule.clone())
        else {
            continue;
        };
        parts.push(format!(
            "_PATTERN_{} = {}\n\ndef {}_STRUCT(prog, name):\n    \"\"\"structure: {}\"\"\"\n    from patterns import matches\n    return matches(prog, name, _PATTERN_{})\n",
            decl.name,
            python_repr_pattern(&rule),
            decl.name,
            render_pattern(&rule),
            decl.name
        ));
    }
    format!("{}\n\n{}", PRED_HEADER.replace("{source}", filename), parts.join("\n\n"))
}

/// Verify the generated definitions by evaluating them, not by running Python.
pub fn verify_predicates(
    prog: &Program,
    learnings: &HashMap<String, Learning>,
    structurals: &[(String, Option<Pattern>)],
) -> Vec<String> {
    let mut problems = Vec::new();
    for decl in prog.of_kind("concept") {
        let Some(learning) = learnings.get(&decl.name) else { continue };
        let Some(clause) = &learning.clause else { continue };
        for name in &learning.positives {
            let Some(fact) = prog.get(name) else { continue };
            let attrs = attrs_of(fact);
            if !clause.iter().all(|lit| lit.holds(&attrs)) {
                problems.push(format!("{}: rejected positive {name}", decl.name));
            }
        }
        for name in &learning.negatives {
            let Some(fact) = prog.get(name) else { continue };
            let attrs = attrs_of(fact);
            if clause.iter().all(|lit| lit.holds(&attrs)) {
                problems.push(format!("{}: accepted negative {name}", decl.name));
            }
        }
    }
    for (name, rule) in structurals {
        let Some(rule) = rule else { continue };
        let Some(decl) = prog.get(name) else { continue };
        for positive in pos(decl) {
            if !matches(prog, &positive, rule) {
                problems.push(format!("{name}_STRUCT: rejected positive {positive}"));
            }
        }
        for negative in crate::program::neg(decl) {
            if matches(prog, &negative, rule) {
                problems.push(format!("{name}_STRUCT: accepted negative {negative}"));
            }
        }
    }
    problems
}

pub fn learnings_of(prog: &Program) -> Vec<Learning> {
    prog.of_kind("concept").iter().map(|concept| induce_concept(prog, concept)).collect()
}
