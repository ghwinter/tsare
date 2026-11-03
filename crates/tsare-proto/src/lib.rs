//! Prototypes.  Nothing here is part of the trusted base, and nothing here is allowed to decide
//! whether a document is well formed -- that is `check`.  These are the four components that a
//! coprocessor needs above the checker, each reduced to the smallest thing that does the job so
//! the shape can be argued about before it is made robust:
//!
//! | module  | the question it answers                                   |
//! | ------- | --------------------------------------------------------- |
//! | `route` | given a goal, which command reads this document for it     |
//! | `sat`   | can every rule, fact and attack hold at once               |
//! | `cause` | what changes when the value is set by intervention         |
//! | `index` | which declarations are near each other in their own words  |
//!
//! Each is deliberately crude: the router scores words, the solver is plain DPLL with no theory
//! reasoning, the intervention is an edge surgery on the declared graph, and the index is term
//! frequency with cosine.  A prototype that admits what it does not do is worth more than one
//! that hides it.

pub mod cause;
pub mod index;
pub mod route;
pub mod sat;

/// The words a text contributes to a bag: lowercased, split on anything that is not a letter or a
/// digit, short tokens dropped.  Chinese has no spaces, so a run of Han characters is one token --
/// crude, and the index says so.
pub fn tokens(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    for character in text.chars() {
        if character.is_alphanumeric() {
            current.extend(character.to_lowercase());
        } else if !current.is_empty() {
            out.push(std::mem::take(&mut current));
        }
    }
    if !current.is_empty() {
        out.push(current);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens_are_lowercased_words() {
        assert_eq!(
            tokens("The list scrolls at 60 fps"),
            vec!["the", "list", "scrolls", "at", "60", "fps"]
        );
    }
}
