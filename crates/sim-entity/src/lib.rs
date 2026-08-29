//! Stable identity primitives for persistent simulation entities.
//!
//! [`EntityId`] is the only entity identity allowed to cross persistence,
//! event, snapshot, or client boundaries. Runtime ECS handles are deliberately
//! absent from this crate.

use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::num::{NonZeroU64, ParseIntError};
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// Stable, persistent identity for a simulation-domain entity.
///
/// Zero is reserved as an invalid/sentinel value. The numeric representation
/// is serialized directly so it remains independent of any runtime ECS.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct EntityId(NonZeroU64);

impl EntityId {
    /// Lowest valid persistent entity ID.
    pub const MIN: Self = Self(NonZeroU64::MIN);

    /// Highest valid persistent entity ID.
    pub const MAX: Self = Self(NonZeroU64::MAX);

    /// Creates an ID when `raw` is non-zero.
    #[must_use]
    pub const fn new(raw: u64) -> Option<Self> {
        match NonZeroU64::new(raw) {
            Some(value) => Some(Self(value)),
            None => None,
        }
    }

    /// Returns the stable numeric representation.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0.get()
    }
}

impl Display for EntityId {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        self.get().fmt(formatter)
    }
}

impl From<EntityId> for u64 {
    fn from(id: EntityId) -> Self {
        id.get()
    }
}

impl TryFrom<u64> for EntityId {
    type Error = InvalidEntityId;

    fn try_from(raw: u64) -> Result<Self, Self::Error> {
        Self::new(raw).ok_or(InvalidEntityId)
    }
}

impl FromStr for EntityId {
    type Err = ParseEntityIdError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let raw = value
            .parse::<u64>()
            .map_err(ParseEntityIdError::InvalidInteger)?;
        Self::new(raw).ok_or(ParseEntityIdError::ReservedZero)
    }
}

/// Error returned when constructing an [`EntityId`] from zero.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvalidEntityId;

impl Display for InvalidEntityId {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("entity ID zero is reserved")
    }
}

impl Error for InvalidEntityId {}

/// Error returned when parsing an [`EntityId`] from text.
#[derive(Debug)]
pub enum ParseEntityIdError {
    /// The input is not an unsigned 64-bit integer.
    InvalidInteger(ParseIntError),
    /// The input is zero, which is reserved.
    ReservedZero,
}

impl Display for ParseEntityIdError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidInteger(error) => write!(formatter, "invalid entity ID: {error}"),
            Self::ReservedZero => formatter.write_str("entity ID zero is reserved"),
        }
    }
}

impl Error for ParseEntityIdError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidInteger(error) => Some(error),
            Self::ReservedZero => None,
        }
    }
}

/// Monotonic allocator for stable entity IDs.
///
/// Allocator state is serializable so snapshots can resume without reusing an
/// ID. A serialized `next_raw` value of zero represents permanent exhaustion.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EntityIdAllocator {
    next_raw: u64,
}

impl EntityIdAllocator {
    /// Creates an allocator whose next allocation is `next`.
    #[must_use]
    pub const fn from_next(next: EntityId) -> Self {
        Self {
            next_raw: next.get(),
        }
    }

    /// Returns the next ID without allocating it, or `None` after exhaustion.
    #[must_use]
    pub const fn next(&self) -> Option<EntityId> {
        EntityId::new(self.next_raw)
    }

    /// Allocates the next stable ID.
    ///
    /// The allocator becomes exhausted after returning [`EntityId::MAX`].
    ///
    /// # Errors
    ///
    /// Returns [`EntityIdAllocationError`] when the complete non-zero `u64`
    /// identity space has already been allocated.
    pub fn allocate(&mut self) -> Result<EntityId, EntityIdAllocationError> {
        let id = self.next().ok_or(EntityIdAllocationError)?;
        self.next_raw = self.next_raw.checked_add(1).unwrap_or(0);
        Ok(id)
    }

    /// Advances allocation beyond an ID restored from persistent state.
    pub fn advance_past(&mut self, existing: EntityId) {
        if self.next().is_some_and(|next| next <= existing) {
            self.next_raw = existing.get().checked_add(1).unwrap_or(0);
        }
    }
}

impl Default for EntityIdAllocator {
    fn default() -> Self {
        Self::from_next(EntityId::MIN)
    }
}

/// Error returned after all valid entity IDs have been allocated.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EntityIdAllocationError;

impl Display for EntityIdAllocationError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("entity ID space is exhausted")
    }
}

impl Error for EntityIdAllocationError {}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::mem::size_of;

    use serde::{Deserialize, Serialize};

    use super::{
        EntityId, EntityIdAllocationError, EntityIdAllocator, InvalidEntityId, ParseEntityIdError,
    };

    #[test]
    fn entity_id_is_exactly_one_u64() {
        assert_eq!(size_of::<EntityId>(), size_of::<u64>());
    }

    #[test]
    fn zero_is_always_rejected() {
        assert_eq!(EntityId::new(0), None);
        assert_eq!(EntityId::try_from(0), Err(InvalidEntityId));
        assert!(matches!(
            "0".parse::<EntityId>(),
            Err(ParseEntityIdError::ReservedZero)
        ));
    }

    #[test]
    fn text_round_trip_preserves_numeric_identity() {
        let id = EntityId::new(81_271).expect("test ID is non-zero");
        assert_eq!(id.to_string(), "81271");
        assert_eq!(id.to_string().parse::<EntityId>().expect("valid ID"), id);
        assert!(matches!(
            "person-81271".parse::<EntityId>(),
            Err(ParseEntityIdError::InvalidInteger(_))
        ));
    }

    #[test]
    fn serde_uses_a_plain_non_zero_number() {
        let id = EntityId::new(81_271).expect("test ID is non-zero");
        let encoded = serde_json::to_string(&id).expect("serialize ID");
        assert_eq!(encoded, "81271");
        assert_eq!(
            serde_json::from_str::<EntityId>(&encoded).expect("deserialize ID"),
            id
        );
        assert!(serde_json::from_str::<EntityId>("0").is_err());
    }

    #[test]
    fn allocator_is_monotonic_and_unique() {
        let mut allocator = EntityIdAllocator::default();
        let ids: Vec<_> = (0..10_000)
            .map(|_| allocator.allocate().expect("ID space available"))
            .collect();

        assert_eq!(ids.first().copied(), Some(EntityId::MIN));
        assert_eq!(ids.last().map(|id| id.get()), Some(10_000));
        assert!(ids.windows(2).all(|pair| pair[0] < pair[1]));
    }

    #[test]
    fn allocator_state_round_trip_prevents_reuse() {
        let mut allocator = EntityIdAllocator::default();
        let allocated = allocator.allocate().expect("ID space available");
        let encoded = serde_json::to_string(&allocator).expect("serialize allocator");
        let mut restored: EntityIdAllocator =
            serde_json::from_str(&encoded).expect("deserialize allocator");

        assert_eq!(allocated, EntityId::MIN);
        assert_eq!(restored.allocate().expect("ID space available").get(), 2);
    }

    #[test]
    fn allocator_can_advance_beyond_restored_entities() {
        let mut allocator = EntityIdAllocator::default();
        allocator.advance_past(EntityId::new(9_999).expect("test ID is non-zero"));
        assert_eq!(
            allocator.allocate().expect("ID space available").get(),
            10_000
        );
    }

    #[test]
    fn allocator_exhaustion_is_explicit() {
        let mut allocator = EntityIdAllocator::from_next(EntityId::MAX);
        assert_eq!(allocator.allocate(), Ok(EntityId::MAX));
        assert_eq!(allocator.allocate(), Err(EntityIdAllocationError));
        assert_eq!(allocator.next(), None);
    }

    #[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
    struct FakeRuntimeHandle {
        index: u32,
        generation: u32,
    }

    #[derive(Debug)]
    struct RuntimeBinding {
        persistent: EntityId,
        runtime: FakeRuntimeHandle,
    }

    #[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
    struct PersistedEntityReference {
        entity_id: EntityId,
    }

    #[test]
    fn runtime_handles_are_not_the_persistence_contract() {
        let persistent = EntityId::new(42).expect("test ID is non-zero");
        let runtime = FakeRuntimeHandle {
            index: 7,
            generation: 3,
        };
        let binding = RuntimeBinding {
            persistent,
            runtime,
        };
        let runtime_lookup = HashMap::from([(binding.persistent, binding.runtime)]);
        let persisted = PersistedEntityReference {
            entity_id: binding.persistent,
        };

        assert_eq!(runtime_lookup.get(&persistent), Some(&runtime));
        assert_eq!(
            serde_json::to_string(&persisted).expect("serialize persistent reference"),
            r#"{"entity_id":42}"#
        );
    }
}
