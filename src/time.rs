use chrono::{DateTime, Utc};

pub trait Clock {
    fn now(&self) -> DateTime<Utc>;
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "Setter is only used in tests/mocked time")
    )]
    fn set_time(&mut self, t: DateTime<Utc>);
}

pub struct OsClock {}

impl Clock for OsClock {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }

    fn set_time(&mut self, _: DateTime<Utc>) {
        panic!("cannot set time for OsClock");
    }
}

#[cfg(any(test, feature = "mock-time"))]
pub struct MockClock {
    currect_time: DateTime<Utc>,
}

#[cfg(any(test, feature = "mock-time"))]
impl MockClock {
    pub fn new(start: DateTime<Utc>) -> MockClock {
        MockClock {
            currect_time: start,
        }
    }
}

#[cfg(any(test, feature = "mock-time"))]
impl Clock for MockClock {
    fn now(&self) -> DateTime<Utc> {
        self.currect_time
    }

    fn set_time(&mut self, t: DateTime<Utc>) {
        self.currect_time = t;
    }
}
