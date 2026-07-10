//! Parse KULAS「シラバス参照」detail pages into [`SanshoDetail`] — the content
//! (授業計画・成績評価・オフィスアワー…) the findPage API omits. Pure (HTML in,
//! data out), so it is testable against committed fixtures; fetching lives in
//! [`crate::fetch_details`].

mod classify;
mod enrich;
mod model;
mod parse;

pub use enrich::enrich;
pub use model::{Delivery, Eval, EvalRow, Labelled, OfficeHour, PlanItem, SanshoDetail};
pub use parse::parse_sansho_html;
