//! Strongly-typed indices for the domain layer.
//!
//! The wire DTOs in [`crate::model`] keep raw integer indices, faithful to the
//! JSON. But everywhere a value *means* "a position into a particular table" we
//! wrap it, so the type system rejects mixing a course position with a
//! dictionary position — or a department with a campus. The wrappers are
//! zero-cost; the `usize`/`u32` conversions live only at the edges (JSON in,
//! WASM out).

/// A course's position in the dataset — the unit the filter/grid boundary
/// speaks. The whole "indices-out" design hands these around and resolves them
/// against the course `Vec` only at the very end.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CourseIndex(usize);

impl CourseIndex {
    /// Wrap a raw position.
    #[must_use]
    pub const fn new(raw: usize) -> Self {
        Self(raw)
    }

    /// The underlying position, for slicing into the course `Vec`.
    #[must_use]
    pub const fn get(self) -> usize {
        self.0
    }
}

impl From<usize> for CourseIndex {
    fn from(raw: usize) -> Self {
        Self(raw)
    }
}

/// Define a dictionary-position newtype: a typed index into one of the
/// [`Dictionaries`](crate::model::Dictionaries) tables. Distinct types keep a
/// semester index from being looked up in the department map, and vice versa.
macro_rules! dict_index {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub struct $name(usize);

        impl $name {
            /// Wrap a raw dictionary position.
            #[must_use]
            pub const fn new(raw: usize) -> Self {
                Self(raw)
            }

            /// The underlying dictionary position.
            #[must_use]
            pub const fn get(self) -> usize {
                self.0
            }
        }

        impl From<usize> for $name {
            fn from(raw: usize) -> Self {
                Self(raw)
            }
        }
    };
}

dict_index! {
    /// Position into [`Dictionaries::semesters`](crate::model::Dictionaries::semesters).
    SemesterIndex
}
dict_index! {
    /// Position into [`Dictionaries::departments`](crate::model::Dictionaries::departments).
    DepartmentIndex
}
dict_index! {
    /// Position into [`Dictionaries::campuses`](crate::model::Dictionaries::campuses).
    CampusIndex
}

/// A weekday column on the timetable (0 = 月 … 5 = 土). The label lives in the
/// presentation layer; the grid only ever speaks this numeric column.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Day(u8);

impl Day {
    /// Wrap a raw column index. The valid upper bound depends on whether the
    /// dataset has a Saturday, so it is enforced by the caller, not here.
    #[must_use]
    pub const fn new(column: u8) -> Self {
        Self(column)
    }

    /// The underlying column index.
    #[must_use]
    pub const fn get(self) -> u8 {
        self.0
    }
}

/// A teaching period (1限 … 6限). Construction enforces the range invariant, so
/// a `Period` value is always one the grid can display.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Period(u8);

impl Period {
    /// The periods the grid lays out (1限‥6限).
    pub const RANGE: std::ops::RangeInclusive<u8> = 1..=6;

    /// Build a period, returning `None` for anything outside 1限‥6限.
    #[must_use]
    pub fn new(period: u8) -> Option<Self> {
        Self::RANGE.contains(&period).then_some(Self(period))
    }

    /// The underlying period number (1‥6).
    #[must_use]
    pub const fn get(self) -> u8 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::Period;

    #[test]
    fn period_accepts_the_visible_range() {
        assert!(Period::new(1).is_some());
        assert!(Period::new(6).is_some());
    }

    #[test]
    fn period_rejects_out_of_range() {
        assert!(Period::new(0).is_none());
        assert!(Period::new(7).is_none());
    }
}
