//! The router: given a goal in words, rank the commands whose usage line speaks the same words.
//!
//! This is the honest core of "semantic routing" -- a bag of words and an overlap score, with no
//! embeddings and no model.  It is here to show the shape of the interface: a goal in, a ranked
//! shortlist out, each row saying why it was picked.  A real router would learn the mapping from
//! outcomes; this one reads the help screen.

use crate::tokens;

pub struct Route {
    pub name: String,
    pub score: f64,
    pub why: String,
}

/// `commands` is `(name, usage)`; `kinds` are the declaration kinds the document contains, which
/// break ties toward commands that mention what is actually there.
pub fn rank(goal: &str, commands: &[(&str, &str)], kinds: &[&str]) -> Vec<Route> {
    let wanted = tokens(goal);
    if wanted.is_empty() {
        return Vec::new();
    }
    let mut out: Vec<Route> = Vec::new();
    for (name, usage) in commands {
        if *name == "help" {
            continue;
        }
        let said = tokens(usage);
        let mut hits = Vec::new();
        for word in &wanted {
            if said.iter().any(|other| other == word) && !hits.contains(word) {
                hits.push(word.clone());
            }
        }
        if hits.is_empty() {
            continue;
        }
        let mut score = hits.len() as f64 / wanted.len() as f64;
        let mut why = format!("shares {}", hits.join(", "));
        let kind_hit: Vec<&str> = kinds
            .iter()
            .copied()
            .filter(|kind| said.iter().any(|word| word == kind))
            .collect();
        if !kind_hit.is_empty() {
            score += 0.5;
            why.push_str(&format!("; reads {}", kind_hit.join(", ")));
        }
        out.push(Route {
            name: name.to_string(),
            score,
            why,
        });
    }
    out.sort_by(|left, right| {
        right
            .score
            .partial_cmp(&left.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left.name.cmp(&right.name))
    });
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_goal_picks_the_command_that_reads_it() {
        let commands = [
            ("debt", "which facts transitively gate a conclusion"),
            ("shape", "the forms a document shares, at a depth"),
        ];
        let ranked = rank("which facts gate this conclusion", &commands, &["fact"]);
        assert_eq!(ranked[0].name, "debt", "{:?}", ranked.len());
        assert!(ranked[0].why.contains("facts"));
    }
}
