use std::collections::HashSet;
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::io::Cursor;

use palimpsest_sim_entity::{EntityId, EntityIdAllocator};
use palimpsest_sim_time::{SimClock, SimInstant};
use serde::{Deserialize, Serialize};

const MAGIC: &[u8; 8] = b"PLMSNP01";
const SNAPSHOT_SCHEMA_VERSION: u16 = 1;

/// Minimal persistent entity state used by the Phase 0 snapshot spike.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EntitySnapshot {
    /// Stable persistent identity.
    pub entity_id: EntityId,
    /// Dummy component value for round-trip measurement.
    pub state: u64,
}

/// Persistable scheduled-work representation without runtime tokens or heap nodes.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PendingWorkSnapshot {
    /// Stable target identity.
    pub entity_id: EntityId,
    /// Absolute due instant.
    pub due: SimInstant,
    /// Stable work-kind key.
    pub work_type: String,
}

/// Versioned domain snapshot independent of ECS runtime handles.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Snapshot {
    schema_version: u16,
    /// Simulation clock.
    pub clock: SimClock,
    /// Persistent identity allocator progress.
    pub entity_allocator: EntityIdAllocator,
    /// Stable entity state.
    pub entities: Vec<EntitySnapshot>,
    /// Stable pending work, reconstructed into a Scheduler on restore.
    pub pending_work: Vec<PendingWorkSnapshot>,
}

impl Snapshot {
    /// Creates and validates a schema-version-1 snapshot.
    ///
    /// # Errors
    /// Returns [`SnapshotError`] for duplicate IDs, dangling work, blank work
    /// types, or allocator state that could reuse a restored ID.
    pub fn new(
        clock: SimClock,
        entity_allocator: EntityIdAllocator,
        entities: Vec<EntitySnapshot>,
        pending_work: Vec<PendingWorkSnapshot>,
    ) -> Result<Self, SnapshotError> {
        let value = Self {
            schema_version: SNAPSHOT_SCHEMA_VERSION,
            clock,
            entity_allocator,
            entities,
            pending_work,
        };
        value.validate()?;
        Ok(value)
    }

    /// Validates stable-identity and schema invariants.
    ///
    /// # Errors
    /// Returns the first invariant violation.
    pub fn validate(&self) -> Result<(), SnapshotError> {
        if self.schema_version != SNAPSHOT_SCHEMA_VERSION {
            return Err(SnapshotError::UnsupportedVersion(self.schema_version));
        }
        let mut ids = HashSet::with_capacity(self.entities.len());
        for entity in &self.entities {
            if !ids.insert(entity.entity_id) {
                return Err(SnapshotError::DuplicateEntity(entity.entity_id));
            }
        }
        for work in &self.pending_work {
            if !ids.contains(&work.entity_id) {
                return Err(SnapshotError::DanglingWork(work.entity_id));
            }
            if work.work_type.trim().is_empty() {
                return Err(SnapshotError::EmptyWorkType);
            }
        }
        if let Some(maximum) = self.entities.iter().map(|entity| entity.entity_id).max()
            && self
                .entity_allocator
                .next()
                .is_some_and(|next| next <= maximum)
        {
            return Err(SnapshotError::AllocatorWouldReuse(maximum));
        }
        Ok(())
    }
}

/// Binary bincode + zstd snapshot codec.
pub struct SnapshotCodec;

impl SnapshotCodec {
    /// Encodes and zstd-compresses a validated snapshot.
    ///
    /// # Errors
    /// Returns [`SnapshotError`] for validation, binary encoding, or compression failures.
    pub fn encode(snapshot: &Snapshot) -> Result<Vec<u8>, SnapshotError> {
        snapshot.validate()?;
        let raw = bincode::serde::encode_to_vec(snapshot, bincode::config::standard())?;
        let compressed = zstd::stream::encode_all(Cursor::new(raw), 3)?;
        let mut output = Vec::with_capacity(MAGIC.len() + compressed.len());
        output.extend_from_slice(MAGIC);
        output.extend_from_slice(&compressed);
        Ok(output)
    }

    /// Decompresses, decodes, and validates a snapshot.
    ///
    /// # Errors
    /// Returns [`SnapshotError`] for magic, corruption, version, decoding, or validation failures.
    pub fn decode(bytes: &[u8]) -> Result<Snapshot, SnapshotError> {
        let compressed = bytes
            .strip_prefix(MAGIC)
            .ok_or(SnapshotError::InvalidMagic)?;
        let raw = zstd::stream::decode_all(Cursor::new(compressed))?;
        let (snapshot, consumed): (Snapshot, usize) =
            bincode::serde::decode_from_slice(&raw, bincode::config::standard())?;
        if consumed != raw.len() {
            return Err(SnapshotError::TrailingBytes);
        }
        snapshot.validate()?;
        Ok(snapshot)
    }
}

/// Snapshot encoding, corruption, version, or invariant failure.
#[derive(Debug)]
pub enum SnapshotError {
    /// Invalid file magic.
    InvalidMagic,
    /// Unsupported schema.
    UnsupportedVersion(u16),
    /// Duplicate stable entity.
    DuplicateEntity(EntityId),
    /// Pending work references no entity.
    DanglingWork(EntityId),
    /// Blank work type.
    EmptyWorkType,
    /// Allocator could reuse an existing ID.
    AllocatorWouldReuse(EntityId),
    /// Extra decoded bytes.
    TrailingBytes,
    /// Bincode encode failure.
    Encode(bincode::error::EncodeError),
    /// Bincode decode failure.
    Decode(bincode::error::DecodeError),
    /// Compression I/O failure.
    Io(std::io::Error),
}

impl From<bincode::error::EncodeError> for SnapshotError {
    fn from(value: bincode::error::EncodeError) -> Self {
        Self::Encode(value)
    }
}
impl From<bincode::error::DecodeError> for SnapshotError {
    fn from(value: bincode::error::DecodeError) -> Self {
        Self::Decode(value)
    }
}
impl From<std::io::Error> for SnapshotError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}
impl Display for SnapshotError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "snapshot error: {self:?}")
    }
}
impl Error for SnapshotError {}

#[cfg(test)]
mod tests {
    use super::{EntitySnapshot, PendingWorkSnapshot, Snapshot, SnapshotCodec};
    use palimpsest_sim_entity::{EntityId, EntityIdAllocator};
    use palimpsest_sim_time::{SimClock, SimInstant};

    #[test]
    fn compressed_round_trip_excludes_runtime_handles() {
        let id = EntityId::new(1).expect("non-zero");
        let mut allocator = EntityIdAllocator::default();
        allocator.advance_past(id);
        let snapshot = Snapshot::new(
            SimClock::at(SimInstant::from_seconds(42)),
            allocator,
            vec![EntitySnapshot {
                entity_id: id,
                state: 7,
            }],
            vec![PendingWorkSnapshot {
                entity_id: id,
                due: SimInstant::from_seconds(50),
                work_type: "dummy".to_owned(),
            }],
        )
        .expect("valid");
        let bytes = SnapshotCodec::encode(&snapshot).expect("encode");
        assert_eq!(SnapshotCodec::decode(&bytes).expect("decode"), snapshot);
    }

    #[test]
    fn corruption_and_dangling_work_are_rejected() {
        assert!(SnapshotCodec::decode(b"not-a-snapshot").is_err());
        let id = EntityId::new(1).expect("non-zero");
        assert!(
            Snapshot::new(
                SimClock::default(),
                EntityIdAllocator::default(),
                Vec::new(),
                vec![PendingWorkSnapshot {
                    entity_id: id,
                    due: SimInstant::EPOCH,
                    work_type: "dummy".to_owned()
                }]
            )
            .is_err()
        );
    }
}
