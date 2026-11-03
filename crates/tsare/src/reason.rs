//! Reasoning over a whole document, re-exported from its two crates.
//!
//! The deterministic analyzers live in `tsare-analyses`, the proposers in `tsare-infer`, and the
//! six analyzers the kernel decisions call directly live in `tsare-core`.  Every public path --
//! `tsare::reason::closure`, `tsare::argue` -- keeps resolving as it always has.

pub use tsare_analyses::{
    argue, assumptions, audit, blame, cat, cost, coverage, debt, delta, derive, diff, egraph,
    impact, imply, merge, moves, patterns, shape, whatif,
};
pub use tsare_core::{ask, chain, closure, fca, induce, weigh};
pub use tsare_infer::{abstraction, generalize, hint, novel, propose};
