//! Timetable-conflict detection over a plan (a set of registered courses).
//!
//! A conflict is two or more registered courses meeting in the same
//! `(day, period)` cell. The heavy lifting is [`crate::grid::build_grid`], reused
//! per semester by [`crate::Engine::plan_summary`] — this module only reads the
//! built grid, so 通年 propagation and per-course de-duplication stay in one
//! place.

use crate::grid::Grid;
use crate::index::{CourseIndex, Day, Period};

/// Two or more registered courses colliding in one timetable cell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Conflict {
    pub day: Day,
    pub period: Period,
    /// The colliding courses, ascending, always 2 or more.
    pub courses: Vec<CourseIndex>,
}

/// Enumerate the multi-course cells of a single built grid as conflicts.
pub fn conflicts_in_grid(grid: &Grid) -> impl Iterator<Item = Conflict> + '_ {
    grid.cells()
        .filter(|(_, _, courses)| courses.len() >= 2)
        .map(|(day, period, courses)| Conflict {
            day,
            period,
            courses: courses.to_vec(),
        })
}

#[cfg(test)]
mod tests {
    use super::conflicts_in_grid;
    use crate::grid::{GridSlot, build_grid};
    use crate::index::{CourseIndex, Day, Period};
    use crate::model::Slot;

    fn timetable(slots: &[(u32, i32, i32)]) -> Vec<GridSlot> {
        slots
            .iter()
            .filter_map(|&(s, d, p)| GridSlot::from_wire(&Slot { s, d, p }))
            .collect()
    }

    /// Build a grid over courses (no semester filter, no 通年).
    fn grid(courses: &[&[(u32, i32, i32)]]) -> crate::grid::Grid {
        let tts: Vec<Vec<GridSlot>> = courses.iter().copied().map(timetable).collect();
        build_grid(
            tts.iter()
                .enumerate()
                .map(|(i, t)| (CourseIndex::new(i), t.as_slice())),
            None,
            None,
            false,
        )
    }

    #[test]
    fn no_conflict_when_cells_are_distinct() {
        let g = grid(&[&[(0, 0, 1)], &[(0, 1, 2)]]);
        assert_eq!(conflicts_in_grid(&g).count(), 0);
    }

    #[test]
    fn detects_two_courses_in_one_cell() {
        let g = grid(&[&[(0, 0, 1)], &[(0, 0, 1)]]);
        let cs: Vec<_> = conflicts_in_grid(&g).collect();
        assert_eq!(cs.len(), 1);
        assert_eq!(cs[0].day, Day::new(0));
        assert_eq!(cs[0].period, Period::new(1).unwrap());
        assert_eq!(cs[0].courses, [CourseIndex::new(0), CourseIndex::new(1)]);
    }

    #[test]
    fn three_courses_collide_as_one_conflict() {
        let g = grid(&[&[(0, 2, 3)], &[(0, 2, 3)], &[(0, 2, 3)]]);
        let cs: Vec<_> = conflicts_in_grid(&g).collect();
        assert_eq!(cs.len(), 1);
        assert_eq!(cs[0].courses.len(), 3);
    }

    #[test]
    fn a_course_meeting_itself_twice_in_a_cell_is_not_a_conflict() {
        // Same course via two slots into the same cell — build_grid de-dupes it,
        // so it never looks like a collision with itself.
        let g = grid(&[&[(0, 0, 1), (0, 0, 1)]]);
        assert_eq!(conflicts_in_grid(&g).count(), 0);
    }
}
