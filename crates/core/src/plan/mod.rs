//! Timetable planner domain: conflict detection and credit tallies over a plan
//! (a set of registered courses), plus the [`PlanSummary`] the UI renders.

pub mod conflict;
pub mod credits;

pub use conflict::{Conflict, conflicts_in_grid};
pub use credits::{CategoryTally, CreditSummary, parse_unit, summarize_credits};

/// The full summary of a plan: every timetable collision and the credit tallies.
#[derive(Debug, Clone, PartialEq)]
pub struct PlanSummary {
    pub conflicts: Vec<Conflict>,
    pub credits: CreditSummary,
}
