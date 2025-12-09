use chrono::{DateTime, Utc};

pub trait Clock {
    fn now(&self) -> DateTime<Utc>;
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

pub struct MockClock {
    currect_time: DateTime<Utc>,
}

impl MockClock {
    pub fn new(start: DateTime<Utc>) -> MockClock {
        MockClock {
            currect_time: start,
        }
    }
}

impl Clock for MockClock {
    fn now(&self) -> DateTime<Utc> {
        self.currect_time
    }

    fn set_time(&mut self, t: DateTime<Utc>) {
        self.currect_time = t;
    }
}
