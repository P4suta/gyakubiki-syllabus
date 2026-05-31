//! The timetable grid: which courses land in each (day, period) cell.
//!
//! A faithful port of the former TS `buildGrid` / `countUnique`
//! (`web/src/lib/grid.ts`), kept deliberately **display-agnostic**: cells are
//! keyed by a numeric [`Day`] (0=月 … 5=土) and [`Period`], leaving the day
//! *labels* to the presentation layer. The engine resolves semester strings to
//! [`SemesterIndex`]es before calling in, so this module never touches the
//! dictionaries.

use std::collections::{BTreeMap, HashSet};

use crate::index::{CourseIndex, Day, Period, SemesterIndex};
use crate::model::Course;

/// Weekday columns always present (月‥金). Saturday (day 5) is added only when
/// the dataset contains a Saturday slot — see [`build_grid`]'s `saturday`.
const WEEKDAYS: u8 = 5;

/// A built timetable: course indices bucketed into `(day, period)` cells.
#[derive(Debug, Clone)]
pub struct Grid {
    saturday: bool,
    cells: BTreeMap<(Day, Period), Vec<CourseIndex>>,
}

impl Grid {
    /// Number of day columns: 5 (月‥金), or 6 when Saturday is present.
    #[must_use]
    pub fn day_count(&self) -> usize {
        usize::from(WEEKDAYS) + usize::from(self.saturday)
    }

    /// Whether the grid includes a Saturday column.
    #[must_use]
    pub fn has_saturday(&self) -> bool {
        self.saturday
    }

    /// Course indices in the given cell, in ascending course order; an empty
    /// slice if the cell holds nothing or lies outside the grid.
    #[must_use]
    pub fn cell(&self, day: Day, period: Period) -> &[CourseIndex] {
        self.cells.get(&(day, period)).map_or(&[], Vec::as_slice)
    }

    /// Iterate the non-empty cells in `(day, period)` order.
    pub fn cells(&self) -> impl Iterator<Item = (Day, Period, &[CourseIndex])> {
        self.cells
            .iter()
            .map(|(&(day, period), indices)| (day, period, indices.as_slice()))
    }

    /// Number of distinct courses placed anywhere in the grid.
    ///
    /// The pipeline de-duplicates courses by `cd` (one index ⇔ one `cd`), so
    /// the distinct-index count equals the distinct-`cd` count of the TS version.
    #[must_use]
    pub fn count_unique(&self) -> usize {
        self.cells
            .values()
            .flatten()
            .copied()
            .collect::<HashSet<_>>()
            .len()
    }
}

/// Build the timetable for an already-filtered set of courses.
///
/// - `courses` yields `(index, course)` pairs in ascending index order (the
///   filter output); the index is what each cell stores.
/// - `semester` is the chosen semester, or `None` for "all".
/// - `tsuunen` is the 通年 semester if the dataset has it; its courses appear
///   under every semester filter (`None` when 通年 is absent).
/// - `saturday` widens the day bound to include 土 (day 5).
pub fn build_grid<'a>(
    courses: impl IntoIterator<Item = (CourseIndex, &'a Course)>,
    semester: Option<SemesterIndex>,
    tsuunen: Option<SemesterIndex>,
    saturday: bool,
) -> Grid {
    let day_count = i32::from(WEEKDAYS) + i32::from(saturday);
    let mut cells: BTreeMap<(Day, Period), Vec<CourseIndex>> = BTreeMap::new();

    for (index, course) in courses {
        for slot in &course.slots {
            // Semester filter: keep the slot when no filter is set, it matches
            // the chosen semester, or the course is 通年 (shown every term).
            if let Some(sem) = semester {
                let slot_sem = slot.s as usize;
                let is_tsuunen = tsuunen.is_some_and(|t| t.get() == slot_sem);
                if slot_sem != sem.get() && !is_tsuunen {
                    continue;
                }
            }
            // The day must fall within the visible columns…
            if slot.d < 0 || slot.d >= day_count {
                continue;
            }
            let day = Day::new(slot.d as u8);
            // …and the period within 1限‥6限.
            let Ok(raw_period) = u8::try_from(slot.p) else {
                continue;
            };
            let Some(period) = Period::new(raw_period) else {
                continue;
            };
            let cell = cells.entry((day, period)).or_default();
            // De-duplicate by course within a cell (1限 reached via two slots,
            // e.g. {1学期,月,1} and {通年,月,1}).
            if !cell.contains(&index) {
                cell.push(index);
            }
        }
    }

    Grid { saturday, cells }
}

#[cfg(test)]
mod tests {
    //! Ported from `web/src/lib/grid.test.ts`. The test dataset has no Saturday,
    //! so `saturday = false` throughout; 通年 sits at semester index 2.

    use super::{build_grid, Grid};
    use crate::index::{CourseIndex, Day, Period, SemesterIndex};
    use crate::model::{Course, Slot};

    const TSUUNEN: Option<SemesterIndex> = Some(SemesterIndex::new(2));

    fn course(cd: &str, slots: &[(u32, i32, i32)]) -> Course {
        Course {
            cd: cd.to_owned(),
            nm: "テスト講義".to_owned(),
            sub: None,
            prof: "教員".to_owned(),
            raw: String::new(),
            slots: slots.iter().map(|&(s, d, p)| Slot { s, d, p }).collect(),
            ki: 0,
            kbn: 0,
            dept: 0,
            campus: 0,
            gaku: None,
            gakka: None,
            nen: None,
            bunrui: None,
            bunya: None,
            st: String::new(),
        }
    }

    /// Build over the whole `courses` slice, enumerating indices like the engine.
    fn grid(courses: &[Course], semester: Option<usize>) -> Grid {
        build_grid(
            courses
                .iter()
                .enumerate()
                .map(|(i, c)| (CourseIndex::new(i), c)),
            semester.map(SemesterIndex::from),
            TSUUNEN,
            false,
        )
    }

    /// Course indices in cell `(d, p)`, as plain `usize` for terse assertions.
    fn at(g: &Grid, d: u8, p: u8) -> Vec<usize> {
        g.cell(Day::new(d), Period::new(p).expect("valid period"))
            .iter()
            .map(|i| i.get())
            .collect()
    }

    #[test]
    fn empty_courses_yield_no_cells() {
        let g = grid(&[], None);
        assert_eq!(g.cells().count(), 0);
        assert_eq!(g.count_unique(), 0);
    }

    #[test]
    fn places_course_in_correct_cell() {
        let courses = [course("001", &[(0, 0, 1)])]; // 1学期, 月, 1
        let g = grid(&courses, None);
        assert_eq!(at(&g, 0, 1), [0]);
    }

    #[test]
    fn places_multiple_slots_in_multiple_cells() {
        let courses = [course("001", &[(0, 0, 1), (0, 2, 3)])]; // 月1, 水3
        let g = grid(&courses, None);
        assert_eq!(at(&g, 0, 1), [0]);
        assert_eq!(at(&g, 2, 3), [0]);
    }

    #[test]
    fn filters_by_semester() {
        let courses = [course("001", &[(1, 1, 2)])]; // 2学期, 火, 2
        let g = grid(&courses, Some(0)); // 1学期
        assert!(at(&g, 1, 2).is_empty());
    }

    #[test]
    fn tsuunen_courses_appear_in_any_semester() {
        let courses = [course("001", &[(2, 4, 5)])]; // 通年, 金, 5
        assert_eq!(at(&grid(&courses, Some(0)), 4, 5), [0]);
        assert_eq!(at(&grid(&courses, Some(1)), 4, 5), [0]);
    }

    #[test]
    fn deduplicates_same_course_in_same_cell() {
        let courses = [course("001", &[(0, 0, 1), (2, 0, 1)])]; // 月1 via 1学期 & 通年
        let g = grid(&courses, None);
        assert_eq!(at(&g, 0, 1), [0]);
    }

    #[test]
    fn courses_with_no_slots_place_nothing() {
        let courses = [course("001", &[])];
        let g = grid(&courses, None);
        assert_eq!(g.cells().count(), 0);
    }

    #[test]
    fn ignores_day_index_6_when_no_saturday() {
        let courses = [course("001", &[(0, 6, 1)])]; // 日 (index 6) is off-grid
        let g = grid(&courses, None);
        assert_eq!(g.cells().count(), 0);
    }

    #[test]
    fn ignores_period_outside_1_to_6() {
        let courses = [course("001", &[(0, 0, 7)])];
        let g = grid(&courses, None);
        assert_eq!(g.cells().count(), 0);
    }

    #[test]
    fn count_unique_is_zero_for_empty_grid() {
        assert_eq!(grid(&[], None).count_unique(), 0);
    }

    #[test]
    fn count_unique_counts_distinct_courses_across_cells() {
        let courses = [
            course("001", &[(0, 0, 1), (0, 2, 3)]),
            course("002", &[(0, 1, 2)]),
        ];
        assert_eq!(grid(&courses, None).count_unique(), 2);
    }

    #[test]
    fn count_unique_does_not_double_count() {
        let courses = [course("001", &[(0, 0, 1), (0, 1, 2), (0, 2, 3)])];
        assert_eq!(grid(&courses, None).count_unique(), 1);
    }
}
