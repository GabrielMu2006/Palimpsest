//! Shared deterministic workload used only for Phase 0 mode comparison.

use std::error::Error;
use std::fmt::{self, Display, Formatter};

use serde::Serialize;

use crate::{EntityId, EntityIdAllocator, EventId, EventRecord, Scheduler, SimClock, SimInstant};

/// Machine-readable result of one finite Phase 0 workload run.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SpikeRunMetrics {
    /// Requested dummy entities.
    pub entities: u64,
    /// Final simulation second.
    pub final_sim_second: i64,
    /// Work items processed.
    pub processed_work: u64,
    /// Structured events generated.
    pub generated_events: u64,
    /// Live scheduler entries after completion.
    pub remaining_scheduled: usize,
}

#[derive(Clone, Copy, Debug)]
struct DummyWork {
    entity_id: EntityId,
}

/// Executes the finite deterministic Phase 0 comparison workload.
///
/// # Errors
///
/// Returns [`SpikeRunError`] for invalid limits, identity exhaustion,
/// scheduling failure, or event construction failure.
pub fn run_spike_workload(
    entity_count: u64,
    final_sim_second: i64,
) -> Result<SpikeRunMetrics, SpikeRunError> {
    if final_sim_second < 0 {
        return Err(SpikeRunError::NegativeFinalTime(final_sim_second));
    }
    let mut allocator = EntityIdAllocator::default();
    let mut scheduler = Scheduler::new();
    let cadence = u64::try_from(final_sim_second)
        .map_err(|_| SpikeRunError::NegativeFinalTime(final_sim_second))?
        .saturating_add(1);

    for index in 0..entity_count {
        let entity_id = allocator
            .allocate()
            .map_err(|_| SpikeRunError::EntityIdExhausted)?;
        let due_u64 = index % cadence;
        let due = i64::try_from(due_u64).map_err(|_| SpikeRunError::TimeOutOfRange(due_u64))?;
        scheduler
            .schedule_at(SimInstant::from_seconds(due), DummyWork { entity_id })
            .map_err(|_| SpikeRunError::SchedulerExhausted)?;
    }

    let mut clock = SimClock::default();
    let target = SimInstant::from_seconds(final_sim_second);
    let mut processed_work = 0_u64;
    while let Some(due) = scheduler.next_due() {
        if due > target {
            break;
        }
        clock
            .advance_to(due)
            .map_err(|_| SpikeRunError::ClockAdvance)?;
        while let Some(item) = scheduler.pop_due(clock.now()) {
            processed_work = processed_work
                .checked_add(1)
                .ok_or(SpikeRunError::EventIdExhausted)?;
            let event_id = EventId::new(processed_work).ok_or(SpikeRunError::EventIdExhausted)?;
            let mut event = EventRecord::new(event_id, clock.now(), "dummy_update")
                .map_err(|_| SpikeRunError::InvalidEvent)?;
            event
                .add_actor(item.payload().entity_id)
                .map_err(|_| SpikeRunError::InvalidEvent)?;
            event.validate().map_err(|_| SpikeRunError::InvalidEvent)?;
        }
    }
    clock
        .advance_to(target)
        .map_err(|_| SpikeRunError::ClockAdvance)?;
    Ok(SpikeRunMetrics {
        entities: entity_count,
        final_sim_second: clock.now().as_seconds(),
        processed_work,
        generated_events: processed_work,
        remaining_scheduled: scheduler.len(),
    })
}

/// Finite Phase 0 workload failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SpikeRunError {
    /// Requested final time was negative.
    NegativeFinalTime(i64),
    /// A generated due time could not fit the timeline.
    TimeOutOfRange(u64),
    /// Persistent identity space exhausted.
    EntityIdExhausted,
    /// Scheduler runtime counters exhausted.
    SchedulerExhausted,
    /// Event identity space exhausted.
    EventIdExhausted,
    /// Clock rejected advancement.
    ClockAdvance,
    /// Structured event failed validation.
    InvalidEvent,
}

impl Display for SpikeRunError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(formatter, "Phase 0 spike workload failed: {self:?}")
    }
}

impl Error for SpikeRunError {}

#[cfg(test)]
mod tests {
    use super::{SpikeRunError, run_spike_workload};

    #[test]
    fn deterministic_fixture_finishes_and_drains_queue() {
        let metrics = run_spike_workload(10_000, 1_000).expect("valid finite run");
        assert_eq!(metrics.entities, 10_000);
        assert_eq!(metrics.final_sim_second, 1_000);
        assert_eq!(metrics.processed_work, 10_000);
        assert_eq!(metrics.generated_events, 10_000);
        assert_eq!(metrics.remaining_scheduled, 0);
    }

    #[test]
    fn invalid_time_is_rejected() {
        assert_eq!(
            run_spike_workload(1, -1),
            Err(SpikeRunError::NegativeFinalTime(-1))
        );
    }
}
