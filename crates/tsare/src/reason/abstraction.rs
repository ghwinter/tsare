//! Concepts the document already holds but has not named.
//!
//! An abstraction is proposed here when three things hold:
//!
//! 1. it covers at least two facts -- one fact is not a class;
//! 2. it is not already the extent of a named concept -- naming it twice is `E-COLLAPSE`;
//! 3. it separates a pair: one fact inside, one outside.
//!
//! The third condition is the one worth arguing about.  A concept whose extent is every fact in
//! the document distinguishes nothing, and a concept whose intent is empty is a synonym for
//! "these".  The witness is what makes the proposal a distinction rather than a prettier name.

use std::collections::HashSet;

use crate::fca::{attributes, extent, intent, Pair};
use crate::program::Program;

pub struct Candidate {
    pub extent: Vec<String>,
    pub intent: Vec<Pair>,
    pub inside: String,
    pub outside: String,
}

/// Seeds are single attributes and pairs of attributes; each is closed by taking the intent of
/// its extent, so the proposal is a formal concept and not a guess.  `truncated` is true when the
/// seed list hit the cap, because a search that silently stops looks like a search that finished.
pub fn candidates(
    program: &Program,
    min_support: usize,
    max_intent: usize,
) -> (Vec<Candidate>, bool) {
    let facts = program.of_kind("fact");
    let total = facts.len();
    let mut seeds: Vec<Pair> = Vec::new();
    for fact in &facts {
        for pair in attributes(fact) {
            if !seeds.contains(&pair) {
                seeds.push(pair);
            }
        }
    }
    let cap = 400;
    let mut truncated = false;
    let mut work: Vec<Vec<Pair>> = seeds.iter().map(|pair| vec![pair.clone()]).collect();
    if seeds.len() * seeds.len() <= cap {
        for (index, left) in seeds.iter().enumerate() {
            for right in &seeds[index + 1..] {
                if left.0 == right.0 {
                    continue;
                }
                work.push(vec![left.clone(), right.clone()]);
            }
        }
    } else {
        truncated = true;
    }

    let named: HashSet<Vec<String>> = program
        .of_kind("concept")
        .into_iter()
        .map(|concept| {
            let mut names = crate::program::pos(concept);
            names.sort();
            names
        })
        .collect();

    let mut seen: HashSet<Vec<String>> = HashSet::new();
    let mut out: Vec<Candidate> = Vec::new();
    for seed in work {
        let members = extent(program, &seed);
        if members.len() < min_support.max(2) || members.len() >= total {
            continue;
        }
        let mut names: Vec<String> = members.iter().map(|fact| fact.name.clone()).collect();
        names.sort();
        // facts the recorded vocabulary cannot tell apart are one observation recorded twice
        // (`W-COLLIDE`); naming that class adds a synonym, not a distinction
        let identical = members
            .windows(2)
            .all(|pair| attributes(pair[0]) == attributes(pair[1]));
        if identical {
            continue;
        }
        if named.contains(&names) || !seen.insert(names.clone()) {
            continue;
        }
        let closed = intent(program, &members);
        if closed.is_empty() || closed.len() > max_intent {
            continue;
        }
        let Some(outside) = facts
            .iter()
            .find(|fact| !names.contains(&fact.name))
            .map(|fact| fact.name.clone())
        else {
            continue;
        };
        out.push(Candidate {
            inside: names[0].clone(),
            outside,
            extent: names,
            intent: closed,
        });
    }
    out.sort_by(|left, right| {
        right
            .intent
            .len()
            .cmp(&left.intent.len())
            .then(right.extent.len().cmp(&left.extent.len()))
            .then(left.extent.cmp(&right.extent))
    });
    (out, truncated)
}

pub fn text(candidates: &[Candidate], truncated: bool) -> String {
    if candidates.is_empty() {
        let mut out = String::from(
            "nothing to propose: no attribute set covers two facts and leaves one out\n",
        );
        if truncated {
            out.push_str("(the seed search was cut off, so this is not a proof of absence)\n");
        }
        return out;
    }
    let mut out = format!("{} candidate concept(s)\n\n", candidates.len());
    for (index, candidate) in candidates.iter().enumerate() {
        out.push_str(&format!(
            "{}. {} fact(s): {}\n",
            index + 1,
            candidate.extent.len(),
            candidate.extent.join(", ")
        ));
        out.push_str(&format!(
            "   intent: {}\n",
            crate::fca::render_pairs(&candidate.intent)
        ));
        if candidate.intent.len() == 1 {
            out.push_str("   (one attribute: the concept would be a synonym for that value)\n");
        }
        out.push_str(&format!(
            "   witness: {} in, {} out\n",
            candidate.inside, candidate.outside
        ));
        out.push_str(&format!(
            "   concept C_PROPOSED_{} {{\n     name: \"{}\",\n     grounded_by: [{}],\n     against: [{}]\n   }}\n\n",
            index + 1,
            crate::fca::render_pairs(&candidate.intent),
            candidate.extent.join(", "),
            candidate.outside
        ));
    }
    if truncated {
        out.push_str("(the seed search was cut off; there may be more)\n");
    }
    out
}
