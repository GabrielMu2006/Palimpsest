// Authored by Kimi Code (AI coding agent) — task CHRON-030 (ADR-0015 supplement).
//! The Phase 1 simulation worker command bridge (CHRON-030, ADR-0015).
//!
//! [`SimulationWorker`] owns one [`WorldKernel`] (CHRON-028, ADR-0022) on a
//! single dedicated `std` thread and is the only component allowed to mutate
//! simulation state through it. Callers submit bounded [`WorkerCommand`] values
//! (`Pause`/`Resume`/`SetSpeed`/`Step`/`AdvanceTo`/`Shutdown`); each enqueued
//! command carries a monotonic [`CommandSequence`] and eventually produces
//! exactly one [`CommandAck`] recording whether it was applied or rejected and
//! the real committed boundary — a rejected or preempted command can never
//! masquerade as a completed advance. Commands are applied only between kernel
//! calls, so their effects are visible at a complete committed boundary and
//! readers never observe a partial tick.
//!
//! The worker publishes immutable [`RenderSnapshot`] values (CHRON-029,
//! ADR-0023) built strictly from the kernel's committed boundary: the exchange
//! holds a single latest slot (at most two exchange-owned snapshots including
//! the reader's current frame, ADR-0015), the publication sequence is
//! monotonic, and publication is forced on the initial state, `Pause`,
//! non-zero `Step`, non-no-op `AdvanceTo`, and shutdown, and throttled to at
//! most once per 100 ms of wall clock while running. Speed changes wall-clock
//! pacing only — never simulation cadence, weights, or truth — and `MAX`
//! never waits on the wall clock. The worker starts paused.
//!
//! A kernel execution fault moves the worker to [`WorkerPhase::Faulted`]: the
//! cause is exposed, the last complete publication is retained, no new
//! snapshot is built, and advance commands are rejected while
//! `Pause`/`SetSpeed`/`Shutdown` remain available (ADR-0024 D3). Shutdown has
//! an independent atomic stop path that works even when the command queue is
//! full. This module is safe, standard-library-only Rust: no IPC, no separate
//! process, no thread pool, no async runtime, and no multi-threaded ECS.

use std::collections::VecDeque;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, mpsc};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use palimpsest_sim_time::{SimDuration, SimInstant};

use crate::kernel::{KernelError, KernelState, WorldKernel};
use crate::render::RenderSnapshot;

/// Bounded command-queue capacity (ADR-0015 supplement: 64). A saturated
/// queue makes [`SimulationWorker::submit`] return [`WorkerError::Full`];
/// commands are never silently dropped or blocked unboundedly.
pub const COMMAND_QUEUE_CAPACITY: usize = 64;

/// Bounded acknowledgement-log capacity (ADR-0015 supplement: 1,024 latest
/// acks). Older acknowledgements are reported as [`CommandStatus::Evicted`].
pub const ACK_LOG_CAPACITY: usize = 1_024;

/// Maximum simulation seconds one `Step` command may advance
/// (ADR-0015 supplement: 1,000).
pub const MAX_STEP_STEPS: u64 = 1_000;

/// Wall-clock publication throttle while running (the 10 Hz target).
const PUBLISH_INTERVAL: Duration = Duration::from_millis(100);

/// Command poll interval while paused or faulted; bounds the independent
/// stop-flag latency.
const PAUSED_POLL_INTERVAL: Duration = Duration::from_millis(25);

/// Upper bound on one catch-up sleep while running; bounds stop-flag latency.
const RUNNING_SLEEP_CAP: Duration = Duration::from_millis(50);

/// Per-call advance horizon at [`SpeedMultiplier::Max`]; each kernel call is
/// still bounded by the kernel's work budget (ADR-0022).
const MAX_ADVANCE_CHUNK_SECONDS: i64 = 31_536_000;

/// The closed Phase 1 speed set (`MASTER_SPEC` §66, ADR-0015 supplement).
///
/// A multiplier changes wall-clock pacing only; it never changes simulation
/// cadence, weights, or content, and [`Max`](SpeedMultiplier::Max) advances
/// without waiting on the wall clock.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SpeedMultiplier {
    /// One simulated second per wall second.
    X1,
    /// Five simulated seconds per wall second.
    X5,
    /// Twenty simulated seconds per wall second.
    X20,
    /// One hundred simulated seconds per wall second.
    X100,
    /// One thousand simulated seconds per wall second.
    X1000,
    /// Unthrottled: advance as fast as the kernel commits work.
    Max,
}

impl SpeedMultiplier {
    /// The numeric pacing factor, or `None` for [`Max`](SpeedMultiplier::Max).
    #[must_use]
    pub const fn factor(self) -> Option<u64> {
        match self {
            Self::X1 => Some(1),
            Self::X5 => Some(5),
            Self::X20 => Some(20),
            Self::X100 => Some(100),
            Self::X1000 => Some(1000),
            Self::Max => None,
        }
    }

    /// Maps the numeric UI value to the closed set.
    ///
    /// `Max` has no numeric factor and is never produced here.
    ///
    /// # Errors
    ///
    /// Returns [`WorkerError::InvalidSpeed`] for any value other than 1, 5,
    /// 20, 100, or 1,000.
    pub const fn from_u32(value: u32) -> Result<Self, WorkerError> {
        match value {
            1 => Ok(Self::X1),
            5 => Ok(Self::X5),
            20 => Ok(Self::X20),
            100 => Ok(Self::X100),
            1000 => Ok(Self::X1000),
            _ => Err(WorkerError::InvalidSpeed),
        }
    }
}

/// The closed command set accepted by a [`SimulationWorker`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorkerCommand {
    /// Stop wall-driven advancement at the next committed boundary; later
    /// explicit `Step`/`AdvanceTo` commands remain legal.
    Pause,
    /// Resume wall-paced advancement from the exact committed boundary.
    Resume,
    /// Change the pacing multiplier; simulation content is unaffected.
    SetSpeed(SpeedMultiplier),
    /// While paused, advance exactly this many simulation seconds and remain
    /// paused. `0` is a side-effect-free no-op; values above
    /// [`MAX_STEP_STEPS`] are rejected.
    Step(u64),
    /// While paused, advance to an explicit target and remain paused. The
    /// command completes only when the target is actually reached.
    AdvanceTo(SimInstant),
    /// Apply at the next committed boundary, then close the worker.
    Shutdown,
}

/// Identifies one successfully enqueued command (monotonic from 1).
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct CommandSequence(u64);

impl CommandSequence {
    /// Creates a handle from a raw value. Only sequences returned by
    /// [`SimulationWorker::submit`] were ever assigned; any other value
    /// reports [`CommandStatus::Unknown`].
    #[must_use]
    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }

    /// The raw sequence number.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Worker command and lifecycle failures (CHRON-030 API contract).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WorkerError {
    /// The bounded command queue is saturated; nothing was enqueued.
    Full,
    /// The worker is shut down; no further commands are accepted.
    Closed,
    /// A later queued command interrupted an explicit advance at a complete boundary.
    Interrupted,
    /// A speed outside the closed 1/5/20/100/1000/MAX set was requested.
    InvalidSpeed,
    /// A `Step` was zero-valid but invalid here: above [`MAX_STEP_STEPS`], or
    /// submitted while the worker is not paused.
    InvalidStep,
    /// `AdvanceTo` was submitted while the worker is not paused.
    NotPaused,
    /// A requested target earlier than the committed boundary.
    ClockRegression {
        /// The current committed instant.
        current: SimInstant,
        /// The rejected earlier target.
        requested: SimInstant,
    },
    /// `now + steps` would overflow the simulation timeline.
    TickOverflow,
    /// The kernel faulted (or the worker thread failed at startup); advancing
    /// commands are rejected and the last complete publication is retained.
    KernelFaulted,
    /// The kernel holds spawned persons but was never started
    /// ([`WorldKernel::start_world`]); the worker cannot advance it.
    KernelNotStarted,
}

impl Display for WorkerError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Full => formatter.write_str("command queue is full"),
            Self::Closed => formatter.write_str("worker is shut down"),
            Self::Interrupted => formatter.write_str("advance interrupted by a queued command"),
            Self::InvalidSpeed => formatter.write_str("speed must be one of 1/5/20/100/1000/MAX"),
            Self::InvalidStep => {
                formatter.write_str("step requires a paused worker and at most 1,000 seconds")
            }
            Self::NotPaused => formatter.write_str("advance-to requires a paused worker"),
            Self::ClockRegression { current, requested } => write!(
                formatter,
                "simulation time cannot move backward from {current} to {requested}"
            ),
            Self::TickOverflow => formatter.write_str("step target overflows the timeline"),
            Self::KernelFaulted => formatter.write_str("kernel is faulted"),
            Self::KernelNotStarted => formatter.write_str("kernel world has not been started"),
        }
    }
}

impl Error for WorkerError {}

/// The outcome of one enqueued command.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CommandOutcome {
    /// The command took effect at the acknowledged boundary.
    Applied,
    /// The command was rejected with no simulation side effect, or preempted
    /// by shutdown; the ack's committed boundary reports the real state.
    Rejected(WorkerError),
}

/// The final acknowledgement of one enqueued command.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommandAck {
    sequence: CommandSequence,
    command: WorkerCommand,
    outcome: CommandOutcome,
    committed_to: SimInstant,
}

impl CommandAck {
    /// The acknowledged command's sequence.
    #[must_use]
    pub const fn sequence(&self) -> CommandSequence {
        self.sequence
    }

    /// The acknowledged command.
    #[must_use]
    pub const fn command(&self) -> WorkerCommand {
        self.command
    }

    /// Whether the command was applied or rejected.
    #[must_use]
    pub const fn outcome(&self) -> &CommandOutcome {
        &self.outcome
    }

    /// The actual committed boundary after the command was processed. This is
    /// never the requested target unless the target was truly reached.
    #[must_use]
    pub const fn committed_to(&self) -> SimInstant {
        self.committed_to
    }
}

/// The observable status of one submitted command.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CommandStatus {
    /// The sequence was never assigned by this worker.
    Unknown,
    /// The command is enqueued and has not been processed yet.
    Pending,
    /// The command was processed, but its ack fell out of the bounded
    /// [`ACK_LOG_CAPACITY`] window.
    Evicted,
    /// The command was processed; the ack carries the outcome and boundary.
    Completed(CommandAck),
}

/// The worker lifecycle phase.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorkerPhase {
    /// No wall-driven advancement; explicit `Step`/`AdvanceTo` are legal.
    Paused,
    /// Advancing under the current speed's pacing.
    Running,
    /// The kernel faulted; no advancement, last publication retained.
    Faulted,
    /// Shut down; commands are rejected and the last publication is retained.
    Closed,
}

/// A point-in-time read-only worker status for diagnostics and metrics.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkerStatus {
    /// The lifecycle phase.
    pub phase: WorkerPhase,
    /// The current pacing multiplier.
    pub speed: SpeedMultiplier,
    /// The last committed simulation boundary.
    pub committed: SimInstant,
    /// Total snapshots published so far (monotonic publication sequence).
    pub publications: u64,
    /// Commands applied at a committed boundary.
    pub commands_applied: u64,
    /// Commands rejected (or preempted by shutdown).
    pub commands_rejected: u64,
    /// Commands currently queued.
    pub queue_depth: usize,
    /// The largest queue depth observed so far.
    pub max_queue_depth: usize,
    /// The kernel fault cause, when faulted.
    pub fault: Option<KernelError>,
}

/// One immutable publication with matching metadata. Wall time is diagnostic only.
#[derive(Clone, Debug)]
pub struct WorkerPublication {
    /// Monotonic identity of this exact snapshot.
    pub sequence: u64,
    /// Complete immutable simulation view.
    pub snapshot: Arc<RenderSnapshot>,
    /// Construction start; age includes building and waiting for publication.
    pub built_from: Instant,
    /// Time the completed snapshot became available.
    pub published_at: Instant,
}

/// Atomic observation of publication identity and current worker status.
#[derive(Clone, Debug)]
pub struct WorkerObservation {
    /// The newest publication at this read point.
    pub publication: WorkerPublication,
    /// Current status; committed may be newer than the throttled publication.
    pub status: WorkerStatus,
}

struct StatusInner {
    phase: WorkerPhase,
    speed: SpeedMultiplier,
    committed: SimInstant,
    publications: u64,
    commands_applied: u64,
    commands_rejected: u64,
    fault: Option<KernelError>,
}

struct AckLog {
    acks: VecDeque<CommandAck>,
    evicted_through: u64,
}

struct Shared {
    stop: AtomicBool,
    /// Set under the `next_sequence` lock the moment the worker stops serving
    /// the queue; after it is set, no submission can be enqueued without
    /// observing `Closed`, and the final drain rejects everything queued.
    closing: AtomicBool,
    queue_depth: AtomicUsize,
    control_pending: AtomicUsize,
    max_queue_depth: AtomicUsize,
    next_sequence: Mutex<u64>,
    status: Mutex<StatusInner>,
    snapshot: Mutex<Option<WorkerPublication>>,
    acks: Mutex<AckLog>,
}

impl Shared {
    fn new(now: SimInstant) -> Self {
        Self {
            stop: AtomicBool::new(false),
            closing: AtomicBool::new(false),
            queue_depth: AtomicUsize::new(0),
            control_pending: AtomicUsize::new(0),
            max_queue_depth: AtomicUsize::new(0),
            next_sequence: Mutex::new(1),
            status: Mutex::new(StatusInner {
                phase: WorkerPhase::Paused,
                speed: SpeedMultiplier::X1,
                committed: now,
                publications: 0,
                commands_applied: 0,
                commands_rejected: 0,
                fault: None,
            }),
            snapshot: Mutex::new(None),
            acks: Mutex::new(AckLog {
                acks: VecDeque::new(),
                evicted_through: 0,
            }),
        }
    }
}

/// Locks a worker mutex, recovering from poisoning: a poisoned mutex only
/// means the worker thread panicked mid-update, and the diagnostics inside
/// remain readable.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

#[derive(Clone, Copy)]
struct Envelope {
    sequence: u64,
    command: WorkerCommand,
}

/// The pacing target for a numeric multiplier: `anchor` plus the whole
/// simulated seconds earned over `elapsed` wall time. Saturates at the end of
/// the timeline; pacing is wall-derived and never enters simulation truth.
fn pacing_target(anchor: SimInstant, elapsed: Duration, factor: u64) -> SimInstant {
    let seconds = elapsed.as_millis().saturating_mul(u128::from(factor)) / 1_000;
    let seconds = i64::try_from(seconds).unwrap_or(i64::MAX);
    let duration = SimDuration::from_seconds(seconds).unwrap_or(SimDuration::MAX);
    anchor.checked_add(duration).unwrap_or(SimInstant::MAX)
}

/// The wall wait until the next simulated second becomes due under a numeric
/// multiplier, capped so the stop flag stays responsive.
fn catch_up_sleep(
    anchor: SimInstant,
    anchor_wall: Instant,
    factor: u64,
    now: SimInstant,
) -> Duration {
    let lag_seconds = now.as_seconds().saturating_sub(anchor.as_seconds()) + 1;
    let lag = u128::try_from(lag_seconds).unwrap_or(u128::MAX);
    let factor = u128::from(factor);
    let due_millis = lag.saturating_mul(1_000).div_ceil(factor);
    let elapsed = anchor_wall.elapsed().as_millis();
    let remaining = due_millis.saturating_sub(elapsed);
    let remaining = u64::try_from(remaining).unwrap_or(u64::MAX);
    Duration::from_millis(remaining).min(RUNNING_SLEEP_CAP)
}

/// The result of driving the kernel toward an explicit target.
enum DriveOutcome {
    /// The target was reached with the target committed.
    Reached,
    /// The independent stop flag preempted the drive between kernel calls.
    Stopped,
    /// Yielded to a later command at a complete boundary.
    Interrupted,
    /// The kernel faulted; the cause is carried.
    Faulted(KernelError),
}

/// The worker-thread state between kernel calls. Commands are applied and
/// snapshots are published only at these complete committed boundaries.
struct Loop {
    kernel: WorldKernel,
    paused: bool,
    faulted: bool,
    shutdown: bool,
    speed: SpeedMultiplier,
    anchor_wall: Instant,
    anchor_sim: SimInstant,
    last_publish_wall: Instant,
    last_published_sim: SimInstant,
}

impl Loop {
    fn new(kernel: WorldKernel) -> Self {
        let now = kernel.now();
        Self {
            kernel,
            paused: true,
            faulted: false,
            shutdown: false,
            speed: SpeedMultiplier::X1,
            anchor_wall: Instant::now(),
            anchor_sim: now,
            last_publish_wall: Instant::now(),
            last_published_sim: now,
        }
    }

    /// Drives the kernel to `target`, resuming after budget yields and
    /// honouring the independent stop flag between bounded kernel calls.
    fn drive_to(&mut self, target: SimInstant, shared: &Shared) -> DriveOutcome {
        loop {
            if shared.stop.load(Ordering::Relaxed) {
                return DriveOutcome::Stopped;
            }
            if shared.control_pending.load(Ordering::Relaxed) > 0 {
                return DriveOutcome::Interrupted;
            }
            match self.kernel.advance(target) {
                Ok(result) => {
                    self.sync_committed(shared);
                    self.publish_throttled(shared);
                    if result.reached_target() {
                        return DriveOutcome::Reached;
                    }
                    if shared.stop.load(Ordering::Relaxed) {
                        return DriveOutcome::Stopped;
                    }
                }
                Err(error) => return DriveOutcome::Faulted(error),
            }
        }
    }

    fn sync_committed(&self, shared: &Shared) {
        lock(&shared.status).committed = self.kernel.now();
    }

    fn enter_fault(&mut self, shared: &Shared, cause: KernelError) {
        self.faulted = true;
        let mut status = lock(&shared.status);
        status.phase = WorkerPhase::Faulted;
        status.fault = Some(cause);
    }

    /// Publishes unconditionally (initial, pause, explicit advance, shutdown).
    fn publish_forced(&mut self, shared: &Shared) {
        if self.faulted {
            return;
        }
        let built_from = Instant::now();
        if let Ok(snapshot) = RenderSnapshot::from_kernel(&self.kernel) {
            let now = snapshot.sim_second();
            let arc = Arc::new(snapshot);
            let mut status = lock(&shared.status);
            status.publications += 1;
            status.committed = now;
            let sequence = status.publications;
            *lock(&shared.snapshot) = Some(WorkerPublication {
                sequence,
                snapshot: arc,
                built_from,
                published_at: Instant::now(),
            });
            drop(status);
            self.last_published_sim = now;
            self.last_publish_wall = Instant::now();
        }
        // A build failure means the kernel can no longer produce a complete
        // boundary; retain the last complete publication.
    }

    /// Publishes while running when the committed boundary advanced and the
    /// 10 Hz wall-clock throttle permits.
    fn publish_throttled(&mut self, shared: &Shared) {
        if self.faulted {
            return;
        }
        let now = self.kernel.now();
        if now == self.last_published_sim {
            return;
        }
        if self.last_publish_wall.elapsed() < PUBLISH_INTERVAL {
            return;
        }
        self.publish_forced(shared);
    }

    fn record_ack(
        &mut self,
        shared: &Shared,
        sequence: u64,
        command: WorkerCommand,
        outcome: CommandOutcome,
    ) {
        let committed = self.kernel.now();
        let applied = matches!(outcome, CommandOutcome::Applied);
        let ack = CommandAck {
            sequence: CommandSequence(sequence),
            command,
            outcome,
            committed_to: committed,
        };
        {
            let mut status = lock(&shared.status);
            status.committed = committed;
            if applied {
                status.commands_applied += 1;
            } else {
                status.commands_rejected += 1;
            }
        }
        let mut log = lock(&shared.acks);
        if log.acks.len() >= ACK_LOG_CAPACITY
            && let Some(evicted) = log.acks.pop_front()
        {
            log.evicted_through = evicted.sequence.get();
        }
        log.acks.push_back(ack);
    }

    /// Applies one command at the current committed boundary.
    fn apply(&mut self, shared: &Shared, envelope: &Envelope) {
        let command = envelope.command;
        if self.shutdown || shared.stop.load(Ordering::Relaxed) {
            self.record_ack(
                shared,
                envelope.sequence,
                command,
                CommandOutcome::Rejected(WorkerError::Closed),
            );
            return;
        }
        match command {
            WorkerCommand::Pause => {
                self.paused = true;
                lock(&shared.status).phase = if self.faulted {
                    WorkerPhase::Faulted
                } else {
                    WorkerPhase::Paused
                };
                // Publish before the ack: an observed acknowledgement implies
                // the forced refresh is already visible.
                self.publish_forced(shared);
                self.record_ack(shared, envelope.sequence, command, CommandOutcome::Applied);
            }
            WorkerCommand::Resume => {
                let outcome = if self.faulted {
                    CommandOutcome::Rejected(WorkerError::KernelFaulted)
                } else {
                    self.paused = false;
                    self.anchor_wall = Instant::now();
                    self.anchor_sim = self.kernel.now();
                    lock(&shared.status).phase = WorkerPhase::Running;
                    CommandOutcome::Applied
                };
                self.record_ack(shared, envelope.sequence, command, outcome);
            }
            WorkerCommand::SetSpeed(speed) => {
                self.speed = speed;
                lock(&shared.status).speed = speed;
                if !self.paused {
                    self.anchor_wall = Instant::now();
                    self.anchor_sim = self.kernel.now();
                }
                self.record_ack(shared, envelope.sequence, command, CommandOutcome::Applied);
            }
            WorkerCommand::Step(steps) => self.apply_step(shared, envelope.sequence, steps),
            WorkerCommand::AdvanceTo(target) => {
                self.apply_advance_to(shared, envelope.sequence, target);
            }
            WorkerCommand::Shutdown => {
                self.shutdown = true;
                shared.closing.store(true, Ordering::Relaxed);
                self.publish_forced(shared);
                self.record_ack(shared, envelope.sequence, command, CommandOutcome::Applied);
            }
        }
    }

    /// Applies a `Step`: paused-only, bounded, overflow-checked, then driven.
    fn apply_step(&mut self, shared: &Shared, sequence: u64, steps: u64) {
        let command = WorkerCommand::Step(steps);
        if !self.paused || steps > MAX_STEP_STEPS {
            self.record_ack(
                shared,
                sequence,
                command,
                CommandOutcome::Rejected(WorkerError::InvalidStep),
            );
            return;
        }
        if steps == 0 {
            self.record_ack(shared, sequence, command, CommandOutcome::Applied);
            return;
        }
        if self.faulted {
            self.record_ack(
                shared,
                sequence,
                command,
                CommandOutcome::Rejected(WorkerError::KernelFaulted),
            );
            return;
        }
        let target = u64::try_from(self.kernel.now().as_seconds())
            .ok()
            .and_then(|now| now.checked_add(steps))
            .and_then(|total| i64::try_from(total).ok())
            .map(SimInstant::from_seconds);
        let Some(target) = target else {
            self.record_ack(
                shared,
                sequence,
                command,
                CommandOutcome::Rejected(WorkerError::TickOverflow),
            );
            return;
        };
        self.finish_drive(shared, sequence, command, target);
    }

    /// Applies an `AdvanceTo`: paused-only, regression-checked, then driven.
    fn apply_advance_to(&mut self, shared: &Shared, sequence: u64, target: SimInstant) {
        let command = WorkerCommand::AdvanceTo(target);
        let now = self.kernel.now();
        if !self.paused {
            self.record_ack(
                shared,
                sequence,
                command,
                CommandOutcome::Rejected(WorkerError::NotPaused),
            );
            return;
        }
        if target < now {
            self.record_ack(
                shared,
                sequence,
                command,
                CommandOutcome::Rejected(WorkerError::ClockRegression {
                    current: now,
                    requested: target,
                }),
            );
            return;
        }
        if target == now {
            self.record_ack(shared, sequence, command, CommandOutcome::Applied);
            return;
        }
        if self.faulted {
            self.record_ack(
                shared,
                sequence,
                command,
                CommandOutcome::Rejected(WorkerError::KernelFaulted),
            );
            return;
        }
        self.finish_drive(shared, sequence, command, target);
    }

    /// Drives to `target` and records the honest outcome: a budget yield is
    /// resumed, a stop preemption is rejected `Closed` with the real boundary,
    /// and a kernel error faults the worker.
    fn finish_drive(
        &mut self,
        shared: &Shared,
        sequence: u64,
        command: WorkerCommand,
        target: SimInstant,
    ) {
        match self.drive_to(target, shared) {
            DriveOutcome::Reached => {
                // Publish before the ack: an observed acknowledgement implies
                // the target boundary is already published.
                self.publish_forced(shared);
                self.record_ack(shared, sequence, command, CommandOutcome::Applied);
            }
            DriveOutcome::Interrupted => {
                self.publish_forced(shared);
                self.record_ack(
                    shared,
                    sequence,
                    command,
                    CommandOutcome::Rejected(WorkerError::Interrupted),
                );
            }
            DriveOutcome::Stopped => {
                self.record_ack(
                    shared,
                    sequence,
                    command,
                    CommandOutcome::Rejected(WorkerError::Closed),
                );
            }
            DriveOutcome::Faulted(error) => {
                self.enter_fault(shared, error);
                self.record_ack(
                    shared,
                    sequence,
                    command,
                    CommandOutcome::Rejected(WorkerError::KernelFaulted),
                );
            }
        }
    }
}

/// The worker thread body. Publishes the initial snapshot, then serves
/// commands and wall-paced advancement until shutdown.
#[allow(clippy::too_many_lines)]
fn run(
    kernel: WorldKernel,
    receiver: &mpsc::Receiver<Envelope>,
    shared: &Shared,
    init: &mpsc::Sender<Result<(), WorkerError>>,
) {
    let mut state = Loop::new(kernel);
    if state.kernel.state() == KernelState::Faulted {
        state.faulted = true;
    }
    let built_from = Instant::now();
    let Ok(snapshot) = RenderSnapshot::from_kernel(&state.kernel) else {
        let _ = init.send(Err(WorkerError::KernelFaulted));
        return;
    };
    let now = snapshot.sim_second();
    {
        let mut status = lock(&shared.status);
        status.publications = 1;
        status.committed = now;
    }
    *lock(&shared.snapshot) = Some(WorkerPublication {
        sequence: 1,
        snapshot: Arc::new(snapshot),
        built_from,
        published_at: Instant::now(),
    });
    state.last_published_sim = now;
    state.last_publish_wall = Instant::now();
    if init.send(Ok(())).is_err() {
        return;
    }

    let mut disconnected = false;
    while !state.shutdown && !disconnected {
        if shared.stop.load(Ordering::Relaxed) {
            break;
        }
        // Apply every queued command at this complete committed boundary; the
        // independent stop flag preempts between commands.
        loop {
            if state.shutdown || shared.stop.load(Ordering::Relaxed) {
                break;
            }
            match receiver.try_recv() {
                Ok(envelope) => {
                    shared.queue_depth.fetch_sub(1, Ordering::Relaxed);
                    if matches!(
                        envelope.command,
                        WorkerCommand::Pause | WorkerCommand::Shutdown
                    ) {
                        shared.control_pending.fetch_sub(1, Ordering::Relaxed);
                    }
                    state.apply(shared, &envelope);
                }
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => {
                    disconnected = true;
                    break;
                }
            }
        }
        if state.shutdown || disconnected || shared.stop.load(Ordering::Relaxed) {
            break;
        }
        if state.paused || state.faulted {
            match receiver.recv_timeout(PAUSED_POLL_INTERVAL) {
                Ok(envelope) => {
                    shared.queue_depth.fetch_sub(1, Ordering::Relaxed);
                    if matches!(
                        envelope.command,
                        WorkerCommand::Pause | WorkerCommand::Shutdown
                    ) {
                        shared.control_pending.fetch_sub(1, Ordering::Relaxed);
                    }
                    state.apply(shared, &envelope);
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => disconnected = true,
            }
            continue;
        }
        let now = state.kernel.now();
        let target = match state.speed.factor() {
            None => now
                .checked_add(
                    SimDuration::from_seconds(MAX_ADVANCE_CHUNK_SECONDS)
                        .unwrap_or(SimDuration::MAX),
                )
                .unwrap_or(SimInstant::MAX),
            Some(factor) => pacing_target(state.anchor_sim, state.anchor_wall.elapsed(), factor),
        };
        if target > now {
            match state.kernel.advance(target) {
                Ok(_) => {
                    state.sync_committed(shared);
                    state.publish_throttled(shared);
                }
                Err(error) => state.enter_fault(shared, error),
            }
        } else if let Some(factor) = state.speed.factor() {
            let wait = catch_up_sleep(state.anchor_sim, state.anchor_wall, factor, now);
            if !wait.is_zero() {
                std::thread::sleep(wait);
            }
        }
    }

    // Close: under the sequence lock, stop accepting submissions and reject
    // everything still queued; then publish the final boundary when it
    // advanced past the last publication and mark the phase Closed.
    {
        let _guard = lock(&shared.next_sequence);
        shared.closing.store(true, Ordering::Relaxed);
        while let Ok(envelope) = receiver.try_recv() {
            shared.queue_depth.fetch_sub(1, Ordering::Relaxed);
            state.record_ack(
                shared,
                envelope.sequence,
                envelope.command,
                CommandOutcome::Rejected(WorkerError::Closed),
            );
        }
    }
    if !state.faulted && state.kernel.now() > state.last_published_sim {
        state.publish_forced(shared);
    }
    lock(&shared.status).phase = WorkerPhase::Closed;
}

/// The single in-process simulation worker (CHRON-030, ADR-0015 supplement).
///
/// One worker owns one [`WorldKernel`] on a dedicated thread. The handle is
/// the only way to submit commands and read publications; it exposes no
/// mutable kernel access. Dropping the handle requests shutdown and joins the
/// worker thread.
pub struct SimulationWorker {
    shared: Arc<Shared>,
    sender: mpsc::SyncSender<Envelope>,
    thread: Option<JoinHandle<()>>,
}

impl SimulationWorker {
    /// Starts the worker over `kernel`, blocked until the initial snapshot is
    /// published (publication sequence 1). The worker starts paused.
    ///
    /// # Errors
    ///
    /// Returns [`WorkerError::KernelFaulted`] when the kernel is faulted (no
    /// initial complete boundary exists) and [`WorkerError::KernelNotStarted`]
    /// when the kernel holds spawned persons but [`WorldKernel::start_world`]
    /// was never called.
    pub fn new(kernel: WorldKernel) -> Result<Self, WorkerError> {
        if kernel.state() == KernelState::Faulted {
            return Err(WorkerError::KernelFaulted);
        }
        if kernel.state() == KernelState::Setup && kernel.person_count() > 0 {
            return Err(WorkerError::KernelNotStarted);
        }
        let shared = Arc::new(Shared::new(kernel.now()));
        let (sender, receiver) = mpsc::sync_channel(COMMAND_QUEUE_CAPACITY);
        let (init_sender, init_receiver) = mpsc::channel();
        let thread_shared = Arc::clone(&shared);
        let thread = std::thread::Builder::new()
            .name("palimpsest-simulation".to_string())
            .spawn(move || run(kernel, &receiver, &thread_shared, &init_sender))
            .map_err(|_| WorkerError::KernelFaulted)?;
        match init_receiver.recv() {
            Ok(Ok(())) => Ok(Self {
                shared,
                sender,
                thread: Some(thread),
            }),
            Ok(Err(error)) => {
                let _ = thread.join();
                Err(error)
            }
            Err(_) => {
                let _ = thread.join();
                Err(WorkerError::KernelFaulted)
            }
        }
    }

    /// Enqueues a command and returns its sequence.
    ///
    /// Enqueue success is not application: the command takes effect at a
    /// future committed boundary and its eventual outcome is observable
    /// through [`command_status`](SimulationWorker::command_status).
    ///
    /// # Errors
    ///
    /// Returns [`WorkerError::Full`] when the bounded queue is saturated and
    /// [`WorkerError::Closed`] after shutdown. Nothing is enqueued on error.
    pub fn submit(&self, command: WorkerCommand) -> Result<CommandSequence, WorkerError> {
        // Hold the sequence lock across the send so a failed send never
        // consumes a sequence, concurrent submitters stay consistent, and no
        // command can slip into the queue once the worker's final drain has
        // started (the worker sets `closing` under this same lock).
        let mut next = lock(&self.shared.next_sequence);
        if self.shared.closing.load(Ordering::Relaxed) {
            return Err(WorkerError::Closed);
        }
        let sequence = *next;
        // Account the in-flight command before the send so the worker's
        // decrement on receive can never precede its increment.
        let depth = self.shared.queue_depth.fetch_add(1, Ordering::Relaxed) + 1;
        let interrupts = matches!(command, WorkerCommand::Pause | WorkerCommand::Shutdown);
        if interrupts {
            self.shared.control_pending.fetch_add(1, Ordering::Relaxed);
        }
        match self.sender.try_send(Envelope { sequence, command }) {
            Ok(()) => {
                *next += 1;
                // A receive between the increment and the send can inflate
                // `depth` by one beyond true occupancy; occupancy can never
                // exceed the channel capacity, so clamp to it exactly.
                self.shared
                    .max_queue_depth
                    .fetch_max(depth.min(COMMAND_QUEUE_CAPACITY), Ordering::Relaxed);
                Ok(CommandSequence(sequence))
            }
            Err(mpsc::TrySendError::Full(_)) => {
                self.shared.queue_depth.fetch_sub(1, Ordering::Relaxed);
                if interrupts {
                    self.shared.control_pending.fetch_sub(1, Ordering::Relaxed);
                }
                Err(WorkerError::Full)
            }
            Err(mpsc::TrySendError::Disconnected(_)) => {
                self.shared.queue_depth.fetch_sub(1, Ordering::Relaxed);
                if interrupts {
                    self.shared.control_pending.fetch_sub(1, Ordering::Relaxed);
                }
                Err(WorkerError::Closed)
            }
        }
    }

    /// The observable status of one submitted command.
    #[must_use]
    pub fn command_status(&self, sequence: CommandSequence) -> CommandStatus {
        if sequence.get() == 0 {
            return CommandStatus::Unknown;
        }
        if sequence.get() >= *lock(&self.shared.next_sequence) {
            return CommandStatus::Unknown;
        }
        let log = lock(&self.shared.acks);
        if sequence.get() <= log.evicted_through {
            return CommandStatus::Evicted;
        }
        for ack in log.acks.iter().rev() {
            if ack.sequence() == sequence {
                return CommandStatus::Completed(ack.clone());
            }
        }
        CommandStatus::Pending
    }

    /// The newest complete published snapshot at this read point.
    ///
    /// Keeping the returned `Arc` until the next publication is valid;
    /// consumers must not build an unbounded history of handles (ADR-0015).
    ///
    /// # Panics
    ///
    /// Never in practice: [`new`](SimulationWorker::new) returns only after
    /// the initial snapshot was published, and the slot is never cleared.
    #[must_use]
    pub fn latest_snapshot(&self) -> Arc<RenderSnapshot> {
        Arc::clone(
            &lock(&self.shared.snapshot)
                .as_ref()
                .expect("the initial snapshot is published before new returns")
                .snapshot,
        )
    }

    /// Reads a snapshot and its metadata consistently with current status.
    ///
    /// # Panics
    /// Initial publication is guaranteed by successful construction.
    #[must_use]
    pub fn observe(&self) -> WorkerObservation {
        // Every two-lock operation uses status -> snapshot order.
        let status = lock(&self.shared.status);
        let publication = lock(&self.shared.snapshot)
            .as_ref()
            .expect("initial publication precedes construction")
            .clone();
        WorkerObservation {
            publication,
            status: self.status_from(&status),
        }
    }

    fn status_from(&self, status: &StatusInner) -> WorkerStatus {
        WorkerStatus {
            phase: status.phase,
            speed: status.speed,
            committed: status.committed,
            publications: status.publications,
            commands_applied: status.commands_applied,
            commands_rejected: status.commands_rejected,
            queue_depth: self.shared.queue_depth.load(Ordering::Relaxed),
            max_queue_depth: self.shared.max_queue_depth.load(Ordering::Relaxed),
            fault: status.fault.clone(),
        }
    }

    /// A point-in-time worker status read.
    #[must_use]
    pub fn status(&self) -> WorkerStatus {
        let status = lock(&self.shared.status);
        self.status_from(&status)
    }

    /// Whether the worker is paused (no wall-driven advancement).
    #[must_use]
    pub fn is_paused(&self) -> bool {
        lock(&self.shared.status).phase == WorkerPhase::Paused
    }

    /// The current pacing multiplier.
    #[must_use]
    pub fn speed(&self) -> SpeedMultiplier {
        lock(&self.shared.status).speed
    }

    /// Requests shutdown through the independent stop path: it works even
    /// when the command queue is full and never depends on enqueueing another
    /// command. Idempotent.
    pub fn shutdown(&self) {
        self.shared.stop.store(true, Ordering::Relaxed);
    }
}

impl Drop for SimulationWorker {
    fn drop(&mut self) {
        self.shutdown();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CommandOutcome, DriveOutcome, Envelope, Loop, Shared, SpeedMultiplier, WorkerCommand,
        WorkerError, pacing_target,
    };
    use crate::kernel::{KernelError, WorldKernel};
    use crate::{KernelConfig, SimInstant};
    use palimpsest_sim_world::WorldSeed;
    use std::time::Duration;

    fn started_kernel(persons: usize) -> WorldKernel {
        let kernel = WorldKernel::from_world(WorldSeed::new(42), KernelConfig::default());
        fixture_kernel(kernel, persons)
    }

    fn fixture_kernel(mut kernel: WorldKernel, persons: usize) -> WorldKernel {
        let origin = kernel
            .map()
            .local()
            .coords()
            .find(|origin| {
                kernel
                    .map()
                    .local()
                    .get(origin.x(), origin.y())
                    .is_some_and(|kind| kind.is_walkable())
            })
            .expect("generated map has a walkable spawn cell");
        for _ in 0..persons {
            kernel.spawn_person(origin).expect("spawn person");
        }
        kernel.start_world(SimInstant::EPOCH).expect("start world");
        kernel
    }

    fn apply(state: &mut Loop, shared: &Shared, sequence: u64, command: WorkerCommand) {
        state.apply(shared, &Envelope { sequence, command });
    }

    #[test]
    fn pacing_target_maps_the_closed_speed_set_exactly() {
        let anchor = SimInstant::from_seconds(100);
        let elapsed = Duration::from_millis(2_000);
        let cases = [
            (SpeedMultiplier::X1, 102),
            (SpeedMultiplier::X5, 110),
            (SpeedMultiplier::X20, 140),
            (SpeedMultiplier::X100, 300),
            (SpeedMultiplier::X1000, 2_100),
        ];
        for (speed, expected) in cases {
            let factor = speed.factor().expect("numeric speed has a factor");
            assert_eq!(
                pacing_target(anchor, elapsed, factor),
                SimInstant::from_seconds(expected)
            );
        }
        assert_eq!(SpeedMultiplier::Max.factor(), None);
    }

    #[test]
    fn pacing_target_saturates_instead_of_overflowing() {
        let anchor = SimInstant::from_seconds(i64::MAX - 10);
        let target = pacing_target(anchor, Duration::from_hours(24), 1_000);
        assert_eq!(target, SimInstant::MAX);
    }

    #[test]
    fn speed_from_u32_accepts_only_the_closed_numeric_set() {
        for (value, expected) in [
            (1, SpeedMultiplier::X1),
            (5, SpeedMultiplier::X5),
            (20, SpeedMultiplier::X20),
            (100, SpeedMultiplier::X100),
            (1000, SpeedMultiplier::X1000),
        ] {
            assert_eq!(SpeedMultiplier::from_u32(value), Ok(expected));
        }
        for value in [0, 2, 999, 1001, u32::MAX] {
            assert_eq!(
                SpeedMultiplier::from_u32(value),
                Err(WorkerError::InvalidSpeed)
            );
        }
    }

    #[test]
    fn faulted_kernel_rejects_advance_but_keeps_control_commands() {
        let mut kernel = started_kernel(1);
        kernel.force_fault_for_test(KernelError::InvalidBudget, SimInstant::EPOCH);
        let shared = Shared::new(kernel.now());
        let mut state = Loop::new(kernel);
        state.faulted = true;

        for command in [
            WorkerCommand::Resume,
            WorkerCommand::Step(1),
            WorkerCommand::AdvanceTo(SimInstant::from_seconds(10)),
        ] {
            apply(&mut state, &shared, 1, command);
            let log = super::lock(&shared.acks);
            let ack = log.acks.back().expect("ack recorded");
            assert_eq!(
                ack.outcome(),
                &CommandOutcome::Rejected(WorkerError::KernelFaulted)
            );
            drop(log);
        }
        apply(&mut state, &shared, 2, WorkerCommand::Pause);
        let log = super::lock(&shared.acks);
        assert_eq!(
            log.acks.back().expect("ack recorded").outcome(),
            &CommandOutcome::Applied
        );
        drop(log);
        apply(
            &mut state,
            &shared,
            3,
            WorkerCommand::SetSpeed(SpeedMultiplier::X20),
        );
        let log = super::lock(&shared.acks);
        assert_eq!(
            log.acks.back().expect("ack recorded").outcome(),
            &CommandOutcome::Applied
        );
    }

    #[test]
    fn drive_to_stops_at_the_flag_without_reaching_the_target() {
        let kernel = started_kernel(1);
        let shared = Shared::new(kernel.now());
        let mut state = Loop::new(kernel);
        // A zero-sized budget per call is impossible (the kernel rejects it),
        // so pre-set the flag: the first post-call check preempts the drive.
        shared
            .stop
            .store(true, std::sync::atomic::Ordering::Relaxed);
        let target = SimInstant::from_seconds(31_536_000);
        match state.drive_to(target, &shared) {
            DriveOutcome::Reached => panic!("a stopped drive must not report the target reached"),
            DriveOutcome::Stopped => {
                assert!(state.kernel.now() < target);
            }
            DriveOutcome::Faulted(error) => panic!("unexpected fault: {error}"),
            DriveOutcome::Interrupted => panic!("no queued commands"),
        }
        shared
            .stop
            .store(false, std::sync::atomic::Ordering::Relaxed);
    }

    #[test]
    fn new_rejects_faulted_and_not_started_kernels() {
        let mut faulted = started_kernel(1);
        faulted.force_fault_for_test(KernelError::InvalidBudget, SimInstant::EPOCH);
        assert_eq!(
            super::SimulationWorker::new(faulted).map(|_| ()),
            Err(WorkerError::KernelFaulted)
        );

        let mut not_started = WorldKernel::from_world(WorldSeed::new(42), KernelConfig::default());
        let origin = not_started
            .map()
            .local()
            .coords()
            .find(|origin| {
                not_started
                    .map()
                    .local()
                    .get(origin.x(), origin.y())
                    .is_some_and(|kind| kind.is_walkable())
            })
            .expect("walkable spawn cell");
        not_started.spawn_person(origin).expect("spawn person");
        assert_eq!(
            super::SimulationWorker::new(not_started).map(|_| ()),
            Err(WorkerError::KernelNotStarted)
        );
    }

    #[test]
    fn step_zero_is_a_side_effect_free_noop() {
        let kernel = started_kernel(1);
        let shared = Shared::new(kernel.now());
        let mut state = Loop::new(kernel);
        apply(&mut state, &shared, 1, WorkerCommand::Step(0));
        let log = super::lock(&shared.acks);
        let ack = log.acks.back().expect("ack recorded");
        assert_eq!(ack.outcome(), &CommandOutcome::Applied);
        assert_eq!(ack.committed_to(), SimInstant::EPOCH);
        drop(log);
        let status = super::lock(&shared.status);
        assert_eq!(status.publications, 0);
        assert_eq!(status.committed, SimInstant::EPOCH);
    }
    #[test]
    fn shutdown_rejects_commands_already_queued_after_it() {
        let kernel = started_kernel(1);
        let shared = Shared::new(kernel.now());
        let (sender, receiver) = std::sync::mpsc::sync_channel(4);
        for (sequence, command) in [
            (1, WorkerCommand::Shutdown),
            (2, WorkerCommand::Step(10)),
            (3, WorkerCommand::Resume),
        ] {
            sender.send(Envelope { sequence, command }).unwrap();
            shared
                .queue_depth
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            if matches!(command, WorkerCommand::Shutdown) {
                shared
                    .control_pending
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            }
        }
        let (init, initialized) = std::sync::mpsc::channel();
        super::run(kernel, &receiver, &shared, &init);
        assert_eq!(initialized.recv().unwrap(), Ok(()));
        let log = super::lock(&shared.acks);
        assert_eq!(log.acks.len(), 3);
        assert_eq!(log.acks[0].outcome(), &CommandOutcome::Applied);
        for ack in log.acks.iter().skip(1) {
            assert_eq!(
                ack.outcome(),
                &CommandOutcome::Rejected(WorkerError::Closed)
            );
            assert_eq!(ack.committed_to(), SimInstant::EPOCH);
        }
    }

    #[test]
    fn explicit_drive_yields_to_queued_command_without_false_completion() {
        let kernel = started_kernel(1);
        let shared = Shared::new(kernel.now());
        shared
            .control_pending
            .store(1, std::sync::atomic::Ordering::Relaxed);
        let mut state = Loop::new(kernel);
        let command = WorkerCommand::AdvanceTo(SimInstant::from_seconds(31_536_000));
        apply(&mut state, &shared, 1, command);
        let log = super::lock(&shared.acks);
        let ack = log.acks.back().unwrap();
        assert_eq!(
            ack.outcome(),
            &CommandOutcome::Rejected(WorkerError::Interrupted)
        );
        assert_eq!(ack.committed_to(), SimInstant::EPOCH);
        assert_eq!(
            super::lock(&shared.snapshot)
                .as_ref()
                .unwrap()
                .snapshot
                .sim_second(),
            SimInstant::EPOCH
        );
    }
}
