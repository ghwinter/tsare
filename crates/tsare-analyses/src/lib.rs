//! Reasoning over a whole document: the deterministic analyzers.
//!
//! Grouped by the question they answer: proof (argue, assumptions, derive, egraph, imply),
//! structure (cat, moves, patterns, shape), evidence (audit, blame, cost, coverage, debt), and
//! change (delta, diff, impact, merge, whatif).  The six analyzers the kernel decisions call
//! directly (ask, chain, closure, fca, induce, weigh) live below, in `tsare-core`; the five
//! proposers (abstraction, generalize, hint, novel, propose) live above, in `tsare-infer`.

pub mod argue;
pub mod assumptions;
pub mod audit;
pub mod blame;
pub mod cat;
pub mod cost;
pub mod coverage;
pub mod debt;
pub mod delta;
pub mod derive;
pub mod diff;
pub mod egraph;
pub mod impact;
pub mod imply;
pub mod merge;
pub mod moves;
pub mod patterns;
pub mod shape;
pub mod whatif;
