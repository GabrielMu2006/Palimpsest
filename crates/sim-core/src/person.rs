// Authored by Kimi Code (AI coding agent) — task CHRON-021.
//! Person runtime shell: a stable `EntityId` bound to a rebuildable runtime
//! ECS handle plus a valid tile `Location`.
//!
//! The runtime owns the `bevy_ecs` world and a non-persistent
//! `EntityId -> Entity` handle map that is rebuilt with the world and never
//! serialized (ADR-0002, ADR-0011). Needs are attached at spawn (CHRON-022);
//! `CurrentAction` arrives with CHRON-025; no Phase 2 person depth (body,
//! personality, relations, memory, ...) exists.

use std::collections::HashMap;

use bevy_ecs::component::Component;
use bevy_ecs::entity::Entity;
use bevy_ecs::world::World;
use serde::{Deserialize, Serialize};

use palimpsest_sim_ai::Needs;
use palimpsest_sim_entity::{EntityId, EntityIdAllocator};
use palimpsest_sim_world::LocalCoord;

/// Marker component: this runtime entity is a person.
#[derive(Component, Clone, Copy, Debug)]
pub struct Person;

/// Component carrying the persistent domain identity (ADR-0002/0011): the
/// only identity used by events, snapshots, and client views.
#[derive(Component, Clone, Copy, Debug, Eq, PartialEq)]
pub struct StableEntityId(pub EntityId);

/// Component: the tile a person occupies on the single local map.
///
/// Always a valid in-bounds coordinate: `LocalCoord` cannot represent an
/// out-of-bounds tile (CHRON-019), so no invalid state or panic path exists.
#[derive(Component, Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Location(pub LocalCoord);

/// Component: a person's hunger/fatigue drives.
///
/// The domain model lives in `palimpsest-sim-ai` (CHRON-022); this is the
/// narrow ECS attachment point. Drives change only through explicit
/// `advance`/`eat`/`rest` calls issued by the kernel — never implicitly per
/// tick.
#[derive(Component, Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PersonNeeds(pub Needs);

/// Failures from person runtime operations.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PersonError {
    /// The stable identity space was exhausted before a handle was bound.
    IdentityExhausted,
    /// No person with this stable identity exists in the runtime.
    UnknownPerson {
        /// The identity that was not found.
        id: EntityId,
    },
}

impl core::fmt::Display for PersonError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::IdentityExhausted => formatter.write_str("stable identity space is exhausted"),
            Self::UnknownPerson { id } => {
                write!(formatter, "no person with stable identity {}", id.get())
            }
        }
    }
}

impl std::error::Error for PersonError {}

/// Read-only view of a person crossing the domain boundary: stable identity
/// and location only. This is the only person shape that is serializable;
/// runtime handles never appear in it.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PersonView {
    id: EntityId,
    location: LocalCoord,
}

impl PersonView {
    /// The person's stable persistent identity.
    #[must_use]
    pub const fn id(&self) -> EntityId {
        self.id
    }

    /// The tile the person occupies.
    #[must_use]
    pub const fn location(&self) -> LocalCoord {
        self.location
    }
}

/// Owns the person runtime: the ECS world plus the non-persistent
/// `EntityId -> Entity` handle map.
///
/// The handle map is rebuilt together with the world and is never serialized
/// (ADR-0011). No public method exposes a runtime handle; all public identity
/// is `EntityId`.
///
/// The stable identity API remains available to external callers:
///
/// ```
/// use palimpsest_sim_core::{EntityId, PersonRuntime};
///
/// let runtime = PersonRuntime::new();
/// assert_eq!(runtime.location(EntityId::MIN), None);
/// ```
///
/// Runtime ECS handles are intentionally not part of that API:
///
/// ```compile_fail
/// use palimpsest_sim_core::{EntityId, PersonRuntime};
///
/// let runtime = PersonRuntime::new();
/// let _handle = runtime.runtime_handle(EntityId::MIN);
/// ```
#[derive(Debug, Default)]
pub struct PersonRuntime {
    world: World,
    handles: HashMap<EntityId, Entity>,
}

impl PersonRuntime {
    /// Creates an empty person runtime.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Spawns a person: allocates a fresh stable identity, binds a new
    /// runtime handle, and attaches `Person` + `Location`.
    ///
    /// # Errors
    ///
    /// Returns [`PersonError::IdentityExhausted`] when the identity space is
    /// exhausted; no partial state is left behind.
    pub fn spawn(
        &mut self,
        allocator: &mut EntityIdAllocator,
        location: LocalCoord,
    ) -> Result<EntityId, PersonError> {
        let id = allocator
            .allocate()
            .map_err(|_| PersonError::IdentityExhausted)?;
        let handle = self
            .world
            .spawn((
                Person,
                StableEntityId(id),
                Location(location),
                PersonNeeds(Needs::default()),
            ))
            .id();
        let previous = self.handles.insert(id, handle);
        debug_assert!(previous.is_none(), "stable identities are never reused");
        Ok(id)
    }

    /// Returns the number of live persons.
    #[must_use]
    pub fn person_count(&self) -> usize {
        self.handles.len()
    }

    /// Returns the read-only identity + location view for `id`.
    #[must_use]
    pub fn get(&self, id: EntityId) -> Option<PersonView> {
        let handle = *self.handles.get(&id)?;
        let entity = self.world.get_entity(handle).ok()?;
        let location = entity.get::<Location>()?;
        Some(PersonView {
            id,
            location: location.0,
        })
    }

    /// Returns the tile occupied by `id`; the accessor used by later systems
    /// and Developer Metrics.
    #[must_use]
    pub fn location(&self, id: EntityId) -> Option<LocalCoord> {
        Some(self.get(id)?.location())
    }

    /// Explicitly updates the tile location of `id`.
    ///
    /// Out-of-bounds locations are unrepresentable: `location` is a
    /// validated `LocalCoord`, so no bounds error can occur at this layer.
    ///
    /// # Errors
    ///
    /// Returns [`PersonError::UnknownPerson`] when `id` is not a live person;
    /// no state is changed in that case.
    pub fn set_location(&mut self, id: EntityId, location: LocalCoord) -> Result<(), PersonError> {
        let handle = *self
            .handles
            .get(&id)
            .ok_or(PersonError::UnknownPerson { id })?;
        let mut entity = self
            .world
            .get_entity_mut(handle)
            .map_err(|_| PersonError::UnknownPerson { id })?;
        let mut current = entity
            .get_mut::<Location>()
            .ok_or(PersonError::UnknownPerson { id })?;
        current.0 = location;
        Ok(())
    }

    /// Test-only handle lookup. Runtime handles are rebuildable indexes: they
    /// are never serialized and never cross the persistence/bridge boundary
    /// (ADR-0002/0011).
    #[cfg(test)]
    #[must_use]
    fn runtime_handle(&self, id: EntityId) -> Option<Entity> {
        self.handles.get(&id).copied()
    }

    /// Returns the hunger/fatigue drives of `id`.
    #[must_use]
    pub fn needs(&self, id: EntityId) -> Option<Needs> {
        let handle = *self.handles.get(&id)?;
        self.world
            .get_entity(handle)
            .ok()?
            .get::<PersonNeeds>()
            .map(|needs| needs.0)
    }

    /// Replaces the drives of `id`, e.g. after `Needs::advance`.
    ///
    /// # Errors
    ///
    /// Returns [`PersonError::UnknownPerson`] when `id` is not a live person;
    /// no state is changed in that case.
    pub fn set_needs(&mut self, id: EntityId, needs: Needs) -> Result<(), PersonError> {
        let handle = *self
            .handles
            .get(&id)
            .ok_or(PersonError::UnknownPerson { id })?;
        let mut entity = self
            .world
            .get_entity_mut(handle)
            .map_err(|_| PersonError::UnknownPerson { id })?;
        let mut current = entity
            .get_mut::<PersonNeeds>()
            .ok_or(PersonError::UnknownPerson { id })?;
        current.0 = needs;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::{PersonError, PersonRuntime, PersonView};
    use palimpsest_sim_ai::Needs;
    use palimpsest_sim_entity::{EntityId, EntityIdAllocator};
    use palimpsest_sim_time::SimDuration;
    use palimpsest_sim_world::LocalCoord;

    fn coord(x: i32, y: i32) -> LocalCoord {
        LocalCoord::new(x, y).expect("test coordinates are in bounds")
    }

    #[test]
    fn spawn_pairs_unique_stable_ids_with_distinct_handles() {
        let mut runtime = PersonRuntime::new();
        let mut allocator = EntityIdAllocator::default();
        let mut ids = Vec::new();
        for index in 0..100_i32 {
            let id = runtime
                .spawn(&mut allocator, coord(index % 128, index / 128))
                .expect("identity capacity");
            ids.push(id);
        }
        assert_eq!(runtime.person_count(), 100);
        let unique_ids: HashSet<EntityId> = ids.iter().copied().collect();
        assert_eq!(unique_ids.len(), 100);
        let handles: HashSet<_> = ids
            .iter()
            .map(|id| runtime.runtime_handle(*id).expect("handle bound"))
            .collect();
        assert_eq!(handles.len(), 100, "no runtime handle may be reused");
    }

    #[test]
    fn fresh_spawn_has_the_supplied_location() {
        let mut runtime = PersonRuntime::new();
        let mut allocator = EntityIdAllocator::default();
        let id = runtime.spawn(&mut allocator, coord(8, 9)).expect("spawn");
        let view = runtime.get(id).expect("person exists");
        assert_eq!(view.id(), id);
        assert_eq!(view.location(), coord(8, 9));
        assert_eq!(runtime.location(id), Some(coord(8, 9)));
    }

    #[test]
    fn set_location_is_explicit_and_unknown_ids_change_nothing() {
        let mut runtime = PersonRuntime::new();
        let mut allocator = EntityIdAllocator::default();
        let id = runtime.spawn(&mut allocator, coord(3, 4)).expect("spawn");
        let missing = EntityId::new(999).expect("non-zero identity");

        runtime
            .set_location(id, coord(10, 11))
            .expect("known person");
        assert_eq!(runtime.location(id), Some(coord(10, 11)));

        assert_eq!(runtime.get(missing), None);
        assert_eq!(runtime.location(missing), None);
        assert_eq!(
            runtime.set_location(missing, coord(5, 5)),
            Err(PersonError::UnknownPerson { id: missing })
        );
        assert_eq!(runtime.location(id), Some(coord(10, 11)));
        assert_eq!(runtime.person_count(), 1);
    }

    #[test]
    fn invalid_locations_are_unrepresentable() {
        // The bounds rule: every public API takes a validated `LocalCoord`,
        // which cannot be constructed out of bounds (CHRON-019). Invalid
        // locations therefore cannot be expressed, let alone stored.
        assert_eq!(LocalCoord::new(128, 0), None);
        assert_eq!(LocalCoord::new(-1, 0), None);
        let mut runtime = PersonRuntime::new();
        let mut allocator = EntityIdAllocator::default();
        let id = runtime
            .spawn(&mut allocator, coord(127, 127))
            .expect("spawn");
        runtime.set_location(id, coord(0, 0)).expect("known person");
        assert_eq!(runtime.location(id), Some(coord(0, 0)));
    }

    #[test]
    fn allocator_exhaustion_is_an_explicit_error() {
        let mut runtime = PersonRuntime::new();
        let mut allocator = EntityIdAllocator::default();
        allocator.advance_past(EntityId::MAX);
        assert_eq!(
            runtime.spawn(&mut allocator, coord(0, 0)),
            Err(PersonError::IdentityExhausted)
        );
        assert_eq!(runtime.person_count(), 0);
    }

    #[test]
    fn identical_sequences_produce_identical_visible_state() {
        fn run() -> Vec<(u64, i32, i32)> {
            let mut runtime = PersonRuntime::new();
            let mut allocator = EntityIdAllocator::default();
            let mut ids = Vec::new();
            for index in 0..50_i32 {
                ids.push(
                    runtime
                        .spawn(&mut allocator, coord(index, 7))
                        .expect("identity capacity"),
                );
            }
            for (offset, id) in ids.iter().enumerate() {
                let row = i32::try_from(offset % 16).expect("small offset fits i32");
                runtime
                    .set_location(*id, coord(row, 9))
                    .expect("known person");
            }
            ids.iter()
                .map(|id| {
                    let view = runtime.get(*id).expect("person exists");
                    (view.id().get(), view.location().x(), view.location().y())
                })
                .collect()
        }
        assert_eq!(run(), run(), "identity sequence and visible state differ");
    }

    #[test]
    fn only_stable_identity_and_location_are_serializable() {
        let mut runtime = PersonRuntime::new();
        let mut allocator = EntityIdAllocator::default();
        let id = runtime.spawn(&mut allocator, coord(8, 9)).expect("spawn");
        let view = runtime.get(id).expect("person exists");
        let encoded = serde_json::to_string(&view).expect("serialize view");
        assert_eq!(encoded, "{\"id\":1,\"location\":{\"x\":8,\"y\":9}}");
        let restored: PersonView = serde_json::from_str(&encoded).expect("deserialize view");
        assert_eq!(restored, view);
        // `PersonRuntime` deliberately has no `Serialize` implementation and
        // the handle map is reachable only through the test-only private
        // `runtime_handle` helper: handles never cross the persistence or
        // bridge boundary (ADR-0002/0011).
    }

    #[test]
    fn spawn_attaches_fully_satisfied_needs() {
        let mut runtime = PersonRuntime::new();
        let mut allocator = EntityIdAllocator::default();
        let id = runtime.spawn(&mut allocator, coord(1, 2)).expect("spawn");
        assert_eq!(runtime.needs(id), Some(Needs::default()));
    }

    #[test]
    fn needs_update_round_trips_through_stable_identity() {
        let mut runtime = PersonRuntime::new();
        let mut allocator = EntityIdAllocator::default();
        let id = runtime.spawn(&mut allocator, coord(1, 2)).expect("spawn");
        let missing = EntityId::new(999).expect("non-zero identity");

        let one_hour = SimDuration::from_seconds(3_600).expect("non-negative duration");
        let needs = Needs::default().advance(one_hour);
        runtime.set_needs(id, needs).expect("known person");
        assert_eq!(runtime.needs(id), Some(needs));

        assert_eq!(runtime.needs(missing), None);
        assert_eq!(
            runtime.set_needs(missing, needs),
            Err(PersonError::UnknownPerson { id: missing })
        );
        assert_eq!(runtime.needs(id), Some(needs), "state unchanged on error");
    }
}
