//! The similarity index: term frequency, cosine, an inverted index over declarations.
//!
//! No embeddings, no model: the words a declaration already contains are its vector.  What it is
//! for is the question a checker cannot answer -- "which of these say the same thing in different
//! words" -- and it answers it with a number that can be argued with, because the shared terms can
//! be printed.
//!
//! Limits: Chinese is one token per run of Han characters (see `tokens`), stop words are not
//! removed, and two declarations that share every word but mean the opposite score as one.

use std::collections::HashMap;

use crate::tokens;
use tsare_lang::syntax::{Decl, Val};

pub struct Pair {
    pub left: String,
    pub right: String,
    pub score: f64,
    pub shared: Vec<String>,
}

fn text_of(decl: &Decl) -> String {
    let mut out = String::new();
    for (_, value) in &decl.fields {
        match value {
            Val::Str(text) => {
                out.push_str(text);
                out.push(' ');
            }
            Val::List(items) => {
                for item in items {
                    if let Val::Str(text) = item {
                        out.push_str(text);
                        out.push(' ');
                    }
                }
            }
            Val::Rec(fields) => {
                for (key, item) in fields {
                    out.push_str(key);
                    out.push(' ');
                    if let Val::Str(text) = item {
                        out.push_str(text);
                        out.push(' ');
                    }
                }
            }
            _ => {}
        }
    }
    out
}

fn vector(decl: &Decl) -> HashMap<String, f64> {
    let mut out: HashMap<String, f64> = HashMap::new();
    for word in tokens(&text_of(decl)) {
        *out.entry(word).or_insert(0.0) += 1.0;
    }
    out
}

fn cosine(left: &HashMap<String, f64>, right: &HashMap<String, f64>) -> (f64, Vec<String>) {
    let mut dot = 0.0;
    let mut shared = Vec::new();
    for (word, count) in left {
        if let Some(other) = right.get(word) {
            dot += count * other;
            shared.push(word.clone());
        }
    }
    let norm = |vector: &HashMap<String, f64>| -> f64 {
        vector
            .values()
            .map(|value| value * value)
            .sum::<f64>()
            .sqrt()
    };
    let scale = norm(left) * norm(right);
    shared.sort();
    (if scale == 0.0 { 0.0 } else { dot / scale }, shared)
}

/// The nearest pairs, by cosine, among declarations that share at least one word.
pub fn nearest(decls: &[Decl], top: usize) -> Vec<Pair> {
    let vectors: Vec<(String, HashMap<String, f64>)> = decls
        .iter()
        .map(|decl| (decl.name.clone(), vector(decl)))
        .collect();
    let mut out = Vec::new();
    for left in 0..vectors.len() {
        for right in (left + 1)..vectors.len() {
            let (score, shared) = cosine(&vectors[left].1, &vectors[right].1);
            if shared.is_empty() {
                continue;
            }
            out.push(Pair {
                left: vectors[left].0.clone(),
                right: vectors[right].0.clone(),
                score,
                shared,
            });
        }
    }
    out.sort_by(|left, right| {
        right
            .score
            .partial_cmp(&left.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left.left.cmp(&right.left))
    });
    out.truncate(top);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use tsare_lang::syntax::parse;

    #[test]
    fn two_claims_that_share_their_words_are_near() {
        let source = "\
fact F_ONE { claim: \"the list scrolls at sixty frames\", evidence: user }
fact F_TWO { claim: \"the list scrolls at sixty frames per second\", evidence: user }
fact F_THREE { claim: \"the database writes in batches\", evidence: user }
";
        let decls = parse(source).expect("parses");
        let pairs = nearest(&decls, 3);
        assert_eq!(pairs[0].left, "F_ONE");
        assert_eq!(pairs[0].right, "F_TWO");
        assert!(pairs[0].score > 0.5);
    }
}
