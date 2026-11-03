//! What the engine proposes and the check may refuse.
//!
//! The candidates group: abstraction, generalize, hint, novel, propose.  Each output enters the
//! same three-state ledger as anything else and may be refused by the kernel -- proposal is a
//! claim, not a decision.

pub mod abstraction;
pub mod generalize;
pub mod hint;
pub mod novel;
pub mod propose;
