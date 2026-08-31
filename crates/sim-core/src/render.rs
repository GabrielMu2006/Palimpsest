// Authored by opencode (AI coding agent) — task CHRON-029 (ADR-0023); repaired
// to schema 2 under ADR-0024 D6 (KFIX-006).
//! The immutable, versioned render snapshot DTO (CHRON-029, ADR-0023).
//!
//! [`RenderSnapshot`] is the read-only, serde-serializable presentation
//! contract between the simulation core and the Godot client (ADR-0007,
//! ADR-0017): it batches the terrain grid, the static activity-site batch, up
//! to 100 persons with their current action and projected Needs, and
//! observable kernel metrics in one immutable value built strictly from the
//! [`WorldKernel`]'s committed boundary. It carries only stable [`EntityId`]
//! values (never ECS handles or scheduler tokens), has no mutation methods and
//! no interior mutability, and is a transient render view — not a save format
//! (ADR-0009/0016). Godot conversion belongs to CHRON-030/CHRON-031.
//!
//! Schema **2** (ADR-0024 D6) adds the activity-site batch, per-person Needs,
//! and the kernel round/transition/decision totals, and rejects schema 1.
//! The constructor takes no caller-supplied instant: a snapshot always
//! reflects `WorldKernel::now` at build time. Deserialization is for
//! diagnostics only and re-validates the schema, the exact 128×128 dimensions
//! and cell count, the non-zero/unique/ascending person identities, the
//! action/state/target correlation, the site boundary/ordering/walkability,
//! and the metric counts; imported values are never written back into the
//! world.

use std::collections::BTreeSet;
use std::error::Error;
use std::fmt::{Display, Formatter};

use palimpsest_sim_ai::{ActionKind, Needs};
use palimpsest_sim_entity::EntityId;
use palimpsest_sim_time::SimInstant;
use palimpsest_sim_world::{
    LOCAL_GRID_CELL_COUNT, LOCAL_GRID_HEIGHT, LOCAL_GRID_WIDTH, LocalCoord, SiteKind, TerrainKind,
};
use serde::{Deserialize, Deserializer, Serialize};

use crate::actions::ActionState;
use crate::kernel::{KernelReadError, WorldKernel};

/// Current transient render-snapshot bridge schema version (schema 2).
pub const RENDER_SCHEMA_VERSION: u16 = 2;

/// The static local terrain batch: a row-major 128×128 cell grid plus its
/// dimensions. This is the exact cell data the tile renderer consumes.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct TerrainBatch {
    width: usize,
    height: usize,
    cells: Vec<TerrainKind>,
}

impl TerrainBatch {
    /// Number of columns.
    #[must_use]
    pub const fn width(&self) -> usize {
        self.width
    }

    /// Number of rows.
    #[must_use]
    pub const fn height(&self) -> usize {
        self.height
    }

    /// The row-major terrain cells.
    #[must_use]
    pub fn cells(&self) -> &[TerrainKind] {
        &self.cells
    }

    fn validate(&self) -> Result<(), RenderError> {
        if self.width != LOCAL_GRID_WIDTH || self.height != LOCAL_GRID_HEIGHT {
            return Err(RenderError::InvalidDimensions {
                width: self.width,
                height: self.height,
            });
        }
        if self.cells.len() != LOCAL_GRID_CELL_COUNT {
            return Err(RenderError::InvalidCellCount {
                expected: LOCAL_GRID_CELL_COUNT,
                got: self.cells.len(),
            });
        }
        Ok(())
    }

    /// Builds a batch from the documented 128×128 grid.
    fn from_grid(cells: Vec<TerrainKind>) -> Self {
        debug_assert_eq!(cells.len(), LOCAL_GRID_CELL_COUNT);
        Self {
            width: LOCAL_GRID_WIDTH,
            height: LOCAL_GRID_HEIGHT,
            cells,
        }
    }
}

/// One static activity site in presentation space (ADR-0024 D6). It is a plain
/// value: a walkable coordinate plus its affordance; it carries no fabricated
/// `EntityId` and no inventory/economy.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ActivitySiteRender {
    coord: LocalCoord,
    kind: SiteKind,
}

impl ActivitySiteRender {
    /// The site coordinate in the local grid.
    #[must_use]
    pub const fn coord(&self) -> LocalCoord {
        self.coord
    }

    /// The site affordance kind.
    #[must_use]
    pub const fn kind(&self) -> SiteKind {
        self.kind
    }
}

/// One presented person: stable identity, tile, current action (plus target and
/// observable state), and its Needs at the snapshot instant. No ECS handle or
/// scheduler token appears.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub struct PersonRender {
    person_id: EntityId,
    tile: LocalCoord,
    action: ActionKind,
    action_target: Option<LocalCoord>,
    action_state: ActionState,
    needs: Needs,
}

impl PersonRender {
    /// The person's stable persistent identity.
    #[must_use]
    pub const fn person_id(&self) -> EntityId {
        self.person_id
    }

    /// The tile the person occupies.
    #[must_use]
    pub const fn tile(&self) -> LocalCoord {
        self.tile
    }

    /// The top-level action kind the person is executing.
    #[must_use]
    pub const fn action(&self) -> ActionKind {
        self.action
    }

    /// The action target, if any.
    #[must_use]
    pub const fn action_target(&self) -> Option<LocalCoord> {
        self.action_target
    }

    /// The observable action state.
    #[must_use]
    pub const fn action_state(&self) -> ActionState {
        self.action_state
    }

    /// The person's Needs at this snapshot instant (kernel projection).
    #[must_use]
    pub const fn needs(&self) -> Needs {
        self.needs
    }

    fn validate(&self) -> Result<(), RenderError> {
        validate_person_action(self)
    }
}

/// Observable kernel metrics for the metrics overlay, as a bounded batch.
///
/// Values the DTO cannot provide are left absent and labelled unavailable by
/// the presenter; wall-clock time and RSS never appear here.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct RenderMetrics {
    /// Number of presented persons.
    pub person_count: usize,
    /// Live scheduler payload depth under the action runtime.
    pub scheduler_queue_depth: usize,
    /// Total validated high-level outcome events accounted by the kernel.
    pub events_committed: u64,
    /// Outcome events currently buffered in the kernel.
    pub events_buffered: usize,
    /// Outcome events dropped by either retention buffer.
    pub buffer_rotations: u64,
    /// Persons with a live action execution record.
    pub live_actions: u64,
    /// Total advance rounds processed.
    pub rounds_total: u64,
    /// Total action transitions committed.
    pub transitions_total: u64,
    /// Total decisions resolved.
    pub decisions_total: u64,
}

/// The immutable, versioned render snapshot (CHRON-029, ADR-0023).
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RenderSnapshot {
    schema_version: u16,
    sim_second: SimInstant,
    terrain: TerrainBatch,
    sites: Vec<ActivitySiteRender>,
    persons: Vec<PersonRender>,
    metrics: RenderMetrics,
}

impl RenderSnapshot {
    /// Non-cryptographic comparison of this render DTO and its work counters.
    /// This is not a persistence format or a complete simulation-state hash.
    ///
    /// # Panics
    /// Only if serialization of this integer/string DTO unexpectedly fails.
    #[must_use]
    pub fn diagnostic_hash(&self) -> u64 {
        serde_json::to_vec(self)
            .expect("render snapshot serializes")
            .into_iter()
            .fold(0_u64, |hash, byte| {
                hash.wrapping_mul(1_000_003).wrapping_add(u64::from(byte))
            })
    }

    /// The transient bridge schema version.
    #[must_use]
    pub const fn schema_version(&self) -> u16 {
        self.schema_version
    }

    /// The committed simulation instant this snapshot reflects.
    #[must_use]
    pub const fn sim_second(&self) -> SimInstant {
        self.sim_second
    }

    /// The terrain batch.
    #[must_use]
    pub const fn terrain(&self) -> &TerrainBatch {
        &self.terrain
    }

    /// The static activity-site batch, ascending by `(y, x)`.
    #[must_use]
    pub fn sites(&self) -> &[ActivitySiteRender] {
        &self.sites
    }

    /// The presented persons, ascending by stable `EntityId`.
    #[must_use]
    pub fn persons(&self) -> &[PersonRender] {
        &self.persons
    }

    /// The observable kernel metrics.
    #[must_use]
    pub const fn metrics(&self) -> &RenderMetrics {
        &self.metrics
    }

    /// The number of presented persons.
    #[must_use]
    pub fn person_count(&self) -> usize {
        self.persons.len()
    }

    /// Builds the snapshot from the kernel's committed boundary.
    ///
    /// This is the sole constructor. It reads `WorldKernel::now`, the terrain
    /// grid, the static sites, the presented persons (ascending by `EntityId`,
    /// Needs projected to `now`), and the kernel metrics without mutating the
    /// kernel.
    ///
    /// # Errors
    ///
    /// Returns [`RenderError::KernelFaulted`] when the kernel is faulted, so a
    /// snapshot is never built off a half-committed boundary.
    ///
    /// # Panics
    ///
    /// Never in practice: the live-action count fits `u64` on the target and
    /// the kernel person/site iterators reflect a complete boundary.
    pub fn from_kernel(kernel: &WorldKernel) -> Result<Self, RenderError> {
        let mut persons: Vec<PersonRender> = kernel
            .persons()
            .map_err(RenderError::from)?
            .into_iter()
            .map(|view| PersonRender {
                person_id: view.id(),
                tile: view.location(),
                action: view.action(),
                action_target: view.action_target(),
                action_state: view.state(),
                needs: view.needs(),
            })
            .collect();
        persons.sort_unstable_by_key(PersonRender::person_id);
        let mut sites: Vec<ActivitySiteRender> = Vec::new();
        for kind in [SiteKind::Meal, SiteKind::Rest, SiteKind::Work] {
            for site in kernel.sites().map_err(RenderError::from)?.sites_of(kind) {
                sites.push(ActivitySiteRender {
                    coord: site.coord(),
                    kind: site.kind(),
                });
            }
        }
        sites.sort_unstable_by_key(ActivitySiteRender::coord);
        let cells = kernel.map().local().iter().copied().collect::<Vec<_>>();
        let metrics = kernel.metrics();
        let person_count = persons.len();
        let snapshot = Self {
            schema_version: RENDER_SCHEMA_VERSION,
            sim_second: kernel.now(),
            terrain: TerrainBatch::from_grid(cells),
            sites,
            persons,
            metrics: RenderMetrics {
                person_count,
                scheduler_queue_depth: metrics.scheduler_queue_depth,
                events_committed: metrics.events_total,
                events_buffered: metrics.events_buffered,
                buffer_rotations: metrics.events_rotated,
                live_actions: u64::try_from(metrics.live_actions).expect("person count fits u64"),
                rounds_total: metrics.rounds_total,
                transitions_total: metrics.transitions_total,
                decisions_total: metrics.decisions_total,
            },
        };
        snapshot.validate()?;
        Ok(snapshot)
    }

    /// Validates the structural invariants of this DTO (schema 2).
    ///
    /// # Errors
    ///
    /// Returns the first [`RenderError`] schema, size, identity, site,
    /// correlation, or metric violation.
    pub fn validate(&self) -> Result<(), RenderError> {
        if self.schema_version != RENDER_SCHEMA_VERSION {
            return Err(RenderError::UnsupportedSchemaVersion(self.schema_version));
        }
        self.terrain.validate()?;
        let mut seen = BTreeSet::new();
        let mut previous: Option<EntityId> = None;
        for (position, person) in self.persons.iter().enumerate() {
            let id = person.person_id();
            if !seen.insert(id) {
                return Err(RenderError::DuplicatePersonId(id));
            }
            if previous.is_some_and(|prev| id < prev) {
                return Err(RenderError::UnsortedPersons { position });
            }
            previous = Some(id);
            person.validate()?;
        }
        if self.metrics.person_count != self.persons.len() {
            return Err(RenderError::MetricCountMismatch {
                expected: self.persons.len(),
                got: self.metrics.person_count,
            });
        }
        self.validate_sites()?;
        Ok(())
    }

    fn validate_sites(&self) -> Result<(), RenderError> {
        let mut seen = BTreeSet::new();
        let mut previous: Option<LocalCoord> = None;
        for site in &self.sites {
            let coord = site.coord();
            if !seen.insert(coord) {
                return Err(RenderError::DuplicateSite(coord));
            }
            if previous.is_some_and(|prev| coord < prev) {
                return Err(RenderError::UnsortedSites { coord });
            }
            previous = Some(coord);
            let index = coord.index();
            if index >= self.terrain.cells.len() {
                return Err(RenderError::SiteOutOfBounds(coord));
            }
            if !self.terrain.cells[index].is_walkable() {
                return Err(RenderError::UnwalkableSite(coord));
            }
        }
        Ok(())
    }
}

/// Checks the action/state/target correlation invariant of one person.
fn validate_person_action(person: &PersonRender) -> Result<(), RenderError> {
    let (action, state, target) = (
        person.action(),
        person.action_state(),
        person.action_target(),
    );
    let ok = match state {
        ActionState::Idle => action == ActionKind::Idle && target.is_none(),
        ActionState::Moving { action: moving } => {
            action != ActionKind::Idle && action == moving && target.is_some()
        }
        ActionState::Eating => action == ActionKind::Eat && target.is_some(),
        ActionState::Sleeping => action == ActionKind::Sleep && target.is_some(),
        ActionState::Working => action == ActionKind::Work && target.is_some(),
    };
    if ok {
        Ok(())
    } else {
        Err(RenderError::ActionTargetMismatch {
            person: person.person_id(),
        })
    }
}

/// A structural violation of a render snapshot (ADR-0023 §2, ADR-0024 D6).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RenderError {
    /// The snapshot used an unsupported schema version.
    UnsupportedSchemaVersion(u16),
    /// The terrain batch dimensions were not exactly 128×128.
    InvalidDimensions {
        /// Provided width.
        width: usize,
        /// Provided height.
        height: usize,
    },
    /// The terrain grid was not exactly the 128×128 local map.
    InvalidCellCount {
        /// Required cell count.
        expected: usize,
        /// Provided cell count.
        got: usize,
    },
    /// Two persons presented the same stable identity.
    DuplicatePersonId(EntityId),
    /// The person batch was not ascending by `EntityId`.
    UnsortedPersons {
        /// The first out-of-order position.
        position: usize,
    },
    /// The reported metric person count disagreed with the batch length.
    MetricCountMismatch {
        /// Expected count from the batch.
        expected: usize,
        /// Reported metric count.
        got: usize,
    },
    /// A person's action/state/target combination is inconsistent.
    ActionTargetMismatch {
        /// The offending person.
        person: EntityId,
    },
    /// Two sites shared a coordinate.
    DuplicateSite(LocalCoord),
    /// The site batch was not ascending by `(y, x)`.
    UnsortedSites {
        /// The first out-of-order coordinate.
        coord: LocalCoord,
    },
    /// A site coordinate lay outside the terrain batch.
    SiteOutOfBounds(LocalCoord),
    /// A site coordinate was not walkable on the terrain batch.
    UnwalkableSite(LocalCoord),
    /// The kernel is faulted and no live snapshot can be built.
    KernelFaulted,
    /// A complete-boundary read failed its time invariant.
    KernelRead(KernelReadError),
}

impl Display for RenderError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedSchemaVersion(version) => {
                write!(formatter, "unsupported render schema version {version}")
            }
            Self::InvalidDimensions { width, height } => {
                write!(
                    formatter,
                    "terrain dimensions must be 128x128, got {width}x{height}"
                )
            }
            Self::InvalidCellCount { expected, got } => {
                write!(
                    formatter,
                    "terrain needs exactly {expected} cells, got {got}"
                )
            }
            Self::DuplicatePersonId(id) => write!(formatter, "duplicate person id {}", id.get()),
            Self::UnsortedPersons { position } => {
                write!(formatter, "person batch is out of order at {position}")
            }
            Self::MetricCountMismatch { expected, got } => {
                write!(
                    formatter,
                    "metric person count {got} does not match batch {expected}"
                )
            }
            Self::ActionTargetMismatch { person } => {
                write!(
                    formatter,
                    "person {} action/state/target is inconsistent",
                    person.get()
                )
            }
            Self::DuplicateSite(coord) => {
                write!(
                    formatter,
                    "duplicate activity site at ({}, {})",
                    coord.x(),
                    coord.y()
                )
            }
            Self::UnsortedSites { coord } => {
                write!(
                    formatter,
                    "site batch is out of order at ({}, {})",
                    coord.x(),
                    coord.y()
                )
            }
            Self::SiteOutOfBounds(coord) => {
                write!(
                    formatter,
                    "activity site ({}, {}) is out of bounds",
                    coord.x(),
                    coord.y()
                )
            }
            Self::UnwalkableSite(coord) => {
                write!(
                    formatter,
                    "activity site ({}, {}) is not walkable",
                    coord.x(),
                    coord.y()
                )
            }
            Self::KernelFaulted => formatter.write_str("kernel is faulted; no snapshot available"),
            Self::KernelRead(source) => write!(formatter, "kernel read failed: {source}"),
        }
    }
}

impl Error for RenderError {}

impl From<KernelReadError> for RenderError {
    fn from(source: KernelReadError) -> Self {
        match source {
            KernelReadError::KernelFaulted => Self::KernelFaulted,
            KernelReadError::InvalidNeedsTime { .. } => Self::KernelRead(source),
        }
    }
}

#[derive(Deserialize)]
struct TerrainBatchWire {
    width: usize,
    height: usize,
    cells: Vec<TerrainKind>,
}

impl<'de> Deserialize<'de> for TerrainBatch {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let wire = TerrainBatchWire::deserialize(deserializer)?;
        let batch = Self {
            width: wire.width,
            height: wire.height,
            cells: wire.cells,
        };
        batch.validate().map_err(serde::de::Error::custom)?;
        Ok(batch)
    }
}

#[derive(Deserialize)]
struct PersonRenderWire {
    person_id: EntityId,
    tile: LocalCoord,
    action: ActionKind,
    action_target: Option<LocalCoord>,
    action_state: ActionState,
    needs: Needs,
}

impl<'de> Deserialize<'de> for PersonRender {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let wire = PersonRenderWire::deserialize(deserializer)?;
        let person = Self {
            person_id: wire.person_id,
            tile: wire.tile,
            action: wire.action,
            action_target: wire.action_target,
            action_state: wire.action_state,
            needs: wire.needs,
        };
        person.validate().map_err(serde::de::Error::custom)?;
        Ok(person)
    }
}

/// Serde wire form, re-validated on deserialization.
#[derive(Deserialize)]
struct RenderSnapshotWire {
    schema_version: u16,
    sim_second: SimInstant,
    terrain: TerrainBatch,
    sites: Vec<ActivitySiteRender>,
    persons: Vec<PersonRender>,
    metrics: RenderMetrics,
}

impl<'de> Deserialize<'de> for RenderSnapshot {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let wire = RenderSnapshotWire::deserialize(deserializer)?;
        let snapshot = Self {
            schema_version: wire.schema_version,
            sim_second: wire.sim_second,
            terrain: wire.terrain,
            sites: wire.sites,
            persons: wire.persons,
            metrics: wire.metrics,
        };
        snapshot.validate().map_err(serde::de::Error::custom)?;
        Ok(snapshot)
    }
}
