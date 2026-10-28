//! The library, grouped by what each part does rather than by alphabet.
//!
//! `lang` reads text into declarations, `check` decides whether they are well formed, `reason`
//! computes over the whole document, `emit` turns it into files and prose, and `io` owns the few
//! places state lives. The modules are re-exported at the root so `tsare::check::check` and the
//! rest keep working: moving a file is a decision about reading, not about the public path.

pub mod check;
pub mod cli;
pub mod emit;
pub mod io;
pub mod lang;
pub mod mcp;
pub mod reason;

pub use check::{fix, kernel};
pub use emit::{build, graph, mapping, outline};
pub use io::{apply, cache, cases, session};
pub use lang::{explain, hash, json, paths, program, render, specs, sugar, syntax};
pub use reason::{
    abstraction, argue, ask, assumptions, audit, cat, closure, cost, coverage, debt, delta, derive,
    egraph, fca, generalize, hint, impact, imply, induce, moves, novel, patterns, propose, shape,
    weigh, whatif,
};
