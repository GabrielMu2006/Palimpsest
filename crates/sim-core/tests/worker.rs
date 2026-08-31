// Authored by Kimi Code (AI coding agent) — task CHRON-030 (ADR-0015 supplement).
//! Contract tests for the simulation worker command bridge (CHRON-030).
//!
//! Deterministic cases drive the worker with explicit `Step`/`AdvanceTo`
//! commands only; wall-clock pacing is exercised through loose lower bounds
//! and the exactly-tested pure pacing mapping, never through tight sleep
//! assertions.

use std::sync::Arc;
use std::time::{Duration, Instant};

use palimpsest_sim_core::{
    CommandOutcome, CommandSequence, CommandStatus, KernelConfig, RenderSnapshot, SimInstant,
    SimulationWorker, SpeedMultiplier, WorkerCommand, WorkerError, WorkerPhase, WorldKernel,
};
use palimpsest_sim_world::WorldSeed;

/// Bounded poll for a worker condition; panics with `what` on timeout.
fn wait_for(what: &str, mut condition: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(60);
    while !condition() {
        assert!(Instant::now() < deadline, "timed out waiting for {what}");
        std::thread::yield_now();
    }
}

/// Waits for the final acknowledgement of one command sequence.
fn wait_ack(worker: &SimulationWorker, sequence: CommandSequence) -> CommandOutcome {
    let mut ack = None;
    wait_for(&format!("ack for sequence {}", sequence.get()), || {
        if let CommandStatus::Completed(completed) = worker.command_status(sequence) {
            ack = Some(completed);
            true
        } else {
            false
        }
    });
    ack.expect("ack recorded").outcome().clone()
}

fn build_kernel(persons: usize, seed: u64) -> WorldKernel {
    let mut kernel = WorldKernel::from_world(WorldSeed::new(seed), KernelConfig::default());
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
        kernel.spawn_person(origin).expect("identity capacity");
    }
    kernel.start_world(SimInstant::EPOCH).expect("start world");
    kernel
}

fn start_worker(persons: usize) -> SimulationWorker {
    SimulationWorker::new(build_kernel(persons, 42)).expect("worker starts")
}

#[test]
fn initial_snapshot_is_published_and_worker_is_paused() {
    let worker = start_worker(4);
    assert!(worker.is_paused());
    assert_eq!(worker.speed(), SpeedMultiplier::X1);
    let snapshot = worker.latest_snapshot();
    assert_eq!(snapshot.sim_second(), SimInstant::EPOCH);
    assert_eq!(snapshot.person_count(), 4);
    snapshot.validate().expect("initial snapshot validates");
    let status = worker.status();
    assert_eq!(status.phase, WorkerPhase::Paused);
    assert_eq!(status.publications, 1);
    assert_eq!(status.committed, SimInstant::EPOCH);
}

#[test]
fn paused_worker_does_not_advance_without_commands() {
    let worker = start_worker(4);
    std::thread::sleep(Duration::from_millis(150));
    assert_eq!(worker.status().committed, SimInstant::EPOCH);
    assert_eq!(worker.latest_snapshot().sim_second(), SimInstant::EPOCH);
    assert_eq!(worker.status().publications, 1);
}

#[test]
fn step_advances_exactly_and_stays_paused() {
    let worker = start_worker(4);
    let sequence = worker
        .submit(WorkerCommand::Step(10))
        .expect("enqueue step");
    assert_eq!(wait_ack(&worker, sequence), CommandOutcome::Applied);
    wait_for("forced step publication", || {
        worker.latest_snapshot().sim_second() == SimInstant::from_seconds(10)
    });
    assert!(worker.is_paused());
    let status = worker.status();
    assert_eq!(status.committed, SimInstant::from_seconds(10));
    assert!(status.publications >= 2);
}

#[test]
fn step_zero_is_a_noop_without_publication() {
    let worker = start_worker(4);
    let before = worker.status();
    let sequence = worker.submit(WorkerCommand::Step(0)).expect("enqueue step");
    let ack = wait_ack(&worker, sequence);
    assert_eq!(ack, CommandOutcome::Applied);
    let after = worker.status();
    assert_eq!(after.committed, before.committed);
    assert_eq!(after.publications, before.publications);
}

#[test]
fn step_validation_rejects_oversized_and_unpaused() {
    let worker = start_worker(4);
    let oversized = worker
        .submit(WorkerCommand::Step(1_001))
        .expect("enqueue oversized step");
    assert_eq!(
        wait_ack(&worker, oversized),
        CommandOutcome::Rejected(WorkerError::InvalidStep)
    );

    let resume = worker
        .submit(WorkerCommand::Resume)
        .expect("enqueue resume");
    assert_eq!(wait_ack(&worker, resume), CommandOutcome::Applied);
    let unpaused = worker.submit(WorkerCommand::Step(1)).expect("enqueue step");
    assert_eq!(
        wait_ack(&worker, unpaused),
        CommandOutcome::Rejected(WorkerError::InvalidStep)
    );
    let pause = worker.submit(WorkerCommand::Pause).expect("enqueue pause");
    assert_eq!(wait_ack(&worker, pause), CommandOutcome::Applied);
}

#[test]
fn advance_to_reaches_the_target_and_validates() {
    let worker = start_worker(4);
    let target = SimInstant::from_seconds(600);
    let sequence = worker
        .submit(WorkerCommand::AdvanceTo(target))
        .expect("enqueue advance");
    assert_eq!(wait_ack(&worker, sequence), CommandOutcome::Applied);
    wait_for("forced advance publication", || {
        worker.latest_snapshot().sim_second() == target
    });
    assert!(worker.is_paused());
    assert_eq!(worker.status().committed, target);

    let regression = worker
        .submit(WorkerCommand::AdvanceTo(SimInstant::from_seconds(599)))
        .expect("enqueue regression");
    assert_eq!(
        wait_ack(&worker, regression),
        CommandOutcome::Rejected(WorkerError::ClockRegression {
            current: target,
            requested: SimInstant::from_seconds(599),
        })
    );

    let publications = worker.status().publications;
    let noop = worker
        .submit(WorkerCommand::AdvanceTo(target))
        .expect("enqueue equal-target advance");
    assert_eq!(wait_ack(&worker, noop), CommandOutcome::Applied);
    assert_eq!(worker.status().publications, publications);
}

#[test]
fn advance_to_while_running_is_rejected() {
    let worker = start_worker(4);
    let resume = worker
        .submit(WorkerCommand::Resume)
        .expect("enqueue resume");
    assert_eq!(wait_ack(&worker, resume), CommandOutcome::Applied);
    let advance = worker
        .submit(WorkerCommand::AdvanceTo(SimInstant::from_seconds(60)))
        .expect("enqueue advance");
    assert_eq!(
        wait_ack(&worker, advance),
        CommandOutcome::Rejected(WorkerError::NotPaused)
    );
    let pause = worker.submit(WorkerCommand::Pause).expect("enqueue pause");
    assert_eq!(wait_ack(&worker, pause), CommandOutcome::Applied);
}

#[test]
fn numeric_speed_paces_and_max_does_not_wait() {
    let worker = start_worker(4);
    let set = worker
        .submit(WorkerCommand::SetSpeed(SpeedMultiplier::X1000))
        .expect("enqueue speed");
    assert_eq!(wait_ack(&worker, set), CommandOutcome::Applied);
    let resume = worker
        .submit(WorkerCommand::Resume)
        .expect("enqueue resume");
    assert_eq!(wait_ack(&worker, resume), CommandOutcome::Applied);
    std::thread::sleep(Duration::from_millis(300));
    let pause = worker.submit(WorkerCommand::Pause).expect("enqueue pause");
    assert_eq!(wait_ack(&worker, pause), CommandOutcome::Applied);
    let paced = worker.status().committed.as_seconds();
    // Loose bounds only: pacing is wall-driven and not a reproducible trace.
    assert!(paced >= 50, "1,000x pacing made progress: {paced}");
    assert!(
        paced <= 60_000,
        "1,000x pacing is bounded by wall time: {paced}"
    );

    let set_max = worker
        .submit(WorkerCommand::SetSpeed(SpeedMultiplier::Max))
        .expect("enqueue max speed");
    assert_eq!(wait_ack(&worker, set_max), CommandOutcome::Applied);
    let resume = worker
        .submit(WorkerCommand::Resume)
        .expect("enqueue resume");
    assert_eq!(wait_ack(&worker, resume), CommandOutcome::Applied);
    std::thread::sleep(Duration::from_millis(200));
    let pause = worker.submit(WorkerCommand::Pause).expect("enqueue pause");
    assert_eq!(wait_ack(&worker, pause), CommandOutcome::Applied);
    let maxed = worker.status().committed.as_seconds();
    assert!(
        maxed >= paced + 10_000,
        "MAX advances without waiting for the wall clock: {paced} -> {maxed}"
    );
}

#[test]
fn bounded_queue_reports_full_then_drains() {
    let worker = start_worker(100);
    let busy = worker
        .submit(WorkerCommand::AdvanceTo(SimInstant::from_seconds(86_400)))
        .expect("enqueue long advance");
    let mut queued = 0_usize;
    let mut first_full = None;
    for _ in 0..70 {
        match worker.submit(WorkerCommand::Step(0)) {
            Ok(_) => queued += 1,
            Err(WorkerError::Full) => {
                first_full = Some(());
                break;
            }
            Err(other) => panic!("unexpected submit failure: {other}"),
        }
    }
    assert!(first_full.is_some(), "a saturated queue must report Full");
    // The worker may have received the long advance before the flood, so 64
    // or 65 submissions can succeed against the capacity-64 channel.
    assert!(
        (64..=65).contains(&(queued + 1)),
        "the queue bound is 64 in-flight commands, got {}",
        queued + 1
    );
    assert_eq!(
        wait_ack(&worker, busy),
        CommandOutcome::Applied,
        "the in-flight advance still completes honestly"
    );
    wait_for("queue drains after the advance", || {
        worker.status().queue_depth == 0
    });
    assert_eq!(worker.status().max_queue_depth, 64);
    worker
        .submit(WorkerCommand::Step(0))
        .expect("submit succeeds again after the drain");
}

#[test]
fn shutdown_command_closes_and_later_submissions_fail() {
    let worker = start_worker(4);
    let sequence = worker
        .submit(WorkerCommand::Shutdown)
        .expect("enqueue shutdown");
    assert_eq!(wait_ack(&worker, sequence), CommandOutcome::Applied);
    wait_for("closed phase", || {
        worker.status().phase == WorkerPhase::Closed
    });
    assert_eq!(
        worker.submit(WorkerCommand::Step(1)),
        Err(WorkerError::Closed)
    );
    // The last complete publication remains readable after close.
    let snapshot = worker.latest_snapshot();
    assert_eq!(snapshot.sim_second(), SimInstant::EPOCH);
}

#[test]
fn independent_shutdown_path_works_with_a_full_queue() {
    let worker = start_worker(100);
    let busy = worker
        .submit(WorkerCommand::AdvanceTo(SimInstant::from_seconds(86_400)))
        .expect("enqueue long advance");
    let mut flooded = Vec::new();
    for _ in 0..70 {
        match worker.submit(WorkerCommand::Step(0)) {
            Ok(sequence) => flooded.push(sequence),
            Err(WorkerError::Full) => break,
            Err(other) => panic!("unexpected submit failure: {other}"),
        }
    }
    assert!(!flooded.is_empty(), "queue filled while the advance ran");
    worker.shutdown();
    wait_for("closed phase", || {
        worker.status().phase == WorkerPhase::Closed
    });
    // The in-flight advance completed or was preempted honestly; the queued
    // commands still waiting at the stop were rejected Closed, never dropped.
    let busy_outcome = wait_ack(&worker, busy);
    match busy_outcome {
        CommandOutcome::Applied | CommandOutcome::Rejected(WorkerError::Closed) => {}
        other @ CommandOutcome::Rejected(_) => {
            panic!("unexpected in-flight outcome: {other:?}");
        }
    }
    let rejected = flooded
        .iter()
        .filter(|sequence| {
            wait_ack(&worker, **sequence) == CommandOutcome::Rejected(WorkerError::Closed)
        })
        .count();
    assert!(
        rejected > 0,
        "commands queued behind the stop must be rejected Closed"
    );
    assert_eq!(
        worker.submit(WorkerCommand::Step(1)),
        Err(WorkerError::Closed)
    );
}

#[test]
fn identical_seed_and_command_sequence_reproduce_identical_publications() {
    let script = [
        WorkerCommand::Step(5),
        WorkerCommand::AdvanceTo(SimInstant::from_seconds(900)),
        WorkerCommand::Step(0),
        WorkerCommand::Step(7),
        WorkerCommand::AdvanceTo(SimInstant::from_seconds(3_600)),
    ];
    let run_script = || -> Vec<Arc<RenderSnapshot>> {
        let worker = start_worker(4);
        let mut publications = Vec::new();
        for command in script {
            let sequence = worker.submit(command).expect("enqueue scripted command");
            assert_eq!(
                wait_ack(&worker, sequence),
                CommandOutcome::Applied,
                "scripted command {command:?} applies"
            );
            publications.push(worker.latest_snapshot());
        }
        publications
    };
    let first = run_script();
    let second = run_script();
    assert_eq!(
        first, second,
        "identical inputs reproduce identical snapshots"
    );
    let seconds: Vec<SimInstant> = first.iter().map(|snapshot| snapshot.sim_second()).collect();
    assert_eq!(
        seconds,
        [
            SimInstant::from_seconds(5),
            SimInstant::from_seconds(900),
            SimInstant::from_seconds(900),
            SimInstant::from_seconds(907),
            SimInstant::from_seconds(3_600),
        ]
    );
}

#[test]
fn slow_reader_keeps_a_complete_snapshot_and_never_goes_backwards() {
    let worker = start_worker(4);
    let initial = worker.latest_snapshot();
    let sequence = worker
        .submit(WorkerCommand::Step(500))
        .expect("enqueue step");
    assert_eq!(wait_ack(&worker, sequence), CommandOutcome::Applied);
    wait_for("step publication", || {
        worker.latest_snapshot().sim_second() == SimInstant::from_seconds(500)
    });
    // The retained older handle still reads as its own complete boundary.
    assert_eq!(initial.sim_second(), SimInstant::EPOCH);
    let newer = worker.latest_snapshot();
    assert_eq!(newer.sim_second(), SimInstant::from_seconds(500));
    assert!(!Arc::ptr_eq(&initial, &newer));

    let set_max = worker
        .submit(WorkerCommand::SetSpeed(SpeedMultiplier::Max))
        .expect("enqueue max");
    assert_eq!(wait_ack(&worker, set_max), CommandOutcome::Applied);
    let resume = worker
        .submit(WorkerCommand::Resume)
        .expect("enqueue resume");
    assert_eq!(wait_ack(&worker, resume), CommandOutcome::Applied);
    let mut previous = worker.latest_snapshot().sim_second();
    for _ in 0..20 {
        let current = worker.latest_snapshot();
        current.validate().expect("every publication validates");
        assert!(
            current.sim_second() >= previous,
            "publications never regress"
        );
        previous = current.sim_second();
        std::thread::yield_now();
    }
    let pause = worker.submit(WorkerCommand::Pause).expect("enqueue pause");
    assert_eq!(wait_ack(&worker, pause), CommandOutcome::Applied);
}

#[test]
fn command_status_lifecycle_and_bounded_ack_eviction() {
    let worker = start_worker(4);
    assert_eq!(
        worker.command_status(CommandSequence::new(9_999)),
        CommandStatus::Unknown
    );
    let first = worker.submit(WorkerCommand::Step(1)).expect("enqueue step");
    assert_eq!(wait_ack(&worker, first), CommandOutcome::Applied);
    match worker.command_status(first) {
        CommandStatus::Completed(ack) => {
            assert_eq!(ack.sequence(), first);
            assert_eq!(ack.committed_to(), SimInstant::from_seconds(1));
        }
        other => panic!("expected a completed ack, got {other:?}"),
    }

    // Drive the bounded ack log (1,024) past the first sequence.
    for _ in 0..1_100 {
        let sequence = worker.submit(WorkerCommand::Step(0)).expect("enqueue noop");
        assert_eq!(wait_ack(&worker, sequence), CommandOutcome::Applied);
    }
    assert_eq!(worker.command_status(first), CommandStatus::Evicted);
}

#[test]
fn empty_setup_kernel_may_advance_through_the_worker() {
    let kernel = WorldKernel::from_world(WorldSeed::new(42), KernelConfig::default());
    let worker = SimulationWorker::new(kernel).expect("empty setup kernel starts");
    let sequence = worker.submit(WorkerCommand::Step(5)).expect("enqueue step");
    assert_eq!(wait_ack(&worker, sequence), CommandOutcome::Applied);
    wait_for("step publication", || {
        worker.latest_snapshot().sim_second() == SimInstant::from_seconds(5)
    });
    assert_eq!(worker.latest_snapshot().person_count(), 0);
}

#[test]
fn pause_interrupts_long_explicit_advance_at_a_real_boundary() {
    let worker = start_worker(100);
    let target = SimInstant::from_seconds(315_360_000);
    let advance = worker.submit(WorkerCommand::AdvanceTo(target)).unwrap();
    wait_for("long advance begins", || {
        worker.status().committed > SimInstant::EPOCH
    });
    let pause = worker.submit(WorkerCommand::Pause).unwrap();
    assert_eq!(
        wait_ack(&worker, advance),
        CommandOutcome::Rejected(WorkerError::Interrupted)
    );
    assert_eq!(wait_ack(&worker, pause), CommandOutcome::Applied);
    let observed = worker.observe();
    assert_eq!(observed.status.phase, WorkerPhase::Paused);
    assert!(observed.status.committed > SimInstant::EPOCH);
    assert!(observed.status.committed < target);
    assert_eq!(
        observed.publication.snapshot.sim_second(),
        observed.status.committed
    );
    assert_eq!(observed.publication.sequence, observed.status.publications);
    let next = worker.submit(WorkerCommand::Step(1)).unwrap();
    assert_eq!(wait_ack(&worker, next), CommandOutcome::Applied);
    assert_eq!(
        worker.latest_snapshot().sim_second().as_seconds(),
        observed.status.committed.as_seconds() + 1
    );
}

#[test]
fn publication_and_metadata_remain_paired_during_running_reads() {
    let worker = start_worker(10);
    let speed = worker
        .submit(WorkerCommand::SetSpeed(SpeedMultiplier::Max))
        .unwrap();
    assert_eq!(wait_ack(&worker, speed), CommandOutcome::Applied);
    let resume = worker.submit(WorkerCommand::Resume).unwrap();
    assert_eq!(wait_ack(&worker, resume), CommandOutcome::Applied);
    let until = Instant::now() + Duration::from_secs(60);
    let mut sequence = 0;
    let mut now = SimInstant::EPOCH;
    while sequence < 3 {
        assert!(
            Instant::now() < until,
            "worker failed to publish complete progress"
        );
        let observed = worker.observe();
        assert_eq!(observed.publication.sequence, observed.status.publications);
        assert!(observed.publication.sequence >= sequence);
        assert!(observed.publication.snapshot.sim_second() >= now);
        assert!(observed.publication.snapshot.sim_second() <= observed.status.committed);
        assert!(observed.publication.built_from <= observed.publication.published_at);
        assert!(observed.publication.published_at <= Instant::now());
        sequence = observed.publication.sequence;
        now = observed.publication.snapshot.sim_second();
        std::thread::yield_now();
    }
    assert!(sequence > 1, "the test must observe an actual publication");
}
