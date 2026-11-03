//! The explicit budget: a document over the limit is not checked, and the report says so.
//!
//! Invariant 5: execution stays inside a declared budget, and exceeding it produces an explicit
//! "unfinished" instead of a silent truncation.

/// How a document stands against the declared limit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BudgetState {
    pub limit: usize,
    pub declarations: usize,
    pub exceeded: bool,
}

/// Assess a document against a limit on its declarations.
pub fn assess(declarations: usize, limit: usize) -> BudgetState {
    BudgetState {
        limit,
        declarations,
        exceeded: declarations > limit,
    }
}
