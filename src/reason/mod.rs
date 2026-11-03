//! Reasoning over a whole document: closure, argument, structure, and what it owes.
//!
//! The analyzers are grouped by the question they answer, so a new one has a home:
//!
//! - proof: the derivation and its defeat -- argue, assumptions, chain, closure, derive, egraph,
//!   imply, weigh;
//! - structure: the shapes declarations take -- cat, fca, moves, patterns, shape;
//! - candidates: what the engine proposes and the check may refuse -- abstraction, ask, generalize,
//!   hint, induce, novel, propose;
//! - evidence: what a document owes and the store remembers -- audit, blame, cost, coverage, debt;
//! - change: what a withdrawal, a merge or an edit does -- delta, diff, impact, merge, whatif.
//!
//! The groups are a reading order, not a visibility rule: every analyzer sees the same `check`
//! types, and none of them depends on another.
pub mod abstraction;
pub mod argue;
pub mod ask;
pub mod assumptions;
pub mod audit;
pub mod blame;
pub mod cat;
pub mod chain;
pub mod closure;
pub mod cost;
pub mod coverage;
pub mod debt;
pub mod delta;
pub mod derive;
pub mod diff;
pub mod egraph;
pub mod fca;
pub mod generalize;
pub mod hint;
pub mod impact;
pub mod imply;
pub mod induce;
pub mod merge;
pub mod moves;
pub mod novel;
pub mod patterns;
pub mod propose;
pub mod shape;
pub mod weigh;
pub mod whatif;
