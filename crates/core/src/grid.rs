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
use crate::model::Slot;

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

/// A timetable slot validated into the grid's typed [`Day`] / [`Period`].
///
/// The wire [`Slot`] carries raw `i32` day/period; [`GridSlot::from_wire`]
/// range-checks them **once** (when the engine is built) so [`build_grid`] —
/// which runs on every filter change — only ever sees displayable slots and
/// never re-validates.
#[derive(Debug, Clone, Copy)]
pub struct GridSlot {
    /// Dictionary index of the slot's semester (drives the semester filter).
    semester: usize,
    day: Day,
    period: Period,
}

impl GridSlot {
    /// Validate a wire [`Slot`], or `None` if it can never appear on the grid:
    /// a day outside the columns (only 月..土 = 0..=5 are shown; 日 and anything
    /// out of range fall away) or a period outside 1限..6限.
    #[must_use]
    pub fn from_wire(slot: &Slot) -> Option<Self> {
        // 0..=5 = 月..土; 日 (6) is never a column, negatives fail `try_from`.
        let day = u8::try_from(slot.d)
            .ok()
            .filter(|&d| d <= WEEKDAYS)
            .map(Day::new)?;
        let period = u8::try_from(slot.p).ok().and_then(Period::new)?;
        Some(Self {
            semester: slot.s as usize,
            day,
            period,
        })
    }

    /// Whether this slot meets on Saturday (drives the extra grid column).
    #[must_use]
    pub fn is_saturday(self) -> bool {
        self.day.get() == WEEKDAYS
    }
}

/// Build the timetable for an already-filtered set of courses.
///
/// - `timetables` yields `(index, slots)` in ascending course-index order (the
///   filter output); `slots` are the course's pre-validated [`GridSlot`]s.
/// - `semester` is the chosen semester, or `None` for "all".
/// - `tsuunen` is the 通年 semester if the dataset has it; its courses appear
///   under every semester filter (`None` when 通年 is absent).
/// - `saturday` records whether the grid shows the 土 column.
pub fn build_grid<'a>(
    timetables: impl IntoIterator<Item = (CourseIndex, &'a [GridSlot])>,
    semester: Option<SemesterIndex>,
    tsuunen: Option<SemesterIndex>,
    saturday: bool,
) -> Grid {
    let mut cells: BTreeMap<(Day, Period), Vec<CourseIndex>> = BTreeMap::new();

    for (index, slots) in timetables {
        for slot in slots {
            // Semester filter: keep the slot when no filter is set, it matches
            // the chosen semester, or the course is 通年 (shown every term).
            if let Some(sem) = semester {
                let is_tsuunen = tsuunen.is_some_and(|t| t.get() == slot.semester);
                if slot.semester != sem.get() && !is_tsuunen {
                    continue;
                }
            }
            // De-duplicate by course within a cell (1限 reached via two slots,
            // e.g. {1学期,月,1} and {通年,月,1}).
            let cell = cells.entry((slot.day, slot.period)).or_default();
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

    use super::{build_grid, Grid, GridSlot};
    use crate::index::{CourseIndex, Day, Period, SemesterIndex};
    use crate::model::Slot;

    const TSUUNEN: Option<SemesterIndex> = Some(SemesterIndex::new(2));

    /// One course's validated timetable from `(semester, day, period)` wire slots.
    fn timetable(slots: &[(u32, i32, i32)]) -> Vec<GridSlot> {
        slots
            .iter()
            .filter_map(|&(s, d, p)| GridSlot::from_wire(&Slot { s, d, p }))
            .collect()
    }

    /// Build a grid over courses given as per-course wire slot lists, enumerating
    /// indices like the engine.
    fn grid(courses: &[&[(u32, i32, i32)]], semester: Option<usize>) -> Grid {
        let timetables: Vec<Vec<GridSlot>> = courses.iter().copied().map(timetable).collect();
        build_grid(
            timetables
                .iter()
                .enumerate()
                .map(|(i, t)| (CourseIndex::new(i), t.as_slice())),
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
        let g = grid(&[&[(0, 0, 1)]], None); // 1学期, 月, 1
        assert_eq!(at(&g, 0, 1), [0]);
    }

    #[test]
    fn places_multiple_slots_in_multiple_cells() {
        let g = grid(&[&[(0, 0, 1), (0, 2, 3)]], None); // 月1, 水3
        assert_eq!(at(&g, 0, 1), [0]);
        assert_eq!(at(&g, 2, 3), [0]);
    }

    #[test]
    fn filters_by_semester() {
        let g = grid(&[&[(1, 1, 2)]], Some(0)); // course in 2学期, filter 1学期
        assert!(at(&g, 1, 2).is_empty());
    }

    #[test]
    fn tsuunen_courses_appear_in_any_semester() {
        let courses: &[&[(u32, i32, i32)]] = &[&[(2, 4, 5)]]; // 通年, 金, 5
        assert_eq!(at(&grid(courses, Some(0)), 4, 5), [0]);
        assert_eq!(at(&grid(courses, Some(1)), 4, 5), [0]);
    }

    #[test]
    fn deduplicates_same_course_in_same_cell() {
        let g = grid(&[&[(0, 0, 1), (2, 0, 1)]], None); // 月1 via 1学期 & 通年
        assert_eq!(at(&g, 0, 1), [0]);
    }

    #[test]
    fn courses_with_no_slots_place_nothing() {
        let g = grid(&[&[]], None);
        assert_eq!(g.cells().count(), 0);
    }

    #[test]
    fn count_unique_is_zero_for_empty_grid() {
        assert_eq!(grid(&[], None).count_unique(), 0);
    }

    #[test]
    fn count_unique_counts_distinct_courses_across_cells() {
        let g = grid(&[&[(0, 0, 1), (0, 2, 3)], &[(0, 1, 2)]], None);
        assert_eq!(g.count_unique(), 2);
    }

    #[test]
    fn count_unique_does_not_double_count() {
        let g = grid(&[&[(0, 0, 1), (0, 1, 2), (0, 2, 3)]], None);
        assert_eq!(g.count_unique(), 1);
    }

    // GridSlot::from_wire — the range validation that used to live in build_grid.

    #[test]
    fn from_wire_drops_sunday_and_beyond() {
        assert!(GridSlot::from_wire(&Slot { s: 0, d: 6, p: 1 }).is_none()); // 日
        assert!(GridSlot::from_wire(&Slot { s: 0, d: 99, p: 1 }).is_none());
    }

    #[test]
    fn from_wire_drops_negative_day() {
        assert!(GridSlot::from_wire(&Slot { s: 0, d: -1, p: 1 }).is_none());
    }

    #[test]
    fn from_wire_keeps_saturday() {
        let slot = GridSlot::from_wire(&Slot { s: 0, d: 5, p: 1 }).expect("土 is a column");
        assert!(slot.is_saturday());
    }

    #[test]
    fn from_wire_drops_period_outside_1_to_6() {
        assert!(GridSlot::from_wire(&Slot { s: 0, d: 0, p: 0 }).is_none());
        assert!(GridSlot::from_wire(&Slot { s: 0, d: 0, p: 7 }).is_none());
    }
}
