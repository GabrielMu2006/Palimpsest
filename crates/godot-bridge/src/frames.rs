// Authored by Kimi Code (AI coding agent) — task CHRON-031 (ADR-0026).
//! Pure frame-conversion layer for the Godot bridge (CHRON-031, ADR-0026).
//!
//! These functions translate the worker's latest published
//! [`RenderSnapshot`] plus [`WorkerStatus`] into plain Rust vectors and values
//! with the exact encodings of ADR-0026. They deliberately avoid Godot
//! container types so the whole conversion — including the lossless
//! full-range `EntityId` little-endian byte encoding — is unit-testable
//! without a running engine; the `#[func]` layer in `lib.rs` only copies
//! these values into Godot packed arrays.

use palimpsest_sim_ai::ActionKind;
use palimpsest_sim_core::{
    ActionState, CommandOutcome, CommandSequence, CommandStatus, RenderSnapshot, SimInstant,
    SpeedMultiplier, WorkerCommand, WorkerPhase, WorkerStatus,
};
use palimpsest_sim_world::{LocalCoord, SiteKind, TerrainKind};

/// One presented person in flat frame form (ADR-0026 §1).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PersonFrame {
    /// Raw `u64` value of the stable `EntityId` (byte-encoded on the wire).
    pub id: u64,
    /// Tile x within the 128×128 local grid.
    pub x: i32,
    /// Tile y within the 128×128 local grid.
    pub y: i32,
    /// Encoded top-level action kind (Idle=0, Move=1, Eat=2, Sleep=3, Work=4).
    pub action: i32,
    /// Encoded observable action state (Idle=0, Moving=1, Eating=2, Sleeping=3, Working=4).
    pub state: i32,
    /// Action target x, or `-1` for `None`.
    pub target_x: i32,
    /// Action target y, or `-1` for `None`.
    pub target_y: i32,
}

/// The flattened, bridge-ready content of one published snapshot plus the
/// worker status at the same read point.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FrameData {
    /// Render DTO schema version.
    pub schema_version: i64,
    /// The committed simulation instant of the snapshot.
    pub sim_second: i64,
    /// Monotonic worker publication count.
    pub publications: i64,
    /// Row-major 128×128 terrain cells (Ground=0, Water=1, Rock=2).
    pub terrain: Vec<u8>,
    /// Flattened site triples `(x, y, kind)` with Meal=0, Rest=1, Work=2.
    pub sites: Vec<(i32, i32, i32)>,
    /// Presented persons, ascending by stable `EntityId` (snapshot order).
    pub persons: Vec<PersonFrame>,
    /// Render metrics, copied verbatim from the snapshot.
    pub metrics: FrameMetrics,
    /// Worker status, copied verbatim.
    pub worker: FrameWorker,
}

/// The schema-2 `RenderMetrics` fields (ADR-0023/0024 D6), as integers.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FrameMetrics {
    /// Number of presented persons.
    pub person_count: i64,
    /// Live scheduler payload depth.
    pub scheduler_queue_depth: i64,
    /// Total committed high-level outcome events.
    pub events_committed: i64,
    /// Currently buffered events.
    pub events_buffered: i64,
    /// Events dropped by either retention buffer.
    pub buffer_rotations: i64,
    /// Persons with a live action execution record.
    pub live_actions: i64,
    /// Total advance rounds.
    pub rounds_total: i64,
    /// Total committed action transitions.
    pub transitions_total: i64,
    /// Total resolved decisions.
    pub decisions_total: i64,
}

/// The worker status fields (ADR-0015 supplement), as integers/strings.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FrameWorker {
    /// Phase: paused=0, running=1, faulted=2, closed=3.
    pub phase: i32,
    /// Speed: 1/5/20/100/1000 as the factor, 0 = MAX (no numeric factor).
    pub speed: i32,
    /// Last committed boundary.
    pub committed: i64,
    /// Publication counter.
    pub publications: i64,
    /// Applied command count.
    pub commands_applied: i64,
    /// Rejected command count.
    pub commands_rejected: i64,
    /// Current command queue depth.
    pub queue_depth: i64,
    /// Maximum observed queue depth.
    pub max_queue_depth: i64,
    /// Whether the worker carries a recorded fault.
    pub faulted: bool,
}

/// Lossless full-range `EntityId` encoding: 8 little-endian bytes per id
/// (ADR-0026 §1). Never routed through `f64` or Godot `int`.
pub fn encode_entity_ids_le(ids: impl IntoIterator<Item = u64>) -> Vec<u8> {
    let ids: Vec<u64> = ids.into_iter().collect();
    let mut bytes = Vec::with_capacity(ids.len() * 8);
    for id in ids {
        bytes.extend_from_slice(&id.to_le_bytes());
    }
    bytes
}

/// Decodes one little-endian 8-byte id; the inverse of
/// [`encode_entity_ids_le`]. Test/diagnostic aid.
#[cfg(test)]
#[must_use]
pub fn decode_entity_id_le(bytes: &[u8]) -> Option<u64> {
    let array: [u8; 8] = bytes.try_into().ok()?;
    Some(u64::from_le_bytes(array))
}

/// The numeric terrain encoding (Ground=0, Water=1, Rock=2).
#[must_use]
pub const fn encode_terrain(kind: TerrainKind) -> u8 {
    match kind {
        TerrainKind::Ground => 0,
        TerrainKind::Water => 1,
        TerrainKind::Rock => 2,
    }
}

/// The numeric site-kind encoding (Meal=0, Rest=1, Work=2).
#[must_use]
pub const fn encode_site_kind(kind: SiteKind) -> i32 {
    match kind {
        SiteKind::Meal => 0,
        SiteKind::Rest => 1,
        SiteKind::Work => 2,
    }
}

/// The numeric action-kind encoding (Idle=0, Move=1, Eat=2, Sleep=3, Work=4).
#[must_use]
pub const fn encode_action(kind: ActionKind) -> i32 {
    match kind {
        ActionKind::Idle => 0,
        ActionKind::Move => 1,
        ActionKind::Eat => 2,
        ActionKind::Sleep => 3,
        ActionKind::Work => 4,
    }
}

/// The numeric action-state encoding (Idle=0, Moving=1, Eating=2, Sleeping=3,
/// Working=4); `Moving` keeps its carried action in the `action` field.
#[must_use]
pub const fn encode_action_state(state: ActionState) -> i32 {
    match state {
        ActionState::Idle => 0,
        ActionState::Moving { .. } => 1,
        ActionState::Eating => 2,
        ActionState::Sleeping => 3,
        ActionState::Working => 4,
    }
}

fn status_to_frame(status: &WorkerStatus) -> FrameWorker {
    FrameWorker {
        phase: match status.phase {
            WorkerPhase::Paused => 0,
            WorkerPhase::Running => 1,
            WorkerPhase::Faulted => 2,
            WorkerPhase::Closed => 3,
        },
        speed: match status.speed.factor() {
            Some(factor) => i32::try_from(factor).unwrap_or(i32::MAX),
            None => 0,
        },
        committed: status.committed.as_seconds(),
        publications: i64::try_from(status.publications).unwrap_or(i64::MAX),
        commands_applied: i64::try_from(status.commands_applied).unwrap_or(i64::MAX),
        commands_rejected: i64::try_from(status.commands_rejected).unwrap_or(i64::MAX),
        queue_depth: i64::try_from(status.queue_depth).unwrap_or(i64::MAX),
        max_queue_depth: i64::try_from(status.max_queue_depth).unwrap_or(i64::MAX),
        faulted: status.fault.is_some(),
    }
}

/// Builds the flat frame from the latest published snapshot and the worker
/// status at the same read point. Copies only; never mutates.
#[must_use]
pub fn frame_from_snapshot(snapshot: &RenderSnapshot, status: &WorkerStatus) -> FrameData {
    let metrics = snapshot.metrics();
    FrameData {
        schema_version: i64::from(snapshot.schema_version()),
        sim_second: snapshot.sim_second().as_seconds(),
        publications: i64::try_from(status.publications).unwrap_or(i64::MAX),
        terrain: snapshot
            .terrain()
            .cells()
            .iter()
            .map(|cell| encode_terrain(*cell))
            .collect(),
        sites: snapshot
            .sites()
            .iter()
            .map(|site| {
                (
                    site.coord().x(),
                    site.coord().y(),
                    encode_site_kind(site.kind()),
                )
            })
            .collect(),
        persons: snapshot
            .persons()
            .iter()
            .map(|person| PersonFrame {
                id: person.person_id().get(),
                x: person.tile().x(),
                y: person.tile().y(),
                action: encode_action(person.action()),
                state: encode_action_state(person.action_state()),
                target_x: person.action_target().map_or(-1, LocalCoord::x),
                target_y: person.action_target().map_or(-1, LocalCoord::y),
            })
            .collect(),
        metrics: FrameMetrics {
            person_count: i64::try_from(metrics.person_count).unwrap_or(i64::MAX),
            scheduler_queue_depth: i64::try_from(metrics.scheduler_queue_depth).unwrap_or(i64::MAX),
            events_committed: i64::try_from(metrics.events_committed).unwrap_or(i64::MAX),
            events_buffered: i64::try_from(metrics.events_buffered).unwrap_or(i64::MAX),
            buffer_rotations: i64::try_from(metrics.buffer_rotations).unwrap_or(i64::MAX),
            live_actions: i64::try_from(metrics.live_actions).unwrap_or(i64::MAX),
            rounds_total: i64::try_from(metrics.rounds_total).unwrap_or(i64::MAX),
            transitions_total: i64::try_from(metrics.transitions_total).unwrap_or(i64::MAX),
            decisions_total: i64::try_from(metrics.decisions_total).unwrap_or(i64::MAX),
        },
        worker: status_to_frame(status),
    }
}

/// Parses one UI command dictionary into a worker command (ADR-0026 §2).
/// Pure validation: nothing reaches the worker on rejection. The error is a
/// stable machine-readable tag for the UI.
pub fn parse_command(command_type: &str, value: i64) -> Result<WorkerCommand, &'static str> {
    match command_type {
        "pause" => Ok(WorkerCommand::Pause),
        "resume" => Ok(WorkerCommand::Resume),
        "shutdown" => Ok(WorkerCommand::Shutdown),
        "step" => {
            let steps = u64::try_from(value).map_err(|_| "invalid_step")?;
            Ok(WorkerCommand::Step(steps))
        }
        "advance_to" => Ok(WorkerCommand::AdvanceTo(SimInstant::from_seconds(value))),
        "set_speed" => {
            if value == 0 {
                return Ok(WorkerCommand::SetSpeed(SpeedMultiplier::Max));
            }
            let factor = u32::try_from(value).map_err(|_| "invalid_speed")?;
            SpeedMultiplier::from_u32(factor)
                .map(WorkerCommand::SetSpeed)
                .map_err(|_| "invalid_speed")
        }
        _ => Err("unknown_command"),
    }
}

/// The wire status string for one command sequence (ADR-0026 §2).
#[must_use]
pub fn command_status_wire(status: &CommandStatus) -> (&'static str, Option<String>, Option<i64>) {
    match status {
        CommandStatus::Unknown => ("unknown", None, None),
        CommandStatus::Pending => ("pending", None, None),
        CommandStatus::Evicted => ("evicted", None, None),
        CommandStatus::Completed(ack) => {
            let outcome = match ack.outcome() {
                CommandOutcome::Applied => "applied".to_string(),
                CommandOutcome::Rejected(error) => format!("rejected:{error}"),
            };
            (
                "completed",
                Some(outcome),
                Some(ack.committed_to().as_seconds()),
            )
        }
    }
}

/// Converts a worker-assigned sequence to the `i64` wire form.
#[must_use]
pub fn sequence_to_wire(sequence: CommandSequence) -> i64 {
    i64::try_from(sequence.get()).unwrap_or(i64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entity_id_byte_encoding_is_lossless_over_the_full_u64_range() {
        let ids = [
            0_u64,
            1,
            42,
            i64::MAX as u64,
            (i64::MAX as u64) + 1,
            u64::MAX,
        ];
        let bytes = encode_entity_ids_le(ids);
        assert_eq!(bytes.len(), ids.len() * 8);
        for (index, id) in ids.iter().enumerate() {
            let decoded = decode_entity_id_le(&bytes[index * 8..(index + 1) * 8]);
            assert_eq!(decoded, Some(*id), "id {id} round-trips losslessly");
        }
        assert_eq!(decode_entity_id_le(&bytes[..7]), None);
    }

    #[test]
    fn command_parsing_validates_before_reaching_the_worker() {
        assert_eq!(parse_command("pause", 0), Ok(WorkerCommand::Pause));
        assert_eq!(parse_command("resume", 0), Ok(WorkerCommand::Resume));
        assert_eq!(parse_command("shutdown", 0), Ok(WorkerCommand::Shutdown));
        assert_eq!(parse_command("step", 7), Ok(WorkerCommand::Step(7)));
        assert_eq!(parse_command("step", -1), Err("invalid_step"));
        assert_eq!(
            parse_command("set_speed", 1000),
            Ok(WorkerCommand::SetSpeed(SpeedMultiplier::X1000))
        );
        assert_eq!(
            parse_command("set_speed", 0),
            Ok(WorkerCommand::SetSpeed(SpeedMultiplier::Max))
        );
        assert_eq!(parse_command("set_speed", 7), Err("invalid_speed"));
        assert_eq!(
            parse_command("advance_to", 86_400),
            Ok(WorkerCommand::AdvanceTo(SimInstant::from_seconds(86_400)))
        );
        assert_eq!(parse_command("teleport", 0), Err("unknown_command"));
    }

    #[test]
    fn terrain_site_action_encodings_match_the_adr() {
        assert_eq!(encode_terrain(TerrainKind::Ground), 0);
        assert_eq!(encode_terrain(TerrainKind::Water), 1);
        assert_eq!(encode_terrain(TerrainKind::Rock), 2);
        assert_eq!(encode_site_kind(SiteKind::Meal), 0);
        assert_eq!(encode_site_kind(SiteKind::Rest), 1);
        assert_eq!(encode_site_kind(SiteKind::Work), 2);
        assert_eq!(encode_action(ActionKind::Idle), 0);
        assert_eq!(encode_action(ActionKind::Move), 1);
        assert_eq!(encode_action(ActionKind::Eat), 2);
        assert_eq!(encode_action(ActionKind::Sleep), 3);
        assert_eq!(encode_action(ActionKind::Work), 4);
        assert_eq!(encode_action_state(ActionState::Idle), 0);
        assert_eq!(
            encode_action_state(ActionState::Moving {
                action: ActionKind::Move
            }),
            1
        );
        assert_eq!(encode_action_state(ActionState::Eating), 2);
        assert_eq!(encode_action_state(ActionState::Sleeping), 3);
        assert_eq!(encode_action_state(ActionState::Working), 4);
    }
}
