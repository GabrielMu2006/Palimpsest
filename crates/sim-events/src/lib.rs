//! Versioned structured events for simulation truth and persistence prototypes.

use std::collections::{BTreeMap, HashSet};
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::num::NonZeroU64;

use palimpsest_sim_entity::EntityId;
use palimpsest_sim_time::SimInstant;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Current structured-event schema version.
pub const EVENT_SCHEMA_VERSION: u16 = 1;

/// Stable persistent identity of one event.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct EventId(NonZeroU64);

impl EventId {
    /// Creates an ID when `raw` is non-zero.
    #[must_use]
    pub const fn new(raw: u64) -> Option<Self> {
        match NonZeroU64::new(raw) {
            Some(value) => Some(Self(value)),
            None => None,
        }
    }

    /// Returns its numeric representation.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0.get()
    }
}

/// Visibility within world-internal information systems.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Visibility {
    /// Observable without a restricted source.
    Public,
    /// Known only through limited observations or sources.
    Restricted,
    /// Simulation truth not directly observed in-world.
    Hidden,
}

/// Bounded historical significance from 0 through 1,000.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(try_from = "u16", into = "u16")]
pub struct SignificanceScore(u16);

impl SignificanceScore {
    /// Minimum significance.
    pub const MIN: Self = Self(0);
    /// Maximum significance.
    pub const MAX: Self = Self(1_000);
    /// Returns the numeric score.
    #[must_use]
    pub const fn get(self) -> u16 {
        self.0
    }
}

impl TryFrom<u16> for SignificanceScore {
    type Error = InvalidSignificanceScore;
    fn try_from(value: u16) -> Result<Self, Self::Error> {
        (value <= Self::MAX.0)
            .then_some(Self(value))
            .ok_or(InvalidSignificanceScore(value))
    }
}

impl From<SignificanceScore> for u16 {
    fn from(value: SignificanceScore) -> Self {
        value.get()
    }
}

/// A versioned, structured simulation-truth event.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(try_from = "EventRecordWire")]
pub struct EventRecord {
    schema_version: u16,
    event_id: EventId,
    timestamp: SimInstant,
    event_type: String,
    actors: Vec<EntityId>,
    targets: Vec<EntityId>,
    location: Option<EntityId>,
    causes: Vec<EventId>,
    consequences: Vec<EventId>,
    visibility: Visibility,
    significance: SignificanceScore,
    metadata: BTreeMap<String, Value>,
}

impl EventRecord {
    /// Creates a minimal schema-version-1 event.
    ///
    /// # Errors
    /// Returns [`EventValidationError::EmptyEventType`] for a blank type.
    pub fn new(
        event_id: EventId,
        timestamp: SimInstant,
        event_type: impl Into<String>,
    ) -> Result<Self, EventValidationError> {
        let record = Self {
            schema_version: EVENT_SCHEMA_VERSION,
            event_id,
            timestamp,
            event_type: event_type.into(),
            actors: Vec::new(),
            targets: Vec::new(),
            location: None,
            causes: Vec::new(),
            consequences: Vec::new(),
            visibility: Visibility::Hidden,
            significance: SignificanceScore::MIN,
            metadata: BTreeMap::new(),
        };
        record.validate()?;
        Ok(record)
    }

    /// Returns the ID.
    #[must_use]
    pub const fn event_id(&self) -> EventId {
        self.event_id
    }
    /// Returns the timestamp.
    #[must_use]
    pub const fn timestamp(&self) -> SimInstant {
        self.timestamp
    }
    /// Returns the event-type key.
    #[must_use]
    pub fn event_type(&self) -> &str {
        &self.event_type
    }
    /// Returns actors.
    #[must_use]
    pub fn actors(&self) -> &[EntityId] {
        &self.actors
    }
    /// Returns targets.
    #[must_use]
    pub fn targets(&self) -> &[EntityId] {
        &self.targets
    }
    /// Returns causes.
    #[must_use]
    pub fn causes(&self) -> &[EventId] {
        &self.causes
    }
    /// Returns metadata.
    #[must_use]
    pub const fn metadata(&self) -> &BTreeMap<String, Value> {
        &self.metadata
    }

    /// Adds a unique actor.
    ///
    /// # Errors
    /// Returns an error for a duplicate.
    pub fn add_actor(&mut self, actor: EntityId) -> Result<(), EventValidationError> {
        if self.actors.contains(&actor) {
            return Err(EventValidationError::DuplicateActor(actor));
        }
        self.actors.push(actor);
        Ok(())
    }

    /// Adds a unique target.
    ///
    /// # Errors
    /// Returns an error for a duplicate.
    pub fn add_target(&mut self, target: EntityId) -> Result<(), EventValidationError> {
        if self.targets.contains(&target) {
            return Err(EventValidationError::DuplicateTarget(target));
        }
        self.targets.push(target);
        Ok(())
    }

    /// Adds a unique, non-self cause.
    ///
    /// # Errors
    /// Returns an error for self-causality or a duplicate.
    pub fn add_cause(&mut self, cause: EventId) -> Result<(), EventValidationError> {
        if cause == self.event_id {
            return Err(EventValidationError::SelfCause);
        }
        if self.causes.contains(&cause) {
            return Err(EventValidationError::DuplicateCause(cause));
        }
        self.causes.push(cause);
        Ok(())
    }

    /// Sets location.
    pub fn set_location(&mut self, value: Option<EntityId>) {
        self.location = value;
    }
    /// Sets visibility.
    pub fn set_visibility(&mut self, value: Visibility) {
        self.visibility = value;
    }
    /// Sets significance.
    pub fn set_significance(&mut self, value: SignificanceScore) {
        self.significance = value;
    }
    /// Inserts structured metadata.
    pub fn insert_metadata(&mut self, key: impl Into<String>, value: Value) -> Option<Value> {
        self.metadata.insert(key.into(), value)
    }

    /// Validates schema and reference invariants.
    ///
    /// # Errors
    /// Returns the first schema, type, duplicate, or self-reference violation.
    pub fn validate(&self) -> Result<(), EventValidationError> {
        if self.schema_version != EVENT_SCHEMA_VERSION {
            return Err(EventValidationError::UnsupportedSchemaVersion(
                self.schema_version,
            ));
        }
        if self.event_type.trim().is_empty() {
            return Err(EventValidationError::EmptyEventType);
        }
        unique_entities(&self.actors, EventValidationError::DuplicateActor)?;
        unique_entities(&self.targets, EventValidationError::DuplicateTarget)?;
        unique_events(&self.causes, EventValidationError::DuplicateCause)?;
        unique_events(
            &self.consequences,
            EventValidationError::DuplicateConsequence,
        )?;
        if self.causes.contains(&self.event_id) {
            return Err(EventValidationError::SelfCause);
        }
        if self.consequences.contains(&self.event_id) {
            return Err(EventValidationError::SelfConsequence);
        }
        Ok(())
    }
}

#[derive(Deserialize)]
struct EventRecordWire {
    schema_version: u16,
    event_id: EventId,
    timestamp: SimInstant,
    event_type: String,
    actors: Vec<EntityId>,
    targets: Vec<EntityId>,
    location: Option<EntityId>,
    causes: Vec<EventId>,
    consequences: Vec<EventId>,
    visibility: Visibility,
    significance: SignificanceScore,
    metadata: BTreeMap<String, Value>,
}

impl TryFrom<EventRecordWire> for EventRecord {
    type Error = EventValidationError;
    fn try_from(w: EventRecordWire) -> Result<Self, Self::Error> {
        let record = Self {
            schema_version: w.schema_version,
            event_id: w.event_id,
            timestamp: w.timestamp,
            event_type: w.event_type,
            actors: w.actors,
            targets: w.targets,
            location: w.location,
            causes: w.causes,
            consequences: w.consequences,
            visibility: w.visibility,
            significance: w.significance,
            metadata: w.metadata,
        };
        record.validate()?;
        Ok(record)
    }
}

fn unique_entities(
    values: &[EntityId],
    error: impl Fn(EntityId) -> EventValidationError,
) -> Result<(), EventValidationError> {
    let mut seen = HashSet::with_capacity(values.len());
    for &value in values {
        if !seen.insert(value) {
            return Err(error(value));
        }
    }
    Ok(())
}

fn unique_events(
    values: &[EventId],
    error: impl Fn(EventId) -> EventValidationError,
) -> Result<(), EventValidationError> {
    let mut seen = HashSet::with_capacity(values.len());
    for &value in values {
        if !seen.insert(value) {
            return Err(error(value));
        }
    }
    Ok(())
}

/// Invalid significance.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvalidSignificanceScore(u16);
impl Display for InvalidSignificanceScore {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "significance {} exceeds maximum 1000", self.0)
    }
}
impl Error for InvalidSignificanceScore {}

/// Event schema or reference violation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EventValidationError {
    /// Unknown schema.
    UnsupportedSchemaVersion(u16),
    /// Missing type.
    EmptyEventType,
    /// Duplicate actor.
    DuplicateActor(EntityId),
    /// Duplicate target.
    DuplicateTarget(EntityId),
    /// Duplicate cause.
    DuplicateCause(EventId),
    /// Duplicate consequence.
    DuplicateConsequence(EventId),
    /// Self cause.
    SelfCause,
    /// Self consequence.
    SelfConsequence,
}
impl Display for EventValidationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "invalid structured event: {self:?}")
    }
}
impl Error for EventValidationError {}

#[cfg(test)]
mod tests {
    use super::{EventId, EventRecord, EventValidationError, SignificanceScore, Visibility};
    use palimpsest_sim_entity::EntityId;
    use palimpsest_sim_time::SimInstant;
    use serde_json::json;
    fn entity(raw: u64) -> EntityId {
        EntityId::new(raw).expect("non-zero")
    }
    fn event(raw: u64) -> EventId {
        EventId::new(raw).expect("non-zero")
    }

    #[test]
    fn representative_event_round_trips() {
        let mut value =
            EventRecord::new(event(2), SimInstant::from_seconds(42), "migration").expect("valid");
        value.add_actor(entity(10)).expect("unique");
        value.add_target(entity(11)).expect("unique");
        value.add_cause(event(1)).expect("valid");
        value.set_location(Some(entity(99)));
        value.set_visibility(Visibility::Restricted);
        value.set_significance(SignificanceScore::try_from(700).expect("bounded"));
        value.insert_metadata("distance_tiles", json!(27));
        let bytes = serde_json::to_vec(&value).expect("serialize");
        assert_eq!(
            serde_json::from_slice::<EventRecord>(&bytes).expect("deserialize"),
            value
        );
        assert!(bytes.len() < 512);
    }

    #[test]
    fn identifiers_scores_and_references_are_validated() {
        assert_eq!(EventId::new(0), None);
        assert!(SignificanceScore::try_from(1_001).is_err());
        let mut value = EventRecord::new(event(2), SimInstant::EPOCH, "test").expect("valid");
        value.add_actor(entity(1)).expect("unique");
        assert_eq!(
            value.add_actor(entity(1)),
            Err(EventValidationError::DuplicateActor(entity(1)))
        );
        assert_eq!(
            value.add_cause(event(2)),
            Err(EventValidationError::SelfCause)
        );
    }

    #[test]
    fn deserialization_rejects_duplicate_references() {
        let value = json!({"schema_version":1,"event_id":2,"timestamp":0,"event_type":"test",
            "actors":[1,1],"targets":[],"location":null,"causes":[],"consequences":[],
            "visibility":"hidden","significance":0,"metadata":{}});
        assert!(serde_json::from_value::<EventRecord>(value).is_err());
    }
}
