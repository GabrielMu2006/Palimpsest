// Authored by Kimi Code (AI coding agent) — task CHRON-022.
//! Bounded integer needs: the hunger and fatigue drives of a Phase 1 person.
//!
//! All quantities are fixed-point integers (`NEED_SCALE` raw units per 1.0 of
//! drive): no float type appears anywhere in this model, so NaN, drift, and
//! cross-platform divergence are impossible by construction. Values are
//! clamped to `[0, NEED_MAX]` on every operation and change only through an
//! explicit `advance(elapsed)` or an explicit action (`eat`/`rest`) — never
//! implicitly per tick (ADR-0003, ADR-0013; Master Spec §13/§76).

use serde::{Deserialize, Serialize};

use palimpsest_sim_time::SimDuration;

/// Fixed-point scale: raw units per 1.0 of drive.
pub const NEED_SCALE: i64 = 1_000;
/// Maximum raw drive value (`100.0` in fixed point; `0` is fully satisfied).
pub const NEED_MAX: i64 = 100_000;
/// Upper bound of the pressure signal: pressures are reported in
/// `0..=PRESSURE_MAX`.
pub const PRESSURE_MAX: i64 = 1_000;
/// Pressure at or above which [`Needs::is_critical`] holds (90%).
pub const CRITICAL_PRESSURE: i64 = 900;
/// Hunger growth per elapsed second (full drive in ~27.8 h). A Phase 1
/// tuning constant, not a design invariant.
pub const HUNGER_RATE_PER_SECOND: i64 = 1;
/// Fatigue growth per elapsed second (full drive in ~13.9 h). A Phase 1
/// tuning constant, not a design invariant.
pub const FATIGUE_RATE_PER_SECOND: i64 = 2;

/// One bounded drive quantity: an integer in `[0, NEED_MAX]`.
///
/// Serde is a plain integer; out-of-range values are rejected on
/// deserialization.
#[derive(
    Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize,
)]
#[serde(try_from = "i64", into = "i64")]
pub struct NeedValue(i64);

impl NeedValue {
    /// Fully satisfied.
    pub const MIN: Self = Self(0);
    /// Maximum drive.
    pub const MAX: Self = Self(NEED_MAX);

    /// Creates a value when `raw` is within `[0, NEED_MAX]`.
    #[must_use]
    pub fn from_raw(raw: i64) -> Option<Self> {
        (0..=NEED_MAX).contains(&raw).then_some(Self(raw))
    }

    /// The raw fixed-point value.
    #[must_use]
    pub const fn raw(self) -> i64 {
        self.0
    }

    /// Saturating shift clamped into `[0, NEED_MAX]`; never overflows or
    /// underflows.
    fn shifted(self, delta: i64) -> Self {
        Self(self.0.saturating_add(delta).clamp(0, NEED_MAX))
    }
}

impl From<NeedValue> for i64 {
    fn from(value: NeedValue) -> Self {
        value.raw()
    }
}

impl TryFrom<i64> for NeedValue {
    type Error = NeedValueError;

    fn try_from(raw: i64) -> Result<Self, Self::Error> {
        Self::from_raw(raw).ok_or(NeedValueError { raw })
    }
}

/// Error returned when constructing a [`NeedValue`] outside `[0, NEED_MAX]`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NeedValueError {
    raw: i64,
}

impl core::fmt::Display for NeedValueError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            formatter,
            "need value out of range [0, {NEED_MAX}]: {}",
            self.raw
        )
    }
}

impl std::error::Error for NeedValueError {}

/// The two Phase 1 drives of a person: hunger and fatigue (ADR-0013).
///
/// `Needs::default()` is fully satisfied (both drives at
/// [`NeedValue::MIN`]). No other drives exist in Phase 1, and no personality
/// or trait weighting is applied here.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub struct Needs {
    hunger: NeedValue,
    fatigue: NeedValue,
}

impl Needs {
    /// Creates needs from two validated drive values.
    #[must_use]
    pub const fn new(hunger: NeedValue, fatigue: NeedValue) -> Self {
        Self { hunger, fatigue }
    }

    /// Advances both drives by their deterministic per-second rates.
    ///
    /// Integer-exact and saturating: `advance(a)` then `advance(b)` equals
    /// `advance(a + b)`, and a huge `elapsed` simply clamps at
    /// [`NeedValue::MAX`]. `SimDuration` is non-negative by construction
    /// (ADR-0003), so no negative-elapsed branch exists.
    #[must_use]
    pub fn advance(self, elapsed: SimDuration) -> Self {
        let seconds = elapsed.as_seconds();
        Self {
            hunger: self
                .hunger
                .shifted(HUNGER_RATE_PER_SECOND.saturating_mul(seconds)),
            fatigue: self
                .fatigue
                .shifted(FATIGUE_RATE_PER_SECOND.saturating_mul(seconds)),
        }
    }

    /// Reduces hunger by `amount`, saturating at zero.
    ///
    /// Returns the new needs and the amount actually consumed. A
    /// non-positive `amount` is a documented no-op (consumed = 0). Eating is
    /// driven by the action state machine (CHRON-027), never implicitly.
    #[must_use]
    pub fn eat(self, amount: i64) -> (Self, i64) {
        let amount = amount.max(0);
        let consumed = self.hunger.raw().min(amount);
        (
            Self {
                hunger: self.hunger.shifted(-consumed),
                ..self
            },
            consumed,
        )
    }

    /// Reduces fatigue by `amount`, with the same contract as
    /// [`Needs::eat`].
    #[must_use]
    pub fn rest(self, amount: i64) -> (Self, i64) {
        let amount = amount.max(0);
        let consumed = self.fatigue.raw().min(amount);
        (
            Self {
                fatigue: self.fatigue.shifted(-consumed),
                ..self
            },
            consumed,
        )
    }

    /// The hunger drive.
    #[must_use]
    pub const fn hunger(self) -> NeedValue {
        self.hunger
    }

    /// The fatigue drive.
    #[must_use]
    pub const fn fatigue(self) -> NeedValue {
        self.fatigue
    }

    /// Bounded hunger pressure in `0..=PRESSURE_MAX` for Utility AI
    /// weighting (CHRON-026).
    #[must_use]
    pub fn hunger_pressure(self) -> i64 {
        self.hunger.raw() * PRESSURE_MAX / NEED_MAX
    }

    /// Bounded fatigue pressure in `0..=PRESSURE_MAX`.
    #[must_use]
    pub fn fatigue_pressure(self) -> i64 {
        self.fatigue.raw() * PRESSURE_MAX / NEED_MAX
    }

    /// Whether any drive is at or above [`CRITICAL_PRESSURE`].
    #[must_use]
    pub fn is_critical(self) -> bool {
        self.hunger_pressure() >= CRITICAL_PRESSURE || self.fatigue_pressure() >= CRITICAL_PRESSURE
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CRITICAL_PRESSURE, FATIGUE_RATE_PER_SECOND, HUNGER_RATE_PER_SECOND, NEED_MAX, NeedValue,
        Needs, PRESSURE_MAX,
    };
    use palimpsest_sim_time::SimDuration;

    fn seconds(value: i64) -> SimDuration {
        SimDuration::from_seconds(value).expect("non-negative duration")
    }

    fn needs_with(hunger: i64, fatigue: i64) -> Needs {
        Needs::new(
            NeedValue::from_raw(hunger).expect("in range"),
            NeedValue::from_raw(fatigue).expect("in range"),
        )
    }

    #[test]
    fn default_is_fully_satisfied() {
        let needs = Needs::default();
        assert_eq!(needs.hunger(), NeedValue::MIN);
        assert_eq!(needs.fatigue(), NeedValue::MIN);
        assert!(!needs.is_critical());
        assert_eq!(needs.hunger_pressure(), 0);
        assert_eq!(needs.fatigue_pressure(), 0);
    }

    #[test]
    fn advance_is_monotonic_and_zero_is_a_noop() {
        let mut needs = Needs::default();
        let mut previous = (0_i64, 0_i64);
        for step in [0_i64, 1, 59, 60, 3_600, 86_400] {
            needs = needs.advance(seconds(step));
            let current = (needs.hunger().raw(), needs.fatigue().raw());
            assert!(current.0 >= previous.0 && current.1 >= previous.1);
            previous = current;
        }
        let still = needs.advance(SimDuration::ZERO);
        assert_eq!(still, needs);
    }

    #[test]
    fn advance_saturates_without_overflow() {
        let needs = Needs::default().advance(seconds(i64::MAX));
        assert_eq!(needs.hunger(), NeedValue::MAX);
        assert_eq!(needs.fatigue(), NeedValue::MAX);
        assert!(needs.is_critical());
    }

    #[test]
    fn fixed_point_advance_is_exact_and_associative() {
        let whole = Needs::default().advance(seconds(3_600));
        let mut per_second = Needs::default();
        for _ in 0..3_600 {
            per_second = per_second.advance(seconds(1));
        }
        assert_eq!(whole, per_second);

        let split = Needs::default()
            .advance(seconds(1_234))
            .advance(seconds(5_678));
        let joined = Needs::default().advance(seconds(1_234 + 5_678));
        assert_eq!(split, joined);
        assert_eq!(
            whole.hunger().raw(),
            HUNGER_RATE_PER_SECOND * 3_600,
            "hunger grows at exactly the documented rate"
        );
        assert_eq!(whole.fatigue().raw(), FATIGUE_RATE_PER_SECOND * 3_600);
    }

    #[test]
    fn eat_and_rest_clamp_without_underflow() {
        let needs = needs_with(100, 50);
        let (after, consumed) = needs.eat(250);
        assert_eq!(after.hunger(), NeedValue::MIN);
        assert_eq!(consumed, 100, "only the available drive is consumed");

        let (after, consumed) = needs.rest(i64::MAX);
        assert_eq!(after.fatigue(), NeedValue::MIN);
        assert_eq!(consumed, 50);

        let (unchanged, consumed) = needs.eat(-5);
        assert_eq!(unchanged, needs);
        assert_eq!(consumed, 0, "a non-positive amount is a no-op");
    }

    #[test]
    fn pressure_is_bounded_and_critical_threshold_is_documented() {
        let half = needs_with(NEED_MAX / 2, 0);
        assert_eq!(half.hunger_pressure(), PRESSURE_MAX / 2);
        assert!(!half.is_critical());

        let at_threshold = needs_with(NEED_MAX * CRITICAL_PRESSURE / PRESSURE_MAX, 0);
        assert_eq!(at_threshold.hunger_pressure(), CRITICAL_PRESSURE);
        assert!(at_threshold.is_critical());

        let just_below = needs_with(NEED_MAX * CRITICAL_PRESSURE / PRESSURE_MAX - 1, 0);
        assert!(!just_below.is_critical());

        let full = needs_with(NEED_MAX, NEED_MAX);
        assert_eq!(full.hunger_pressure(), PRESSURE_MAX);
        assert_eq!(full.fatigue_pressure(), PRESSURE_MAX);
    }

    #[test]
    fn determinism_repeats_exactly() {
        let mut one = needs_with(1_000, 2_000);
        let mut two = needs_with(1_000, 2_000);
        for step in [3_600_i64, 60, 9_999] {
            one = one.advance(seconds(step));
            two = two.advance(seconds(step));
        }
        assert_eq!(one, two);
        let (one_eaten, one_consumed) = one.eat(12_345);
        let (two_eaten, two_consumed) = two.eat(12_345);
        assert_eq!(one_eaten, two_eaten);
        assert_eq!(one_consumed, two_consumed);
    }

    #[test]
    fn serde_round_trips_and_rejects_out_of_range() {
        let needs = needs_with(12_345, 67_890);
        let encoded = serde_json::to_string(&needs).expect("serialize needs");
        assert_eq!(encoded, "{\"hunger\":12345,\"fatigue\":67890}");
        let restored: Needs = serde_json::from_str(&encoded).expect("deserialize needs");
        assert_eq!(restored, needs);

        let value = NeedValue::from_raw(1_000).expect("in range");
        let encoded = serde_json::to_string(&value).expect("serialize value");
        assert_eq!(encoded, "1000");
        assert_eq!(
            serde_json::from_str::<NeedValue>(&encoded).expect("deserialize value"),
            value
        );

        assert!(serde_json::from_str::<NeedValue>("-1").is_err());
        assert!(serde_json::from_str::<NeedValue>("100001").is_err());
        assert!(serde_json::from_str::<Needs>("{\"hunger\":-5,\"fatigue\":0}").is_err());
        assert!(serde_json::from_str::<Needs>("{\"hunger\":0,\"fatigue\":100001}").is_err());
        // No float is accepted anywhere: the model is integer-only.
        assert!(serde_json::from_str::<NeedValue>("1.5").is_err());
        assert!(serde_json::from_str::<Needs>("{\"hunger\":1.5,\"fatigue\":0}").is_err());
    }
}
