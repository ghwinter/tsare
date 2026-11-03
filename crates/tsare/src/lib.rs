//! The library, grouped by what each part does rather than by alphabet.
//!
//! `lang` reads text into declarations, `check` decides whether they are well formed, `reason`
//! computes over the whole document, `emit` turns it into files and prose, `io` owns the few
//! places state lives, and `out` carries a rendered line to a sink. The modules are re-exported at
//! the root so `tsare::check::check` and the rest keep working: moving a file is a decision about
//! reading, not about the public path.
//!
//! Since the workspace split, the groups are crates and this package is their facade:
//! `check` = `tsare-core` (the trusted base), `lang` = `tsare-lang`, `io` = `tsare-state`,
//! `reason` = `tsare-analyses` + `tsare-infer` (plus six kernel analyzers re-exported from
//! `tsare-core`), `emit` = `tsare-emit`.  Everything here is re-export; the only resident code is
//! the CLI, the MCP adapter, and the transports in `out`.

pub mod cli;
pub mod reason;
pub mod emit {
    //! The emit crate, re-exported whole: `tsare::emit` keeps the shape it had as one crate.
    pub use tsare_emit::*;
}
pub mod io {
    //! The state crate, re-exported whole: `tsare::io` keeps the shape it had as one crate.
    pub use tsare_state::*;
}
pub use tsare_lang as lang;
pub mod mcp;
pub mod out;

pub use tsare_util as util;

pub mod check {
    //! The trusted base, re-exported whole: `tsare::check` keeps the shape it had as one crate.
    pub use tsare_core::*;
}
pub use check::render;
pub use check::{arrow, budget, decide, epistemic, fix, kernel, provenance, status};
pub use emit::{build, graph, growth, mapping, outline, views};
pub use io::{apply, cache, cases, session, trace, version};
pub use lang::format;
pub use lang::{contracts, explain, extract, hash, json, paths, program, specs, sugar, syntax};
pub use reason::{
    abstraction, argue, ask, assumptions, audit, blame, cat, chain, closure, cost, coverage, debt,
    delta, derive, diff, egraph, fca, generalize, hint, impact, imply, induce, merge, moves, novel,
    patterns, propose, shape, weigh, whatif,
};
