use chrono::{SubsecRound, Utc};

use crate::kernel::Timestamp;

/// Wall clock. Injected into application code as `Arc<dyn Clock>`; domain code receives `at: Timestamp`.
pub(crate) trait Clock: Send + Sync {
    fn now(&self) -> Timestamp;
}

/// Microsecond precision, like Postgres `timestamptz`, so a stored instant reads back unchanged.
pub(crate) struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Timestamp {
        Timestamp::from_utc(Utc::now().trunc_subsecs(6))
    }
}

#[cfg(test)]
pub(crate) mod fakes {
    use std::sync::Mutex;

    use super::Clock;
    use crate::kernel::Timestamp;

    /// A clock that stays where the test put it.
    pub(crate) struct FakeClock(Mutex<Timestamp>);

    impl FakeClock {
        pub(crate) fn at(at: Timestamp) -> Self {
            Self(Mutex::new(at))
        }
    }

    impl Clock for FakeClock {
        fn now(&self) -> Timestamp {
            *self
                .0
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
        }
    }
}
