// Authored by Kimi Code (AI coding agent) — task CHRON-020.
//! Deterministic world generation for the single 128×128 local map.
//!
//! The generator is pure integer arithmetic — a splitmix64-style hash
//! (Stafford/Vigna, public domain) plus fixed-point value noise with two
//! octaves. No floats, no threads, no wall-clock, and no randomized `std`
//! hashing, so equal `(WorldSeed, WorldGenConfig)` inputs produce
//! byte-identical maps on every platform (Master Spec §63).

use serde::{Deserialize, Deserializer, Serialize};

use crate::grid::{
    LOCAL_GRID_CELL_COUNT, LOCAL_GRID_HEIGHT, LOCAL_GRID_WIDTH, LocalGrid, WorldGrid,
};
use crate::terrain::TerrainKind;

/// Fixed-point interpolation scale for the value-noise octaves.
const NOISE_SCALE: i64 = 1024;
/// Noise value below which a cell becomes water.
const WATER_BELOW: i64 = 341;
/// Noise value above which a cell becomes rock.
const ROCK_ABOVE: i64 = 682;
/// Cell size of the coarse noise octave.
const MAJOR_CELL_SIZE: u32 = 16;
/// Cell size of the detail noise octave.
const MINOR_CELL_SIZE: u32 = 8;
/// Decorrelation salts for the noise octaves and spawn placement.
const SALT_MAJOR_OCTAVE: u64 = 0x243F_6A88_85A3_08D3;
const SALT_MINOR_OCTAVE: u64 = 0x4528_21E6_38D0_1377;
const SALT_SPAWN_X: u64 = 0x1319_8A2E_0370_7344;
const SALT_SPAWN_Y: u64 = 0xBE54_66CF_34E9_0C6C;

/// A world-generation seed: any `u64`, including zero, is valid and
/// reproducible.
///
/// `WorldSeed` is deliberately a distinct newtype from
/// `palimpsest_sim_entity::EntityId`: seeds are world inputs, never entity
/// identity. Generation takes the seed by value and never advances or mutates
/// it. Serde encodes a plain unsigned integer.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct WorldSeed(u64);

impl WorldSeed {
    /// The zero seed; valid and reproducible like any other.
    pub const ZERO: Self = Self(0);

    /// Creates a seed from any `u64` value.
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Returns the raw seed value.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Generator settings: the algorithm version and the minimum guaranteed
/// walkable spawn area. There are deliberately no terrain-ratio or climate
/// knobs (Master Spec §64).
///
/// Only [`WorldGenConfig::GENERATOR_VERSION`] is supported; other versions
/// and out-of-range spawn sizes are rejected at construction and at
/// deserialization.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub struct WorldGenConfig {
    generator_version: u32,
    min_walkable_spawn_cells: usize,
}

impl WorldGenConfig {
    /// The only generator version implemented by this crate.
    pub const GENERATOR_VERSION: u32 = 1;
    /// Largest accepted spawn-clearing requirement (a quarter of the map).
    pub const MAX_SPAWN_CELLS: usize = LOCAL_GRID_CELL_COUNT / 4;
    /// Default spawn guarantee: a connected clearing of at least 64 cells
    /// (carved as an 8×8 square).
    pub const DEFAULT_SPAWN_CELLS: usize = 64;

    /// Creates a config when the version is supported and the spawn
    /// requirement is in `1..=MAX_SPAWN_CELLS`.
    #[must_use]
    pub fn new(generator_version: u32, min_walkable_spawn_cells: usize) -> Option<Self> {
        if generator_version != Self::GENERATOR_VERSION {
            return None;
        }
        if min_walkable_spawn_cells == 0 || min_walkable_spawn_cells > Self::MAX_SPAWN_CELLS {
            return None;
        }
        Some(Self {
            generator_version,
            min_walkable_spawn_cells,
        })
    }

    /// The generator algorithm version.
    #[must_use]
    pub const fn generator_version(self) -> u32 {
        self.generator_version
    }

    /// Minimum size in cells of the guaranteed connected walkable spawn area.
    #[must_use]
    pub const fn min_walkable_spawn_cells(self) -> usize {
        self.min_walkable_spawn_cells
    }
}

impl Default for WorldGenConfig {
    fn default() -> Self {
        Self {
            generator_version: Self::GENERATOR_VERSION,
            min_walkable_spawn_cells: Self::DEFAULT_SPAWN_CELLS,
        }
    }
}

/// Serde wire form, re-validated on deserialization.
#[derive(Deserialize)]
struct WorldGenConfigWire {
    generator_version: u32,
    min_walkable_spawn_cells: usize,
}

impl<'de> Deserialize<'de> for WorldGenConfig {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let wire = WorldGenConfigWire::deserialize(deserializer)?;
        Self::new(wire.generator_version, wire.min_walkable_spawn_cells).ok_or_else(|| {
            serde::de::Error::custom(format_args!(
                "unsupported world-gen config: version {}, min spawn cells {}",
                wire.generator_version, wire.min_walkable_spawn_cells
            ))
        })
    }
}

/// A generated world: the single 128×128 local terrain map plus the exact
/// seed and config that produced it, so provenance travels with the map.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct WorldMap {
    grid: WorldGrid<TerrainKind>,
    seed: WorldSeed,
    config: WorldGenConfig,
}

impl WorldMap {
    /// Generates the single local map for `seed` under `config`.
    ///
    /// This is the sole public generation entry point. Equal inputs produce
    /// byte-identical maps on every platform; the generated map always
    /// contains a connected walkable spawn clearing of at least
    /// `config.min_walkable_spawn_cells()` and at least one impassable cell.
    ///
    /// # Panics
    ///
    /// Never in practice: panics only if the generator were to emit a cell
    /// count other than 16,384, which is a fixed compile-time invariant.
    #[must_use]
    pub fn generate(seed: WorldSeed, config: WorldGenConfig) -> Self {
        let cells = generate_cells(seed, config);
        let local = LocalGrid::from_cells(cells).expect("the generator emits exactly 16384 cells");
        Self {
            grid: WorldGrid::new(local),
            seed,
            config,
        }
    }

    /// The single local terrain map.
    #[must_use]
    pub fn local(&self) -> &LocalGrid<TerrainKind> {
        self.grid.local()
    }

    /// The world-grid wrapper around the local map.
    #[must_use]
    pub fn grid(&self) -> &WorldGrid<TerrainKind> {
        &self.grid
    }

    /// The seed this map was generated from.
    #[must_use]
    pub const fn seed(&self) -> WorldSeed {
        self.seed
    }

    /// The config this map was generated with.
    #[must_use]
    pub const fn config(&self) -> WorldGenConfig {
        self.config
    }
}

/// The spawn clearing carved into the map, in cell coordinates.
#[derive(Clone, Copy)]
struct CarveRect {
    origin_x: usize,
    origin_y: usize,
    side: usize,
}

/// The splitmix64 finalizer: a well-documented, platform-independent integer
/// hash (Stafford/Vigna, public domain reference constants).
const fn splitmix64_mix(mut value: u64) -> u64 {
    value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^ (value >> 31)
}

/// Hashes one noise-lattice point to a fixed-point value in
/// `[0, NOISE_SCALE)`.
fn lattice_value(seed: WorldSeed, lattice_x: u32, lattice_y: u32, salt: u64) -> i64 {
    let packed = (u64::from(lattice_x) << 32) | u64::from(lattice_y);
    let hashed = splitmix64_mix(seed.get() ^ splitmix64_mix(packed).wrapping_add(salt));
    i64::try_from(hashed >> 54).expect("a 10-bit value fits i64")
}

/// Smoothstep-eases a fraction in `[0, NOISE_SCALE]` with integer math.
fn smoothstep(t: i64) -> i64 {
    (t * t * (3 * NOISE_SCALE - 2 * t)) / (NOISE_SCALE * NOISE_SCALE)
}

/// Linear interpolation between fixed-point values.
fn lerp(a: i64, b: i64, t: i64) -> i64 {
    a + (b - a) * t / NOISE_SCALE
}

/// Fixed-point value noise for one octave of the given lattice cell size.
fn octave(seed: WorldSeed, x: u32, y: u32, cell_size: u32, salt: u64) -> i64 {
    let lattice_x = x / cell_size;
    let lattice_y = y / cell_size;
    let frac_x = smoothstep(i64::from(x % cell_size) * NOISE_SCALE / i64::from(cell_size));
    let frac_y = smoothstep(i64::from(y % cell_size) * NOISE_SCALE / i64::from(cell_size));
    let v00 = lattice_value(seed, lattice_x, lattice_y, salt);
    let v10 = lattice_value(seed, lattice_x + 1, lattice_y, salt);
    let v01 = lattice_value(seed, lattice_x, lattice_y + 1, salt);
    let v11 = lattice_value(seed, lattice_x + 1, lattice_y + 1, salt);
    let top = lerp(v00, v10, frac_x);
    let bottom = lerp(v01, v11, frac_x);
    lerp(top, bottom, frac_y)
}

/// Smallest square side whose area is at least `value`.
fn ceil_sqrt(value: usize) -> usize {
    let mut side = 1_usize;
    while side * side < value {
        side += 1;
    }
    side
}

/// Carves a deterministically placed, fully walkable square that satisfies
/// the config's minimum spawn-area guarantee.
fn carve_spawn_clearing(
    cells: &mut [TerrainKind],
    seed: WorldSeed,
    config: WorldGenConfig,
) -> CarveRect {
    let side = ceil_sqrt(config.min_walkable_spawn_cells());
    let max_origin = LOCAL_GRID_WIDTH - side;
    let span = u64::try_from(max_origin + 1).expect("origin span fits u64");
    let origin_x =
        usize::try_from(splitmix64_mix(seed.get() ^ SALT_SPAWN_X) % span).expect("origin fits");
    let origin_y =
        usize::try_from(splitmix64_mix(seed.get() ^ SALT_SPAWN_Y) % span).expect("origin fits");
    for dy in 0..side {
        for dx in 0..side {
            cells[(origin_y + dy) * LOCAL_GRID_WIDTH + (origin_x + dx)] = TerrainKind::Ground;
        }
    }
    CarveRect {
        origin_x,
        origin_y,
        side,
    }
}

/// Guarantees at least one impassable cell even for a pathological
/// all-ground noise outcome, without touching the spawn clearing.
fn ensure_impassable_feature(cells: &mut [TerrainKind], spawn: CarveRect) {
    if cells.iter().any(|cell| !cell.is_walkable()) {
        return;
    }
    for y in 0..LOCAL_GRID_HEIGHT {
        for x in 0..LOCAL_GRID_WIDTH {
            let inside_spawn = x >= spawn.origin_x
                && x < spawn.origin_x + spawn.side
                && y >= spawn.origin_y
                && y < spawn.origin_y + spawn.side;
            if !inside_spawn {
                cells[y * LOCAL_GRID_WIDTH + x] = TerrainKind::Rock;
                return;
            }
        }
    }
}

/// Generates the raw cell vector: two-octave value noise thresholded into
/// water/ground/rock, then the spawn clearing and impassable guarantee.
fn generate_cells(seed: WorldSeed, config: WorldGenConfig) -> Vec<TerrainKind> {
    let mut cells = vec![TerrainKind::Ground; LOCAL_GRID_CELL_COUNT];
    for y in 0..LOCAL_GRID_HEIGHT {
        for x in 0..LOCAL_GRID_WIDTH {
            let xu = u32::try_from(x).expect("grid axis fits u32");
            let yu = u32::try_from(y).expect("grid axis fits u32");
            let major = octave(seed, xu, yu, MAJOR_CELL_SIZE, SALT_MAJOR_OCTAVE);
            let minor = octave(seed, xu, yu, MINOR_CELL_SIZE, SALT_MINOR_OCTAVE);
            let value = (2 * major + minor) / 3;
            let kind = if value < WATER_BELOW {
                TerrainKind::Water
            } else if value > ROCK_ABOVE {
                TerrainKind::Rock
            } else {
                TerrainKind::Ground
            };
            cells[y * LOCAL_GRID_WIDTH + x] = kind;
        }
    }
    let spawn = carve_spawn_clearing(&mut cells, seed, config);
    ensure_impassable_feature(&mut cells, spawn);
    cells
}

#[cfg(test)]
mod tests {
    use super::{WorldGenConfig, WorldMap, WorldSeed};
    use crate::grid::{LOCAL_GRID_CELL_COUNT, LocalGrid};
    use crate::terrain::TerrainKind;

    /// Deterministic content hash (FNV-1a 64) over the terrain cells; used to
    /// lock golden seeds against accidental generator changes.
    fn content_hash(map: &LocalGrid<TerrainKind>) -> u64 {
        let mut hash = 0xcbf2_9ce4_8422_2325_u64;
        for cell in map.iter() {
            let byte = match cell {
                TerrainKind::Ground => 0_u64,
                TerrainKind::Water => 1,
                TerrainKind::Rock => 2,
            };
            hash = (hash ^ byte).wrapping_mul(0x0000_0100_0000_01b3);
        }
        hash
    }

    /// Size of the largest connected (4-neighbour) walkable region.
    fn largest_walkable_region(map: &LocalGrid<TerrainKind>) -> usize {
        let mut visited = vec![false; LOCAL_GRID_CELL_COUNT];
        let mut largest = 0_usize;
        for start in 0..LOCAL_GRID_CELL_COUNT {
            let walkable = map.get_index(start).expect("in range").is_walkable();
            if visited[start] || !walkable {
                continue;
            }
            let mut stack = vec![start];
            let mut size = 0_usize;
            while let Some(index) = stack.pop() {
                if visited[index] {
                    continue;
                }
                visited[index] = true;
                size += 1;
                let x = index % crate::grid::LOCAL_GRID_WIDTH;
                let y = index / crate::grid::LOCAL_GRID_WIDTH;
                for neighbour in [
                    (x > 0).then(|| index - 1),
                    (x + 1 < crate::grid::LOCAL_GRID_WIDTH).then(|| index + 1),
                    (y > 0).then(|| index - crate::grid::LOCAL_GRID_WIDTH),
                    (y + 1 < crate::grid::LOCAL_GRID_HEIGHT)
                        .then(|| index + crate::grid::LOCAL_GRID_WIDTH),
                ]
                .into_iter()
                .flatten()
                {
                    if !visited[neighbour]
                        && map.get_index(neighbour).expect("in range").is_walkable()
                    {
                        stack.push(neighbour);
                    }
                }
            }
            largest = largest.max(size);
        }
        largest
    }

    #[test]
    fn same_seed_is_cell_and_byte_identical() {
        let config = WorldGenConfig::default();
        let first = WorldMap::generate(WorldSeed::new(42), config);
        let second = WorldMap::generate(WorldSeed::new(42), config);
        assert!(first.local().iter().eq(second.local().iter()));
        let first_bytes = serde_json::to_vec(first.local()).expect("serialize first map");
        let second_bytes = serde_json::to_vec(second.local()).expect("serialize second map");
        assert_eq!(first_bytes, second_bytes);
    }

    #[test]
    fn different_seeds_diverge() {
        let config = WorldGenConfig::default();
        let one = WorldMap::generate(WorldSeed::new(1), config);
        let two = WorldMap::generate(WorldSeed::new(2), config);
        assert_ne!(content_hash(one.local()), content_hash(two.local()));
    }

    #[test]
    fn zero_seed_is_valid_and_reproducible() {
        let config = WorldGenConfig::default();
        let first = WorldMap::generate(WorldSeed::ZERO, config);
        let second = WorldMap::generate(WorldSeed::ZERO, config);
        assert!(first.local().iter().eq(second.local().iter()));
    }

    #[test]
    fn spawn_clearing_and_impassable_feature_are_guaranteed() {
        for seed in [0_u64, 1, 42, 12_345, u64::MAX] {
            let map = WorldMap::generate(WorldSeed::new(seed), WorldGenConfig::default());
            let largest = largest_walkable_region(map.local());
            assert!(
                largest >= WorldGenConfig::DEFAULT_SPAWN_CELLS,
                "seed {seed}: largest walkable region {largest} below the spawn guarantee"
            );
            assert!(
                map.local().iter().any(|cell| !cell.is_walkable()),
                "seed {seed}: map must contain an impassable feature"
            );
        }
    }

    #[test]
    fn custom_spawn_size_is_honored() {
        let config =
            WorldGenConfig::new(WorldGenConfig::GENERATOR_VERSION, 100).expect("valid config");
        let map = WorldMap::generate(WorldSeed::new(7), config);
        assert!(largest_walkable_region(map.local()) >= 100);
    }

    #[test]
    fn generated_map_has_exact_grid_shape() {
        let map = WorldMap::generate(WorldSeed::new(9), WorldGenConfig::default());
        assert_eq!(map.local().len(), LOCAL_GRID_CELL_COUNT);
        assert!(!map.local().is_empty());
    }

    #[test]
    fn walkability_matches_terrain_kind_everywhere() {
        let map = WorldMap::generate(WorldSeed::new(3), WorldGenConfig::default());
        for cell in map.local().iter() {
            assert_eq!(cell.is_walkable(), *cell == TerrainKind::Ground);
        }
    }

    #[test]
    fn provenance_travels_with_the_map() {
        let seed = WorldSeed::new(77);
        let config = WorldGenConfig::default();
        let map = WorldMap::generate(seed, config);
        assert_eq!(map.seed(), seed);
        assert_eq!(map.config(), config);
    }

    #[test]
    fn config_validation_rejects_unsupported_inputs() {
        assert!(WorldGenConfig::new(2, 64).is_none());
        assert!(WorldGenConfig::new(WorldGenConfig::GENERATOR_VERSION, 0).is_none());
        assert!(
            WorldGenConfig::new(
                WorldGenConfig::GENERATOR_VERSION,
                WorldGenConfig::MAX_SPAWN_CELLS + 1
            )
            .is_none()
        );
        assert!(
            WorldGenConfig::new(
                WorldGenConfig::GENERATOR_VERSION,
                WorldGenConfig::MAX_SPAWN_CELLS
            )
            .is_some()
        );
        let default = WorldGenConfig::default();
        assert_eq!(
            default.generator_version(),
            WorldGenConfig::GENERATOR_VERSION
        );
        assert_eq!(
            default.min_walkable_spawn_cells(),
            WorldGenConfig::DEFAULT_SPAWN_CELLS
        );
    }

    #[test]
    fn serde_round_trips() {
        let seed = WorldSeed::new(u64::MAX);
        assert_eq!(
            serde_json::to_string(&seed).expect("serialize seed"),
            "18446744073709551615"
        );
        assert_eq!(
            serde_json::from_str::<WorldSeed>("0").expect("deserialize zero seed"),
            WorldSeed::ZERO
        );
        assert_eq!(
            serde_json::from_str::<WorldSeed>("18446744073709551615").expect("deserialize seed"),
            seed
        );

        let config = WorldGenConfig::default();
        let encoded = serde_json::to_string(&config).expect("serialize config");
        assert_eq!(
            serde_json::from_str::<WorldGenConfig>(&encoded).expect("deserialize config"),
            config
        );
        assert!(
            serde_json::from_str::<WorldGenConfig>(
                "{\"generator_version\":2,\"min_walkable_spawn_cells\":64}"
            )
            .is_err()
        );

        let map = WorldMap::generate(WorldSeed::new(5), config);
        let encoded = serde_json::to_string(map.local()).expect("serialize map");
        let restored: LocalGrid<TerrainKind> =
            serde_json::from_str(&encoded).expect("deserialize map");
        assert!(map.local().iter().eq(restored.iter()));
        assert_eq!(content_hash(map.local()), content_hash(&restored));
    }

    #[test]
    fn golden_seeds_lock_the_generator() {
        // Locked on 2026-08-29 (generator version 1, M5 reference machine);
        // any intentional generator change must bump
        // `WorldGenConfig::GENERATOR_VERSION` and re-lock these values.
        let config = WorldGenConfig::default();
        let goldens = [
            (0_u64, 10_103_231_413_028_631_179_u64),
            (1, 9_466_269_938_330_766_210),
            (42, 8_056_959_030_977_719_378),
        ];
        for (seed, expected_hash) in goldens {
            let map = WorldMap::generate(WorldSeed::new(seed), config);
            assert_eq!(
                content_hash(map.local()),
                expected_hash,
                "golden map changed for seed {seed}"
            );
        }
    }
}
