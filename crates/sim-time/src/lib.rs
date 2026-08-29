//! Deterministic integer time primitives for the headless simulation.

use std::error::Error;
use std::fmt::{self, Display, Formatter};

use serde::{Deserialize, Serialize};

/// A point on the simulation timeline, represented as integer seconds.
#[derive(
    Clone, Copy, Debug, Default, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize,
)]
#[serde(transparent)]
pub struct SimInstant(i64);

impl SimInstant {
    /// The default simulation epoch.
    pub const EPOCH: Self = Self(0);

    /// Lowest representable instant.
    pub const MIN: Self = Self(i64::MIN);

    /// Highest representable instant.
    pub const MAX: Self = Self(i64::MAX);

    /// Creates an instant from signed simulation seconds.
    #[must_use]
    pub const fn from_seconds(seconds: i64) -> Self {
        Self(seconds)
    }

    /// Returns signed simulation seconds from the epoch.
    #[must_use]
    pub const fn as_seconds(self) -> i64 {
        self.0
    }

    /// Adds a non-negative simulation duration, returning `None` on overflow.
    #[must_use]
    pub const fn checked_add(self, duration: SimDuration) -> Option<Self> {
        match self.0.checked_add(duration.0) {
            Some(seconds) => Some(Self(seconds)),
            None => None,
        }
    }

    /// Subtracts a non-negative simulation duration, returning `None` on overflow.
    #[must_use]
    pub const fn checked_sub(self, duration: SimDuration) -> Option<Self> {
        match self.0.checked_sub(duration.0) {
            Some(seconds) => Some(Self(seconds)),
            None => None,
        }
    }

    /// Returns elapsed non-negative time since `earlier`.
    #[must_use]
    pub const fn duration_since(self, earlier: Self) -> Option<SimDuration> {
        match self.0.checked_sub(earlier.0) {
            Some(seconds) => SimDuration::from_seconds(seconds),
            None => None,
        }
    }
}

impl Display for SimInstant {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}s", self.0)
    }
}

/// A non-negative span of simulation time in integer seconds.
#[derive(
    Clone, Copy, Debug, Default, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize,
)]
#[serde(try_from = "i64", into = "i64")]
pub struct SimDuration(i64);

impl SimDuration {
    /// Zero elapsed simulation time.
    pub const ZERO: Self = Self(0);

    /// Largest representable duration.
    pub const MAX: Self = Self(i64::MAX);

    /// Creates a duration when `seconds` is non-negative.
    #[must_use]
    pub const fn from_seconds(seconds: i64) -> Option<Self> {
        if seconds >= 0 {
            Some(Self(seconds))
        } else {
            None
        }
    }

    /// Returns the duration in integer simulation seconds.
    #[must_use]
    pub const fn as_seconds(self) -> i64 {
        self.0
    }
}

impl Display for SimDuration {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}s", self.0)
    }
}

impl From<SimDuration> for i64 {
    fn from(duration: SimDuration) -> Self {
        duration.as_seconds()
    }
}

impl TryFrom<i64> for SimDuration {
    type Error = InvalidSimDuration;

    fn try_from(seconds: i64) -> Result<Self, Self::Error> {
        Self::from_seconds(seconds).ok_or(InvalidSimDuration { seconds })
    }
}

/// Error returned when constructing a duration from negative seconds.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvalidSimDuration {
    seconds: i64,
}

impl Display for InvalidSimDuration {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "simulation duration cannot be negative: {}s",
            self.seconds
        )
    }
}

impl Error for InvalidSimDuration {}

/// Monotonic owner of the current simulation instant.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct SimClock {
    now: SimInstant,
}

impl SimClock {
    /// Creates a clock at an explicitly restored instant.
    #[must_use]
    pub const fn at(now: SimInstant) -> Self {
        Self { now }
    }

    /// Returns the current simulation instant.
    #[must_use]
    pub const fn now(self) -> SimInstant {
        self.now
    }

    /// Advances by a non-negative duration.
    ///
    /// # Errors
    ///
    /// Returns [`SimClockError::Overflow`] if the resulting instant cannot be
    /// represented by `i64` seconds.
    pub fn advance_by(&mut self, duration: SimDuration) -> Result<SimInstant, SimClockError> {
        let target = self
            .now
            .checked_add(duration)
            .ok_or(SimClockError::Overflow)?;
        self.now = target;
        Ok(target)
    }

    /// Advances to an absolute instant without allowing time reversal.
    ///
    /// # Errors
    ///
    /// Returns [`SimClockError::TimeReversal`] when `target` is earlier than
    /// the current instant.
    pub fn advance_to(&mut self, target: SimInstant) -> Result<SimInstant, SimClockError> {
        if target < self.now {
            return Err(SimClockError::TimeReversal {
                current: self.now,
                requested: target,
            });
        }
        self.now = target;
        Ok(target)
    }
}

/// Failure to advance a monotonic [`SimClock`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SimClockError {
    /// Adding a duration exceeded the representable timeline.
    Overflow,
    /// An absolute advancement requested an earlier instant.
    TimeReversal {
        /// Current clock value.
        current: SimInstant,
        /// Rejected earlier value.
        requested: SimInstant,
    },
}

impl Display for SimClockError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Overflow => formatter.write_str("simulation clock overflow"),
            Self::TimeReversal { current, requested } => {
                write!(
                    formatter,
                    "simulation time cannot move backward from {current} to {requested}"
                )
            }
        }
    }
}

impl Error for SimClockError {}

#[cfg(test)]
mod tests {
    use super::{SimClock, SimClockError, SimDuration, SimInstant};

    #[test]
    fn instant_serde_is_a_signed_integer() {
        let instant = SimInstant::from_seconds(-42);
        let encoded = serde_json::to_string(&instant).expect("serialize instant");
        assert_eq!(encoded, "-42");
        assert_eq!(
            serde_json::from_str::<SimInstant>(&encoded).expect("deserialize instant"),
            instant
        );
    }

    #[test]
    fn negative_duration_is_rejected() {
        assert_eq!(SimDuration::from_seconds(-1), None);
        assert_eq!(SimDuration::from_seconds(0), Some(SimDuration::ZERO));
    }

    #[test]
    fn duration_deserialization_rejects_negative_values() {
        assert!(serde_json::from_str::<SimDuration>("-1").is_err());
    }

    #[test]
    fn clock_advances_monotonically() {
        let mut clock = SimClock::default();
        let duration = SimDuration::from_seconds(60).expect("non-negative duration");
        assert_eq!(clock.advance_by(duration), Ok(SimInstant::from_seconds(60)));
        assert_eq!(
            clock.advance_to(SimInstant::from_seconds(120)),
            Ok(SimInstant::from_seconds(120))
        );
        assert_eq!(clock.now(), SimInstant::from_seconds(120));
    }

    #[test]
    fn advancing_to_the_same_instant_is_allowed() {
        let instant = SimInstant::from_seconds(7);
        let mut clock = SimClock::at(instant);
        assert_eq!(clock.advance_to(instant), Ok(instant));
    }

    #[test]
    fn time_reversal_is_explicitly_rejected() {
        let current = SimInstant::from_seconds(10);
        let requested = SimInstant::from_seconds(9);
        let mut clock = SimClock::at(current);
        assert_eq!(
            clock.advance_to(requested),
            Err(SimClockError::TimeReversal { current, requested })
        );
        assert_eq!(clock.now(), current);
    }

    #[test]
    fn overflow_is_explicit_and_does_not_mutate_clock() {
        let mut clock = SimClock::at(SimInstant::MAX);
        let one = SimDuration::from_seconds(1).expect("non-negative duration");
        assert_eq!(clock.advance_by(one), Err(SimClockError::Overflow));
        assert_eq!(clock.now(), SimInstant::MAX);
    }

    #[test]
    fn duration_since_handles_order_and_boundaries() {
        let later = SimInstant::from_seconds(100);
        let earlier = SimInstant::from_seconds(40);
        assert_eq!(later.duration_since(earlier), SimDuration::from_seconds(60));
        assert_eq!(earlier.duration_since(later), None);
        assert_eq!(SimInstant::MAX.duration_since(SimInstant::MIN), None);
    }

    #[test]
    fn clock_serde_round_trip_preserves_current_time() {
        let clock = SimClock::at(SimInstant::from_seconds(9_876_543));
        let encoded = serde_json::to_string(&clock).expect("serialize clock");
        let restored: SimClock = serde_json::from_str(&encoded).expect("deserialize clock");
        assert_eq!(restored, clock);
    }
}
