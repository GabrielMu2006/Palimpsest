// Authored by Kimi Code (AI coding agent) — task CHRON-019.
//! Boundary-safe 128×128 `LocalGrid` and the single-local `WorldGrid`.

use core::fmt::{self, Display, Formatter};

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::coord::LocalCoord;

/// Number of columns in the single local grid.
pub const LOCAL_GRID_WIDTH: usize = 128;
/// Number of rows in the single local grid.
pub const LOCAL_GRID_HEIGHT: usize = 128;
/// Total number of cells in the local grid.
pub const LOCAL_GRID_CELL_COUNT: usize = LOCAL_GRID_WIDTH * LOCAL_GRID_HEIGHT;

/// Errors from fallible grid operations.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GridError {
    /// Raw coordinates fell outside the 128×128 local grid.
    OutOfBounds {
        /// The rejected raw `(x, y)` coordinates.
        coord: (i32, i32),
    },
    /// A cell collection had the wrong length for a 128×128 grid.
    InvalidCellCount {
        /// Required cell count (always 16,384).
        expected: usize,
        /// Cell count that was provided.
        got: usize,
    },
}

impl Display for GridError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::OutOfBounds { coord } => {
                write!(
                    formatter,
                    "local grid coordinates out of bounds: ({}, {})",
                    coord.0, coord.1
                )
            }
            Self::InvalidCellCount { expected, got } => {
                write!(
                    formatter,
                    "local grid needs exactly {expected} cells, got {got}"
                )
            }
        }
    }
}

impl std::error::Error for GridError {}

/// A boundary-safe 128×128 row-major cell container.
///
/// The grid is generic and storage-agnostic: terrain, walkability, and other
/// per-cell meaning arrive in later tasks (CHRON-020+). Invariants:
///
/// - exactly [`LOCAL_GRID_CELL_COUNT`] cells, row-major (`index = y * 128 + x`);
/// - every accessor validates raw coordinates and never panics on invalid input;
/// - iteration order is deterministic row-major.
#[derive(Clone, Debug)]
pub struct LocalGrid<T> {
    cells: Vec<T>,
}

impl<T> LocalGrid<T> {
    /// Creates a grid with every cell set to a clone of `value`.
    #[must_use]
    pub fn filled_with(value: T) -> Self
    where
        T: Clone,
    {
        Self {
            cells: vec![value; LOCAL_GRID_CELL_COUNT],
        }
    }

    /// Creates a grid from exactly [`LOCAL_GRID_CELL_COUNT`] cells in
    /// row-major order.
    ///
    /// # Errors
    ///
    /// Returns [`GridError::InvalidCellCount`] when `cells` does not contain
    /// exactly 16,384 entries.
    pub fn from_cells(cells: Vec<T>) -> Result<Self, GridError> {
        let expected = LOCAL_GRID_CELL_COUNT;
        let got = cells.len();
        if got == expected {
            Ok(Self { cells })
        } else {
            Err(GridError::InvalidCellCount { expected, got })
        }
    }

    /// Returns the number of cells; always [`LOCAL_GRID_CELL_COUNT`].
    #[must_use]
    pub fn len(&self) -> usize {
        self.cells.len()
    }

    /// Returns whether the grid has no cells; always `false` for a
    /// well-formed grid.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }

    /// Returns whether raw coordinates lie inside the grid.
    #[must_use]
    pub fn contains(&self, x: i32, y: i32) -> bool {
        LocalCoord::in_bounds(x, y)
    }

    /// Returns the cell at raw coordinates, or `None` when out of bounds.
    #[must_use]
    pub fn get(&self, x: i32, y: i32) -> Option<&T> {
        let coord = LocalCoord::new(x, y)?;
        self.cells.get(coord.index())
    }

    /// Returns the cell mutably at raw coordinates, or `None` when out of
    /// bounds.
    pub fn get_mut(&mut self, x: i32, y: i32) -> Option<&mut T> {
        let coord = LocalCoord::new(x, y)?;
        self.cells.get_mut(coord.index())
    }

    /// Returns the cell at a row-major index, or `None` when out of range.
    #[must_use]
    pub fn get_index(&self, index: usize) -> Option<&T> {
        self.cells.get(index)
    }

    /// Replaces the cell at raw coordinates.
    ///
    /// # Errors
    ///
    /// Returns [`GridError::OutOfBounds`] when `(x, y)` is outside the grid.
    pub fn set(&mut self, x: i32, y: i32, value: T) -> Result<(), GridError> {
        let coord = LocalCoord::new(x, y).ok_or(GridError::OutOfBounds { coord: (x, y) })?;
        self.cells[coord.index()] = value;
        Ok(())
    }

    /// Swaps the cell at raw coordinates, returning the previous value.
    ///
    /// # Errors
    ///
    /// Returns [`GridError::OutOfBounds`] when `(x, y)` is outside the grid.
    pub fn swap(&mut self, x: i32, y: i32, value: T) -> Result<T, GridError> {
        let coord = LocalCoord::new(x, y).ok_or(GridError::OutOfBounds { coord: (x, y) })?;
        Ok(core::mem::replace(&mut self.cells[coord.index()], value))
    }

    /// Iterates over every cell in deterministic row-major order.
    #[must_use]
    pub fn iter(&self) -> impl ExactSizeIterator<Item = &T> + '_ {
        self.cells.iter()
    }

    /// Iterates over every coordinate in deterministic row-major order.
    ///
    /// Covers each of the 16,384 cells exactly once; the total, invertible
    /// index mapping makes this infallible, so no coordinate can be skipped.
    pub fn coords(&self) -> impl Iterator<Item = LocalCoord> + '_ {
        (0..LOCAL_GRID_CELL_COUNT).filter_map(LocalCoord::from_index)
    }
}

impl<'a, T> IntoIterator for &'a LocalGrid<T> {
    type Item = &'a T;
    type IntoIter = core::slice::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.cells.iter()
    }
}

impl<T: Serialize> Serialize for LocalGrid<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.cells.serialize(serializer)
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for LocalGrid<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let cells = Vec::<T>::deserialize(deserializer)?;
        Self::from_cells(cells).map_err(serde::de::Error::custom)
    }
}

/// The Phase 1 world: exactly one authoritative 128×128 local grid.
///
/// Region, multi-local, chunk, and LOD addressing are deliberately absent
/// (ADR-0012); there are no selector arguments because there is nothing to
/// select.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct WorldGrid<T> {
    local: LocalGrid<T>,
}

impl<T> WorldGrid<T> {
    /// Creates a world grid wrapping the single local grid.
    #[must_use]
    pub fn new(local: LocalGrid<T>) -> Self {
        Self { local }
    }

    /// Returns the single local grid.
    #[must_use]
    pub fn local(&self) -> &LocalGrid<T> {
        &self.local
    }

    /// Returns the single local grid mutably.
    pub fn local_mut(&mut self) -> &mut LocalGrid<T> {
        &mut self.local
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::{
        GridError, LOCAL_GRID_CELL_COUNT, LOCAL_GRID_HEIGHT, LOCAL_GRID_WIDTH, LocalGrid, WorldGrid,
    };
    use crate::coord::LocalCoord;

    fn index_grid() -> LocalGrid<u32> {
        let cells = (0..LOCAL_GRID_CELL_COUNT)
            .map(|index| u32::try_from(index).expect("cell index fits u32"))
            .collect();
        LocalGrid::from_cells(cells).expect("exact cell count")
    }

    fn content_checksum(grid: &LocalGrid<u8>) -> u64 {
        grid.iter().enumerate().fold(0_u64, |acc, (index, cell)| {
            let position = u64::try_from(index).expect("cell index fits u64");
            acc.wrapping_add(u64::from(*cell).wrapping_mul(position.wrapping_add(1)))
        })
    }

    #[test]
    fn constants_fix_the_128_square() {
        assert_eq!(LOCAL_GRID_WIDTH, 128);
        assert_eq!(LOCAL_GRID_HEIGHT, 128);
        assert_eq!(LOCAL_GRID_CELL_COUNT, 16_384);
    }

    #[test]
    fn filled_grid_has_exact_cell_count() {
        let grid = LocalGrid::filled_with(7_u8);
        assert_eq!(grid.len(), LOCAL_GRID_CELL_COUNT);
        assert!(!grid.is_empty());
        assert!(grid.iter().all(|cell| *cell == 7));
    }

    #[test]
    fn from_cells_validates_exact_length() {
        let short = vec![0_u8; LOCAL_GRID_CELL_COUNT - 1];
        assert_eq!(
            LocalGrid::from_cells(short).unwrap_err(),
            GridError::InvalidCellCount {
                expected: LOCAL_GRID_CELL_COUNT,
                got: LOCAL_GRID_CELL_COUNT - 1,
            }
        );
        let long = vec![0_u8; LOCAL_GRID_CELL_COUNT + 1];
        assert_eq!(
            LocalGrid::from_cells(long).unwrap_err(),
            GridError::InvalidCellCount {
                expected: LOCAL_GRID_CELL_COUNT,
                got: LOCAL_GRID_CELL_COUNT + 1,
            }
        );
        let empty: Vec<u8> = Vec::new();
        assert_eq!(
            LocalGrid::from_cells(empty).unwrap_err(),
            GridError::InvalidCellCount {
                expected: LOCAL_GRID_CELL_COUNT,
                got: 0,
            }
        );
        let exact = vec![0_u8; LOCAL_GRID_CELL_COUNT];
        assert!(LocalGrid::from_cells(exact).is_ok());
    }

    #[test]
    fn accessors_are_boundary_safe_and_never_panic() {
        let mut grid = index_grid();
        assert_eq!(grid.get(0, 0), Some(&0_u32));
        assert_eq!(grid.get(127, 0), Some(&127_u32));
        assert_eq!(grid.get(0, 127), Some(&16_256_u32));
        assert_eq!(grid.get(127, 127), Some(&16_383_u32));
        assert_eq!(grid.get(-1, 0), None);
        assert_eq!(grid.get(0, -1), None);
        assert_eq!(grid.get(128, 0), None);
        assert_eq!(grid.get(0, 128), None);
        assert_eq!(grid.get_index(0), Some(&0_u32));
        assert_eq!(grid.get_index(16_383), Some(&16_383_u32));
        assert_eq!(grid.get_index(16_384), None);
        assert!(grid.get_mut(-1, -1).is_none());

        assert_eq!(
            grid.set(128, 0, 1),
            Err(GridError::OutOfBounds { coord: (128, 0) })
        );
        assert_eq!(
            grid.set(-5, 9, 1),
            Err(GridError::OutOfBounds { coord: (-5, 9) })
        );
        grid.set(10, 20, 999).expect("in bounds");
        assert_eq!(grid.get(10, 20), Some(&999_u32));

        assert_eq!(grid.swap(10, 20, 5), Ok(999_u32));
        assert_eq!(grid.get(10, 20), Some(&5_u32));
        assert_eq!(
            grid.swap(0, 200, 1),
            Err(GridError::OutOfBounds { coord: (0, 200) })
        );

        *grid.get_mut(3, 3).expect("in bounds") = 42;
        assert_eq!(grid.get(3, 3), Some(&42_u32));
    }

    #[test]
    fn contains_covers_corners_and_outside() {
        let grid = index_grid();
        for (x, y) in [(0, 0), (127, 0), (0, 127), (127, 127)] {
            assert!(grid.contains(x, y));
        }
        for (x, y) in [(-1, 0), (0, -1), (128, 0), (0, 128), (128, 128), (-1, 127)] {
            assert!(!grid.contains(x, y));
        }
    }

    #[test]
    fn iteration_is_deterministic_row_major() {
        let grid = index_grid();
        let collected: Vec<u32> = grid.iter().copied().collect();
        let expected: Vec<u32> = (0..LOCAL_GRID_CELL_COUNT)
            .map(|index| u32::try_from(index).expect("cell index fits u32"))
            .collect();
        assert_eq!(collected, expected);

        let coords: Vec<LocalCoord> = grid.coords().collect();
        assert_eq!(coords.len(), LOCAL_GRID_CELL_COUNT);
        assert_eq!(coords.first().copied(), LocalCoord::new(0, 0));
        assert_eq!(coords.get(1).copied(), LocalCoord::new(1, 0));
        assert_eq!(coords.get(128).copied(), LocalCoord::new(0, 1));
        assert_eq!(coords.last().copied(), LocalCoord::new(127, 127));

        let unique: HashSet<LocalCoord> = coords.iter().copied().collect();
        assert_eq!(unique.len(), LOCAL_GRID_CELL_COUNT);

        for (position, coord) in coords.iter().enumerate() {
            assert_eq!(coord.index(), position);
            assert_eq!(grid.get(coord.x(), coord.y()), collected.get(position));
        }

        let twin = index_grid();
        assert!(grid.iter().eq(twin.iter()));
        assert!(grid.coords().eq(twin.coords()));
    }

    #[test]
    fn serde_round_trip_preserves_cells_and_checksum() {
        let cells = (0..LOCAL_GRID_CELL_COUNT)
            .map(|index| u8::try_from(index % 251).expect("pattern fits u8"))
            .collect();
        let grid = LocalGrid::from_cells(cells).expect("exact cell count");
        let encoded = serde_json::to_string(&grid).expect("serialize grid");
        let restored: LocalGrid<u8> = serde_json::from_str(&encoded).expect("deserialize grid");
        assert!(grid.iter().eq(restored.iter()));
        assert_eq!(content_checksum(&grid), content_checksum(&restored));
    }

    #[test]
    fn serde_rejects_wrong_cell_count() {
        assert!(serde_json::from_str::<LocalGrid<u8>>("[1,2,3]").is_err());
        assert!(serde_json::from_str::<LocalGrid<u8>>("[]").is_err());
    }

    #[test]
    fn world_grid_wraps_the_single_local_grid() {
        let grid = LocalGrid::filled_with(3_u8);
        let mut world = WorldGrid::new(grid);
        assert_eq!(world.local().len(), LOCAL_GRID_CELL_COUNT);
        world.local_mut().set(5, 6, 9).expect("in bounds");
        assert_eq!(world.local().get(5, 6), Some(&9_u8));

        let encoded = serde_json::to_string(&world).expect("serialize world");
        let restored: WorldGrid<u8> = serde_json::from_str(&encoded).expect("deserialize world");
        assert_eq!(restored.local().get(5, 6), Some(&9_u8));
    }
}
