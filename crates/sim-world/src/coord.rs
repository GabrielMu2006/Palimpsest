// Authored by Kimi Code (AI coding agent) — task CHRON-019.
//! Strongly typed coordinates inside the single 128×128 local grid.

use core::cmp::Ordering;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::grid::{LOCAL_GRID_CELL_COUNT, LOCAL_GRID_HEIGHT, LOCAL_GRID_WIDTH};

/// An in-bounds coordinate inside the 128×128 local grid.
///
/// Construction is fallible and is the only entry point: every `LocalCoord`
/// value satisfies `0 <= x < 128` and `0 <= y < 128`. There is deliberately
/// no invalid or sentinel coordinate; "no tile" is represented as
/// `Option<LocalCoord>` (CHRON-019, ADR-0012).
///
/// Ordering is row-major (`y` first, then `x`), matching [`crate::LocalGrid`]
/// iteration order. Serde encodes two `i32` integers and rejects
/// out-of-bounds values on deserialization.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct LocalCoord {
    x: u16,
    y: u16,
}

impl LocalCoord {
    /// Creates a coordinate when `x` and `y` are inside the local grid.
    #[must_use]
    pub fn new(x: i32, y: i32) -> Option<Self> {
        let x = u16::try_from(x).ok()?;
        let y = u16::try_from(y).ok()?;
        if usize::from(x) >= LOCAL_GRID_WIDTH || usize::from(y) >= LOCAL_GRID_HEIGHT {
            return None;
        }
        Some(Self { x, y })
    }

    /// Returns whether raw coordinates lie inside the local grid.
    #[must_use]
    pub fn in_bounds(x: i32, y: i32) -> bool {
        Self::new(x, y).is_some()
    }

    /// Returns the column in `[0, 128)`.
    #[must_use]
    pub fn x(self) -> i32 {
        i32::from(self.x)
    }

    /// Returns the row in `[0, 128)`.
    #[must_use]
    pub fn y(self) -> i32 {
        i32::from(self.y)
    }

    /// Returns the row-major cell index in `[0, 16384)`.
    #[must_use]
    pub fn index(self) -> usize {
        usize::from(self.y) * LOCAL_GRID_WIDTH + usize::from(self.x)
    }

    /// Creates the coordinate for a row-major cell index.
    ///
    /// Returns `None` for indices at or beyond [`LOCAL_GRID_CELL_COUNT`];
    /// together with [`LocalCoord::index`] this is a total, invertible
    /// mapping over the valid range.
    #[must_use]
    pub fn from_index(index: usize) -> Option<Self> {
        if index >= LOCAL_GRID_CELL_COUNT {
            return None;
        }
        let y = u16::try_from(index / LOCAL_GRID_WIDTH).ok()?;
        let x = u16::try_from(index % LOCAL_GRID_WIDTH).ok()?;
        Some(Self { x, y })
    }
}

impl Ord for LocalCoord {
    fn cmp(&self, other: &Self) -> Ordering {
        (self.y, self.x).cmp(&(other.y, other.x))
    }
}

impl PartialOrd for LocalCoord {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Serde wire form: two signed integers, validated on deserialization.
#[derive(Deserialize, Serialize)]
struct LocalCoordWire {
    x: i32,
    y: i32,
}

impl Serialize for LocalCoord {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        LocalCoordWire {
            x: self.x(),
            y: self.y(),
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for LocalCoord {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let wire = LocalCoordWire::deserialize(deserializer)?;
        Self::new(wire.x, wire.y).ok_or_else(|| {
            serde::de::Error::custom(format_args!(
                "local coordinate out of bounds: ({}, {})",
                wire.x, wire.y
            ))
        })
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::LocalCoord;
    use crate::grid::LOCAL_GRID_CELL_COUNT;

    #[test]
    fn new_accepts_corners_and_rejects_out_of_bounds() {
        assert!(LocalCoord::new(0, 0).is_some());
        assert!(LocalCoord::new(127, 127).is_some());
        assert!(LocalCoord::new(5, 100).is_some());
        assert_eq!(LocalCoord::new(-1, 0), None);
        assert_eq!(LocalCoord::new(0, -1), None);
        assert_eq!(LocalCoord::new(128, 0), None);
        assert_eq!(LocalCoord::new(0, 128), None);
        assert_eq!(LocalCoord::new(i32::MAX, i32::MAX), None);
        assert_eq!(LocalCoord::new(i32::MIN, 0), None);
    }

    #[test]
    fn in_bounds_agrees_with_constructor() {
        for x in [-1_i32, 0, 1, 127, 128, i32::MAX] {
            for y in [-1_i32, 0, 1, 127, 128, i32::MIN] {
                assert_eq!(LocalCoord::in_bounds(x, y), LocalCoord::new(x, y).is_some());
            }
        }
    }

    #[test]
    fn index_is_row_major_and_invertible() {
        let origin = LocalCoord::new(0, 0).expect("origin in bounds");
        let last_in_row = LocalCoord::new(127, 0).expect("in bounds");
        let next_row = LocalCoord::new(0, 1).expect("in bounds");
        let far_corner = LocalCoord::new(127, 127).expect("in bounds");
        assert_eq!(origin.index(), 0);
        assert_eq!(last_in_row.index(), 127);
        assert_eq!(next_row.index(), 128);
        assert_eq!(far_corner.index(), LOCAL_GRID_CELL_COUNT - 1);
        for index in 0..LOCAL_GRID_CELL_COUNT {
            let coord = LocalCoord::from_index(index).expect("valid index has a coordinate");
            assert_eq!(coord.index(), index);
        }
        assert_eq!(LocalCoord::from_index(LOCAL_GRID_CELL_COUNT), None);
        assert_eq!(LocalCoord::from_index(usize::MAX), None);
    }

    #[test]
    fn ordering_is_row_major() {
        let mut coords = [
            LocalCoord::new(0, 1).expect("in bounds"),
            LocalCoord::new(127, 0).expect("in bounds"),
            LocalCoord::new(1, 0).expect("in bounds"),
            LocalCoord::new(0, 0).expect("in bounds"),
        ];
        coords.sort();
        let order: Vec<(i32, i32)> = coords.iter().map(|coord| (coord.x(), coord.y())).collect();
        assert_eq!(order, vec![(0, 0), (1, 0), (127, 0), (0, 1)]);
    }

    #[test]
    fn hash_is_consistent_with_equality() {
        let mut set = HashSet::new();
        set.insert(LocalCoord::new(3, 4).expect("in bounds"));
        set.insert(LocalCoord::new(3, 4).expect("in bounds"));
        set.insert(LocalCoord::new(4, 3).expect("in bounds"));
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn serde_round_trips_and_rejects_out_of_bounds() {
        let coord = LocalCoord::new(12, 34).expect("in bounds");
        let encoded = serde_json::to_string(&coord).expect("serialize coord");
        assert_eq!(encoded, "{\"x\":12,\"y\":34}");
        assert_eq!(
            serde_json::from_str::<LocalCoord>(&encoded).expect("deserialize coord"),
            coord
        );
        assert!(serde_json::from_str::<LocalCoord>("{\"x\":128,\"y\":0}").is_err());
        assert!(serde_json::from_str::<LocalCoord>("{\"x\":-1,\"y\":0}").is_err());
        assert!(serde_json::from_str::<LocalCoord>("{\"x\":0}").is_err());
    }

    #[test]
    fn no_tile_is_an_option_not_a_sentinel() {
        let mut position: Option<LocalCoord> = None;
        assert!(position.is_none());
        position = LocalCoord::new(1, 1);
        assert!(position.is_some());
    }
}
