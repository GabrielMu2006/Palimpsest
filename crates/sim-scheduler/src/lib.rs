//! Deterministic, event-driven scheduling for simulation work.

use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::num::NonZeroU64;

use palimpsest_sim_time::SimInstant;

/// Opaque runtime handle for canceling or rescheduling queued work.
///
/// This token is local to one scheduler instance and is not a persistent
/// simulation identity.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ScheduleToken(NonZeroU64);

impl ScheduleToken {
    const fn new(raw: u64) -> Option<Self> {
        match NonZeroU64::new(raw) {
            Some(value) => Some(Self(value)),
            None => None,
        }
    }

    /// Returns the scheduler-local numeric token.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0.get()
    }
}

/// Work removed from a scheduler when its due time is reached.
#[derive(Debug)]
pub struct Scheduled<T> {
    token: ScheduleToken,
    due: SimInstant,
    payload: T,
}

impl<T> Scheduled<T> {
    /// Returns the scheduler-local token.
    #[must_use]
    pub const fn token(&self) -> ScheduleToken {
        self.token
    }

    /// Returns the due instant.
    #[must_use]
    pub const fn due(&self) -> SimInstant {
        self.due
    }

    /// Borrows the work payload.
    #[must_use]
    pub const fn payload(&self) -> &T {
        &self.payload
    }

    /// Consumes the scheduled item and returns its payload.
    #[must_use]
    pub fn into_payload(self) -> T {
        self.payload
    }
}

#[derive(Debug)]
struct Entry<T> {
    due: SimInstant,
    order: u64,
    payload: T,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct QueueKey {
    due: SimInstant,
    order: u64,
    token: ScheduleToken,
}

impl Ord for QueueKey {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .due
            .cmp(&self.due)
            .then_with(|| other.order.cmp(&self.order))
    }
}

impl PartialOrd for QueueKey {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Read-only queue health metrics for developer diagnostics.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SchedulerMetrics {
    /// Number of live scheduled payloads.
    pub scheduled_entries: usize,
    /// Number of heap nodes, including lazily invalidated nodes.
    pub queue_nodes: usize,
    /// Number of invalidated nodes awaiting compaction or removal.
    pub stale_nodes: usize,
}

/// Cumulative successful queue operations for read-only diagnostics.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SchedulerCounters {
    /// Newly inserted live payloads.
    pub enqueued: u64,
    /// Live payloads returned to the caller (not stale heap nodes).
    pub dequeued: u64,
    /// Successfully cancelled live payloads.
    pub cancelled: u64,
    /// Successful due-time replacements.
    pub rescheduled: u64,
}

/// Deterministic due-time queue for event-driven simulation work.
///
/// Equal due instants are popped in insertion order. Rescheduling assigns a new
/// insertion order. The scheduler never invokes payload code internally.
#[derive(Debug)]
pub struct Scheduler<T> {
    queue: BinaryHeap<QueueKey>,
    entries: HashMap<ScheduleToken, Entry<T>>,
    next_token_raw: u64,
    next_order_raw: u64,
    counters: SchedulerCounters,
}

impl<T> Scheduler<T> {
    const COMPACT_STALE_FLOOR: usize = 64;

    /// Creates an empty scheduler.
    #[must_use]
    pub fn new() -> Self {
        Self {
            queue: BinaryHeap::new(),
            entries: HashMap::new(),
            next_token_raw: 1,
            next_order_raw: 1,
            counters: SchedulerCounters::default(),
        }
    }

    /// Cumulative successful operations; reading does not mutate queue state.
    #[must_use]
    pub const fn counters(&self) -> SchedulerCounters {
        self.counters
    }

    /// Returns the number of live scheduled payloads.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns whether no live work is scheduled.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Returns queue health metrics without mutating the scheduler.
    #[must_use]
    pub fn metrics(&self) -> SchedulerMetrics {
        SchedulerMetrics {
            scheduled_entries: self.entries.len(),
            queue_nodes: self.queue.len(),
            stale_nodes: self.queue.len().saturating_sub(self.entries.len()),
        }
    }

    /// Returns whether `count` further [`schedule_at`](Scheduler::schedule_at)
    /// calls would succeed by checking the remaining token/order space.
    ///
    /// This is a read-only pre-flight for callers that must reserve several
    /// follow-up tokens before committing a multi-step operation (ADR-0024
    /// D1). It never mutates the queue; between the check and the commit there
    /// is no other inserter on the single thread owning this scheduler, so the
    /// checked count must equal the exact number of schedules the commit will
    /// make. A `count` of `0` always succeeds.
    ///
    /// # Errors
    ///
    /// Returns [`SchedulerError::TokenSpaceExhausted`] when the token counter
    /// cannot cover `count` further allocations, or
    /// [`SchedulerError::OrderSpaceExhausted`] when the insertion-order
    /// counter cannot. Token exhaustion takes precedence.
    pub fn check_schedule_capacity(&self, count: usize) -> Result<(), SchedulerError> {
        if self.remaining_token_slots() < count {
            return Err(SchedulerError::TokenSpaceExhausted);
        }
        if self.remaining_order_slots() < count {
            return Err(SchedulerError::OrderSpaceExhausted);
        }
        Ok(())
    }

    fn remaining_token_slots(&self) -> usize {
        if self.next_token_raw == 0 {
            0
        } else {
            usize::try_from(u64::MAX - self.next_token_raw + 1)
                .expect("u64::MAX - raw + 1 fits usize on the 64-bit target")
        }
    }

    fn remaining_order_slots(&self) -> usize {
        if self.next_order_raw == 0 {
            0
        } else {
            usize::try_from(u64::MAX - self.next_order_raw + 1)
                .expect("u64::MAX - raw + 1 fits usize on the 64-bit target")
        }
    }

    /// Schedules a payload for an absolute simulation instant.
    ///
    /// # Errors
    ///
    /// Returns [`SchedulerError::TokenSpaceExhausted`] or
    /// [`SchedulerError::OrderSpaceExhausted`] before mutating the queue if its
    /// corresponding runtime counter is exhausted.
    pub fn schedule_at(
        &mut self,
        due: SimInstant,
        payload: T,
    ) -> Result<ScheduleToken, SchedulerError> {
        let token =
            ScheduleToken::new(self.next_token_raw).ok_or(SchedulerError::TokenSpaceExhausted)?;
        let order = self.next_order()?;
        let next_token_raw = self.next_token_raw.checked_add(1).unwrap_or(0);

        self.entries.insert(
            token,
            Entry {
                due,
                order,
                payload,
            },
        );
        self.queue.push(QueueKey { due, order, token });
        self.next_token_raw = next_token_raw;
        self.counters.enqueued = self.counters.enqueued.saturating_add(1);
        Ok(token)
    }

    /// Cancels scheduled work, returning whether the token was live.
    pub fn cancel(&mut self, token: ScheduleToken) -> bool {
        let removed = self.entries.remove(&token).is_some();
        if removed {
            self.counters.cancelled = self.counters.cancelled.saturating_add(1);
            self.compact_if_needed();
        }
        removed
    }

    /// Assigns a new due instant and insertion order to live scheduled work.
    ///
    /// Returns `Ok(false)` if `token` is no longer live.
    ///
    /// # Errors
    ///
    /// Returns [`SchedulerError::OrderSpaceExhausted`] without changing the
    /// entry if no further stable insertion order can be assigned.
    pub fn reschedule(
        &mut self,
        token: ScheduleToken,
        due: SimInstant,
    ) -> Result<bool, SchedulerError> {
        if !self.entries.contains_key(&token) {
            return Ok(false);
        }
        let order = self.next_order()?;
        let Some(entry) = self.entries.get_mut(&token) else {
            return Ok(false);
        };
        entry.due = due;
        entry.order = order;
        self.queue.push(QueueKey { due, order, token });
        self.compact_if_needed();
        self.counters.rescheduled = self.counters.rescheduled.saturating_add(1);
        Ok(true)
    }

    /// Returns the earliest live due instant, pruning stale leading nodes.
    pub fn next_due(&mut self) -> Option<SimInstant> {
        self.prune_stale_head();
        self.queue.peek().map(|key| key.due)
    }

    /// Removes and returns the earliest live work due at or before `now`.
    pub fn pop_due(&mut self, now: SimInstant) -> Option<Scheduled<T>> {
        loop {
            let key = *self.queue.peek()?;
            if key.due > now {
                return None;
            }
            self.queue.pop();
            let is_current = self
                .entries
                .get(&key.token)
                .is_some_and(|entry| entry.due == key.due && entry.order == key.order);
            if !is_current {
                continue;
            }
            let Some(entry) = self.entries.remove(&key.token) else {
                continue;
            };
            self.counters.dequeued = self.counters.dequeued.saturating_add(1);
            return Some(Scheduled {
                token: key.token,
                due: entry.due,
                payload: entry.payload,
            });
        }
    }

    /// Rebuilds the heap from live entries, removing all stale nodes.
    pub fn compact(&mut self) {
        self.queue = self
            .entries
            .iter()
            .map(|(&token, entry)| QueueKey {
                due: entry.due,
                order: entry.order,
                token,
            })
            .collect();
    }

    fn next_order(&mut self) -> Result<u64, SchedulerError> {
        if self.next_order_raw == 0 {
            return Err(SchedulerError::OrderSpaceExhausted);
        }
        let order = self.next_order_raw;
        self.next_order_raw = self.next_order_raw.checked_add(1).unwrap_or(0);
        Ok(order)
    }

    fn prune_stale_head(&mut self) {
        while let Some(key) = self.queue.peek().copied() {
            let is_current = self
                .entries
                .get(&key.token)
                .is_some_and(|entry| entry.due == key.due && entry.order == key.order);
            if is_current {
                break;
            }
            self.queue.pop();
        }
    }

    fn compact_if_needed(&mut self) {
        let threshold = self
            .entries
            .len()
            .saturating_mul(2)
            .saturating_add(Self::COMPACT_STALE_FLOOR);
        if self.queue.len() > threshold {
            self.compact();
        }
    }
}

impl<T> Default for Scheduler<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// Failure to assign a scheduler-local token or stable insertion order.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SchedulerError {
    /// Every non-zero `u64` schedule token has been assigned.
    TokenSpaceExhausted,
    /// Every non-zero `u64` insertion order has been assigned.
    OrderSpaceExhausted,
}

impl Display for SchedulerError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::TokenSpaceExhausted => formatter.write_str("schedule token space is exhausted"),
            Self::OrderSpaceExhausted => {
                formatter.write_str("scheduler insertion-order space is exhausted")
            }
        }
    }
}

impl Error for SchedulerError {}

#[cfg(test)]
mod tests {
    use palimpsest_sim_time::SimInstant;

    use super::{Scheduler, SchedulerError};

    #[test]
    fn empty_queue_has_no_due_work() {
        let mut scheduler = Scheduler::<u32>::new();
        assert!(scheduler.is_empty());
        assert_eq!(scheduler.next_due(), None);
        assert!(scheduler.pop_due(SimInstant::MAX).is_none());
    }

    #[test]
    fn work_is_ordered_by_due_time_then_fifo() {
        let mut scheduler = Scheduler::new();
        scheduler
            .schedule_at(SimInstant::from_seconds(20), "late")
            .expect("schedule work");
        scheduler
            .schedule_at(SimInstant::from_seconds(10), "first")
            .expect("schedule work");
        scheduler
            .schedule_at(SimInstant::from_seconds(10), "second")
            .expect("schedule work");

        let now = SimInstant::from_seconds(10);
        assert_eq!(
            scheduler.pop_due(now).map(super::Scheduled::into_payload),
            Some("first")
        );
        assert_eq!(
            scheduler.pop_due(now).map(super::Scheduled::into_payload),
            Some("second")
        );
        assert!(scheduler.pop_due(now).is_none());
        assert_eq!(scheduler.next_due(), Some(SimInstant::from_seconds(20)));
    }

    #[test]
    fn cancellation_removes_work_and_is_idempotent() {
        let mut scheduler = Scheduler::new();
        let token = scheduler
            .schedule_at(SimInstant::from_seconds(5), 1)
            .expect("schedule work");
        assert!(scheduler.cancel(token));
        assert!(!scheduler.cancel(token));
        assert!(scheduler.pop_due(SimInstant::MAX).is_none());
    }

    #[test]
    fn rescheduling_changes_due_time_and_equal_time_order() {
        let mut scheduler = Scheduler::new();
        let moved = scheduler
            .schedule_at(SimInstant::from_seconds(30), "moved")
            .expect("schedule work");
        scheduler
            .schedule_at(SimInstant::from_seconds(10), "original")
            .expect("schedule work");
        assert!(
            scheduler
                .reschedule(moved, SimInstant::from_seconds(10))
                .expect("reschedule work")
        );

        let now = SimInstant::from_seconds(10);
        assert_eq!(
            scheduler.pop_due(now).map(super::Scheduled::into_payload),
            Some("original")
        );
        assert_eq!(
            scheduler.pop_due(now).map(super::Scheduled::into_payload),
            Some("moved")
        );
        assert!(scheduler.pop_due(now).is_none());
    }

    #[test]
    fn canceled_or_popped_tokens_cannot_be_rescheduled() {
        let mut scheduler = Scheduler::new();
        let canceled = scheduler
            .schedule_at(SimInstant::EPOCH, 1)
            .expect("schedule work");
        scheduler.cancel(canceled);
        assert!(
            !scheduler
                .reschedule(canceled, SimInstant::EPOCH)
                .expect("valid counter")
        );

        let popped = scheduler
            .schedule_at(SimInstant::EPOCH, 2)
            .expect("schedule work");
        scheduler.pop_due(SimInstant::EPOCH).expect("due work");
        assert!(
            !scheduler
                .reschedule(popped, SimInstant::EPOCH)
                .expect("valid counter")
        );
    }

    #[test]
    fn caller_controls_reentrant_scheduling() {
        let mut scheduler = Scheduler::new();
        scheduler
            .schedule_at(SimInstant::EPOCH, 40)
            .expect("schedule work");
        let first = scheduler
            .pop_due(SimInstant::EPOCH)
            .expect("due work")
            .into_payload();
        scheduler
            .schedule_at(SimInstant::EPOCH, first + 2)
            .expect("schedule follow-up");
        assert_eq!(
            scheduler
                .pop_due(SimInstant::EPOCH)
                .expect("follow-up work")
                .into_payload(),
            42
        );
    }

    #[test]
    fn stale_nodes_are_compactable_and_metrics_are_exact() {
        let mut scheduler = Scheduler::new();
        let token = scheduler
            .schedule_at(SimInstant::from_seconds(10), 1)
            .expect("schedule work");
        for due in 20..30 {
            scheduler
                .reschedule(token, SimInstant::from_seconds(due))
                .expect("reschedule work");
        }
        assert!(scheduler.metrics().stale_nodes > 0);
        scheduler.compact();
        assert_eq!(scheduler.metrics().scheduled_entries, 1);
        assert_eq!(scheduler.metrics().queue_nodes, 1);
        assert_eq!(scheduler.metrics().stale_nodes, 0);
    }

    #[test]
    fn large_queue_preserves_all_work_without_entity_scans() {
        let mut scheduler = Scheduler::new();
        for value in 0_u32..100_000 {
            let due = SimInstant::from_seconds(i64::from(value % 1_000));
            scheduler.schedule_at(due, value).expect("schedule work");
        }
        let mut popped = 0_usize;
        while scheduler.pop_due(SimInstant::MAX).is_some() {
            popped += 1;
        }
        assert_eq!(popped, 100_000);
        assert!(scheduler.is_empty());
    }

    #[test]
    fn capacity_preflight_covers_zero_one_and_two_requests() {
        // A fresh scheduler must permit at least two further schedules.
        let mut scheduler = Scheduler::<u32>::new();
        assert!(scheduler.check_schedule_capacity(0).is_ok());
        assert!(scheduler.check_schedule_capacity(1).is_ok());
        assert!(scheduler.check_schedule_capacity(2).is_ok());
        // Actually scheduling two then re-checking shows the counter consumed.
        scheduler
            .schedule_at(SimInstant::from_seconds(1), 1)
            .expect("schedule");
        scheduler
            .schedule_at(SimInstant::from_seconds(1), 2)
            .expect("schedule");
        assert!(scheduler.check_schedule_capacity(1).is_ok());
    }

    #[test]
    fn capacity_preflight_reports_exhaustion_without_mutation() {
        // Drive the token counter to its last slot, then confirm a further
        // request is rejected and the queue/metrics are unchanged.
        let mut scheduler = Scheduler::<u32>::new();
        scheduler.next_token_raw = u64::MAX;
        scheduler.next_order_raw = u64::MAX;
        let before = scheduler.metrics();
        assert!(scheduler.check_schedule_capacity(1).is_ok());
        assert_eq!(
            scheduler.check_schedule_capacity(2),
            Err(SchedulerError::TokenSpaceExhausted)
        );
        assert_eq!(scheduler.metrics(), before, "a pre-flight never mutates");

        // Only order space is tight: order exhaustion is reported.
        let mut scheduler = Scheduler::<u32>::new();
        scheduler.next_order_raw = u64::MAX;
        assert_eq!(
            scheduler.check_schedule_capacity(2),
            Err(SchedulerError::OrderSpaceExhausted)
        );

        // Token space is cheap but order is capped at one remaining slot.
        let mut scheduler = Scheduler::<u32>::new();
        scheduler.next_token_raw = 3;
        scheduler.next_order_raw = u64::MAX;
        assert_eq!(
            scheduler.check_schedule_capacity(4),
            Err(SchedulerError::OrderSpaceExhausted)
        );
    }
    #[test]
    fn operation_counters_distinguish_live_payloads_from_stale_nodes() {
        let mut queue = Scheduler::new();
        let first = queue.schedule_at(SimInstant::from_seconds(1), 1).unwrap();
        let second = queue.schedule_at(SimInstant::from_seconds(2), 2).unwrap();
        assert!(
            queue
                .reschedule(first, SimInstant::from_seconds(3))
                .unwrap()
        );
        assert!(queue.cancel(second));
        assert!(!queue.cancel(second));
        assert!(queue.pop_due(SimInstant::from_seconds(2)).is_none());
        assert_eq!(
            queue
                .pop_due(SimInstant::from_seconds(3))
                .unwrap()
                .into_payload(),
            1
        );
        assert!(queue.pop_due(SimInstant::MAX).is_none());
        assert_eq!(
            queue.counters(),
            super::SchedulerCounters {
                enqueued: 2,
                dequeued: 1,
                cancelled: 1,
                rescheduled: 1
            }
        );
    }
}
