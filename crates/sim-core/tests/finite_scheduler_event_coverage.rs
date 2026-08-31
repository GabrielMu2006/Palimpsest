//! Preserve the finite queue/event assertions when the production spike is removed.
use palimpsest_sim_core::{
    EntityIdAllocator, EventId, EventRecord, Scheduler, SimClock, SimInstant,
};
#[test]
fn finite_queue_finishes_with_ten_thousand_validated_events() {
    let mut ids = EntityIdAllocator::default();
    let mut scheduler = Scheduler::new();
    let target = SimInstant::from_seconds(1000);
    for _ in 0..10000 {
        scheduler
            .schedule_at(target, ids.allocate().unwrap())
            .unwrap();
    }
    assert_eq!(scheduler.len(), 10000);
    let mut clock = SimClock::default();
    clock.advance_to(target).unwrap();
    let mut processed = 0_u64;
    let mut generated = 0;
    while let Some(item) = scheduler.pop_due(clock.now()) {
        processed += 1;
        let mut event = EventRecord::new(
            EventId::new(processed).unwrap(),
            clock.now(),
            "finite_coverage",
        )
        .unwrap();
        event.add_actor(*item.payload()).unwrap();
        event.validate().unwrap();
        generated += 1;
    }
    assert_eq!(clock.now().as_seconds(), 1000);
    assert_eq!(processed, 10000);
    assert_eq!(generated, 10000);
    assert_eq!(scheduler.len(), 0);
    assert!(scheduler.next_due().is_none());
}
