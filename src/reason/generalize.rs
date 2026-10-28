//! Subsumption between declaration forms, and the meet it induces.
//!
//! A form with holes is a pattern: `kind` plus, per field, either a fixed shape or a variable.  One
//! pattern subsumes another when every constraint of the first is met by the second -- so the
//! subsuming one is the more general of the two.  Under subsumption the patterns form a preorder,
//! and two patterns have a meet: the most general pattern that both satisfy, which is the fields
//! they agree on, with everything else a variable.
//!
//! That meet is anti-unification restricted to shapes: it is a least general generalisation of the
//! two declarations, written down instead of guessed.  It says "these two have this much in common
//! and no more", which is the difference between seeing a family resemblance and being told one.

use std::collections::BTreeMap;

use crate::program::Program;
use crate::shape::shape_of;

/// A field is either pinned to a shape or free.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Slot {
    Pinned(String),
    Variable,
}

/// A declaration's form with holes: what is left after some fields are generalised away.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Pattern {
    pub kind: String,
    pub fields: BTreeMap<String, Slot>,
}

/// The pattern that pins every field: the declaration itself, read as a form.
pub fn pattern_of(program: &Program, name: &str) -> Option<Pattern> {
    let decl = program.get(name)?;
    let mut fields = BTreeMap::new();
    for (key, value) in &decl.fields {
        fields.insert(key.clone(), Slot::Pinned(shape_of(program, value, 0)));
    }
    Some(Pattern {
        kind: decl.kind.clone(),
        fields,
    })
}

/// Whether `general` subsumes `specific`: every constraint of the first holds in the second.  A
/// variable constrains nothing, and a field the general pattern does not mention is unconstrained.
pub fn subsumes(general: &Pattern, specific: &Pattern) -> bool {
    if general.kind != specific.kind {
        return false;
    }
    general
        .fields
        .iter()
        .all(|(key, slot)| match (slot, specific.fields.get(key)) {
            (Slot::Variable, _) => true,
            (Slot::Pinned(want), Some(Slot::Pinned(has))) => want == has,
            (Slot::Pinned(_), _) => false,
        })
}

/// The meet: the most general pattern both satisfy, or `None` when their kinds differ (the bottom
/// of the preorder pair does not exist in this lattice).
pub fn meet(left: &Pattern, right: &Pattern) -> Option<Pattern> {
    if left.kind != right.kind {
        return None;
    }
    let mut keys: Vec<&String> = left.fields.keys().chain(right.fields.keys()).collect();
    keys.sort();
    keys.dedup();
    let mut fields = BTreeMap::new();
    for key in keys {
        let slot = match (left.fields.get(key), right.fields.get(key)) {
            (Some(Slot::Pinned(a)), Some(Slot::Pinned(b))) if a == b => Slot::Pinned(a.clone()),
            _ => Slot::Variable,
        };
        fields.insert(key.clone(), slot);
    }
    Some(Pattern {
        kind: left.kind.clone(),
        fields,
    })
}

/// The constraints the meet keeps, in a readable line: `claim:text from:list[ref:fact]`.
pub fn text(pattern: &Pattern) -> String {
    let pinned: Vec<String> = pattern
        .fields
        .iter()
        .filter_map(|(key, slot)| match slot {
            Slot::Pinned(shape) => Some(format!("{key}:{shape}")),
            Slot::Variable => None,
        })
        .collect();
    let free: Vec<&str> = pattern
        .fields
        .iter()
        .filter(|(_, slot)| matches!(slot, Slot::Variable))
        .map(|(key, _)| key.as_str())
        .collect();
    if pinned.is_empty() {
        return format!("{} with everything free", pattern.kind);
    }
    let mut out = format!("{} with {}", pattern.kind, pinned.join(" "));
    if !free.is_empty() {
        out.push_str(&format!("; free: {}", free.join(", ")));
    }
    out
}

/// A hole-template for the meet: the shared constraints as a comment, every position as `?`.
/// This is the draft a reader has to fill in -- the pattern says what the family looks like, and
/// the holes are exactly the decisions a substitution has to make.
pub fn draft(pattern: &Pattern, name: &str) -> String {
    let mut out = format!("# {}\n", text(pattern));
    out.push_str(&format!("{} {} {{\n", pattern.kind, name));
    let fields: Vec<&String> = pattern.fields.keys().collect();
    for (index, field) in fields.iter().enumerate() {
        let comma = if index + 1 == fields.len() { "" } else { "," };
        out.push_str(&format!("  {field}: ?{comma}\n"));
    }
    out.push_str("}\n");
    out
}
