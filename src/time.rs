use chrono::{DateTime, Utc};

#[cfg(not(feature = "mock-time"))]
pub fn now() -> DateTime<Utc> {
    Utc::now()
}

#[cfg(not(feature = "mock-time"))]
pub fn set_mock_time(_time: Option<DateTime<Utc>>) {
    warn!("Setting mock time outside of feature = \"mock-time\" enviroment");
}

#[cfg(feature = "mock-time")]
pub mod mock_time {
    use super::*;
    use std::sync::Mutex;
    use lazy_static::lazy_static;

    lazy_static! {
        static ref MOCK_TIME: Mutex<Option<DateTime<Utc>>> = Mutex::new(None);
    }

    pub fn now() -> DateTime<Utc> {
        MOCK_TIME.lock().unwrap().unwrap_or_else(Utc::now)
    }

    pub fn set_mock_time(time: Option<DateTime<Utc>>) {
        *MOCK_TIME.lock().unwrap() = time;
    }
}

#[cfg(feature = "mock-time")]
pub use mock_time::now;

#[cfg(feature = "mock-time")]
pub use mock_time::set_mock_time;