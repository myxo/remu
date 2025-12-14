extern crate chrono;

use chrono::prelude::*;
use log::warn;
use regex::{Captures, Regex};

#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    OneTimeEvent(OneTimeEventImpl),
    RepetitiveEvent(RepetitiveEventImpl),
}

#[derive(Debug, Clone, PartialEq)]
pub struct OneTimeEventImpl {
    pub event_time: DateTime<Utc>,
    pub event_text: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RepetitiveEventImpl {
    pub event_start_time: DateTime<Utc>,
    pub event_wait_time: chrono::Duration,
    pub event_text: String,
}

const MOMENT_DAY_REGEX: &str =
    r"(?:(?P<m_day>[\d]+))?(?:-(?P<m_month>[\d]+))?(?:-(?P<m_year>[\d]+))?";

const MOMENT_TIME_REGEX: &str = r"(?P<m_hour>[\d]+)(?:[.|:](?P<m_minute>[\d]+))?";

const DURATION_REGEX: &str = r"(:?(?P<d_day>[\d]*)[D|d|Д|д])?(:?(?P<d_hour>[\d]*)[H|h|Ч|ч])?(:?(?P<d_minute>[\d]*)[M|m|М|м])?(:?(?P<d_second>[\d]*)[S|s|С|с])?";

pub fn parse_command(
    command_line: String,
    now: DateTime<Utc>,
    user_timezone: i32,
) -> Option<Command> {
    let command_line = String::from(command_line.trim());
    let mut result;
    result = try_parse_for(&command_line, now);
    if result.is_some() {
        return result;
    }

    result = try_parse_at(&command_line, now, user_timezone);
    if result.is_some() {
        return result;
    }

    result = try_parse_rep(&command_line, now, user_timezone);
    if result.is_some() {
        return result;
    }

    warn!(
        "parse_command: line {} doesn't match any regex",
        command_line
    );
    None
}

fn try_parse_at(command_line: &str, now: DateTime<Utc>, user_timezone: i32) -> Option<Command> {
    let reg = format!(
        r"^{}\s*(at|At|в|В)\s*{} (?P<main_text>.*)",
        MOMENT_DAY_REGEX, MOMENT_TIME_REGEX
    );
    let time_format = Regex::new(&reg[..]).unwrap();

    let date_captures = time_format.captures(command_line)?;
    let text = date_captures.name("main_text").unwrap().as_str();

    if let Some(t) = get_datetime_from_capture(&date_captures, now, user_timezone) {
        return Some(Command::OneTimeEvent(OneTimeEventImpl {
            event_text: String::from(text),
            event_time: t,
        }));
    }
    None
}

fn try_parse_for(command_line: &str, now: DateTime<Utc>) -> Option<Command> {
    let reg = String::from("^") + DURATION_REGEX + r"(?P<divider> )(?P<main_text>.*)";
    let reg = Regex::new(&reg[..]).unwrap();

    let capture = reg.captures(command_line)?;

    let text = capture.name("main_text").unwrap().as_str();
    let dt = get_duration_from_capture(&capture)?;

    Some(Command::OneTimeEvent(OneTimeEventImpl {
        event_text: String::from(text),
        event_time: now + dt,
    }))
}

fn try_parse_rep(command_line: &str, now: DateTime<Utc>, user_timezone: i32) -> Option<Command> {
    let reg = format!(
        r"^rep\s*{}\s+{}\s+{}(?P<divider> )(?P<main_text>.*)",
        MOMENT_DAY_REGEX, MOMENT_TIME_REGEX, DURATION_REGEX
    );
    let reg = Regex::new(&reg[..]).unwrap();

    let capture = reg.captures(command_line)?;
    let text = capture.name("main_text").unwrap().as_str();
    let time = get_datetime_from_capture(&capture, now, user_timezone);
    let dt = get_duration_from_capture(&capture);
    if time.is_none() || dt.is_none() {
        return None;
    }

    Some(Command::RepetitiveEvent(RepetitiveEventImpl {
        event_start_time: time.unwrap(),
        event_wait_time: dt.unwrap(),
        event_text: String::from(text),
    }))
}

#[rustfmt::skip]
fn get_duration_from_capture(cap: &Captures) -> Option<chrono::Duration>{
    let day:    i64 = cap.name("d_day").map_or    (0, |c| c.as_str().parse().unwrap() );
    let hour:   i64 = cap.name("d_hour").map_or   (0, |c| c.as_str().parse().unwrap() );
    let minute: i64 = cap.name("d_minute").map_or (0, |c| c.as_str().parse().unwrap() );
    let second: i64 = cap.name("d_second").map_or (0, |c| c.as_str().parse().unwrap() );

    if day == 0 && hour == 0 && minute == 0 && second == 0 {
        return None;
    }

    Some(chrono::Duration::seconds(day * (60*60*24) 
                                 + hour * (60*60) 
                                 + minute * 60 
                                 + second))
}

#[rustfmt::skip]
fn get_datetime_from_capture(cap: &Captures, now: DateTime<Utc>, tz: i32) -> Option<DateTime<Utc>>{
    let dt = chrono::Duration::seconds((tz as i64) * 60 * 60);
    let now = now - dt;

    let day     = cap.name("m_day").map_or(now.day(),       |c| c.as_str().parse().unwrap());
    let month   = cap.name("m_month").map_or(now.month(),   |c| c.as_str().parse().unwrap());
    let year    = cap.name("m_year").map_or(now.year(),     |c| c.as_str().parse().unwrap());
    let minute  = cap.name("m_minute").map_or(0,            |c| c.as_str().parse().unwrap());
    
    let hour    = cap.name("m_hour").unwrap().as_str().parse().unwrap();
    
    use chrono::offset::LocalResult::Single;
    if let Single(date) = Utc.with_ymd_and_hms(year, month, day, hour, minute, 0) {
        return Some(date + dt);
    }
    None
}

//-------- TESTS ---------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::Command::*;

    #[test]
    fn parse_for_general() {
        let command = String::from("1d2h3m4s");
        let text = "some text";
        let command_text = command + " " + text;
        let dt = chrono::Duration::days(1)
            + chrono::Duration::hours(2)
            + chrono::Duration::minutes(3)
            + chrono::Duration::seconds(4);
        let now = Utc::now();
        let result = try_parse_for(&command_text, now);
        assert!(result.is_some());
        match result.unwrap() {
            OneTimeEvent(res) => {
                assert_eq!(res.event_text, text);
                assert!(time_moment_eq(res.event_time, now + dt));
            }
            _ => panic!("Wrong command type"),
        };
    }

    #[test]
    fn parse_for_hm() {
        let command = String::from("2h30m");
        let text = "some text";
        let command_text = command + " " + text;
        let dt = chrono::Duration::hours(2) + chrono::Duration::minutes(30);
        let now = Utc::now();
        let result = try_parse_for(&command_text, now);
        assert!(result.is_some());
        match result.unwrap() {
            OneTimeEvent(res) => {
                assert_eq!(res.event_text, text);
                assert!(time_moment_eq(res.event_time, now + dt));
            }
            _ => panic!("Wrong command type"),
        };
    }

    #[test]
    // #[ignore]
    fn parse_for_negative() {
        let command = String::from("1d-2h3m4s");
        let text = "some text";
        let command_text = command + " " + text;
        let result = try_parse_for(&command_text, Utc::now());
        assert!(result.is_none());
    }

    #[test]
    // #[ignore]
    fn parse_for_misspell() {
        let command = String::from("1d2j3m4s");
        let text = "some text";
        let command_text = command + " " + text;
        let result = try_parse_for(&command_text, Utc::now());
        assert!(result.is_none());
    }

    #[test]
    fn parse_at_tests() {
        let now = Utc.with_ymd_and_hms(2024, 10, 24, 12, 0, 0).unwrap();
        {
            let command = String::from("24-10 at 18.30");
            let text = "some text";
            let command_text = command + " " + text;
            let t = Utc
                .with_ymd_and_hms(now.year(), 10, 24, 18 - 3, 30, 0)
                .unwrap();
            let result = try_parse_at(&command_text, now, -3);
            assert!(result.is_some());
            match result.unwrap() {
                OneTimeEvent(res) => {
                    assert_eq!(res.event_text, text);
                    assert_eq!(res.event_time, t);
                }
                _ => panic!("Wrong command type"),
            };
        }

        {
            let now = Utc.timestamp_opt(61, 0).unwrap();
            let command = String::from("24 at 18.30");
            let text = "some text";
            let command_text = command + " " + text;
            let t = Utc
                .with_ymd_and_hms(now.year(), now.month(), 24, 18 - 3, 30, 0)
                .unwrap();

            let result = try_parse_at(&command_text, now, -3);
            assert!(result.is_some());
            match result.unwrap() {
                OneTimeEvent(res) => {
                    assert_eq!(res.event_text, text);
                    assert_eq!(res.event_time, t);
                }
                _ => panic!("Wrong command type"),
            };
        }
        {
            let command = String::from("at 18.30");
            let text = "some text";
            let command_text = command + " " + text;
            let t = Utc
                .with_ymd_and_hms(now.year(), now.month(), now.day(), 18, 30, 0)
                .unwrap();

            let result = try_parse_at(&command_text, now, 0);
            assert!(result.is_some());
            match result.unwrap() {
                OneTimeEvent(res) => {
                    assert_eq!(res.event_text, text);
                    assert_eq!(res.event_time, t);
                }
                _ => panic!("Wrong command type"),
            };
        }
        {
            let command = String::from("at 18");
            let text = "some text";
            let command_text = command + " " + text;
            let t = Utc
                .with_ymd_and_hms(now.year(), now.month(), now.day(), 18, 0, 0)
                .unwrap();

            let result = try_parse_at(&command_text, now, 0);
            assert!(result.is_some());
            match result.unwrap() {
                OneTimeEvent(res) => {
                    assert_eq!(res.event_text, text);
                    assert_eq!(res.event_time, t);
                }
                _ => panic!("Wrong command type"),
            };
        }
    }

    #[test]
    fn parse_wrong_at_tests() {
        {
            let command = String::from("30-02 at 18");
            let text = "some text";
            let command_text = command + " " + text;

            let result = try_parse_at(&command_text, Utc::now(), -3);
            assert!(result.is_none());
        }
        {
            let command = String::from("30-20 at 18");
            let text = "some text";
            let command_text = command + " " + text;

            let result = try_parse_at(&command_text, Utc::now(), -3);
            assert!(result.is_none());
        }
        {
            let command = String::from("at 25");
            let text = "some text";
            let command_text = command + " " + text;

            let result = try_parse_at(&command_text, Utc::now(), -3);
            assert!(result.is_none());
        }
        {
            let command = String::from("at 23.60");
            let text = "some text";
            let command_text = command + " " + text;

            let result = try_parse_at(&command_text, Utc::now(), -3);
            assert!(result.is_none());
        }
    }

    #[test]
    fn parse_rep_general() {
        let command = String::from("rep 06-10 10.00 5m");
        let text = "test rep";
        let command_text = command + " " + text;
        let now = Utc::now();
        let t = Utc
            .with_ymd_and_hms(now.year(), 10, 6, 10 - 3, 0, 0)
            .unwrap();
        let dt = chrono::Duration::minutes(5);
        let result = try_parse_rep(&command_text, now, -3);
        assert!(result.is_some());
        match result.unwrap() {
            RepetitiveEvent(res) => {
                assert_eq!(res.event_start_time, t);
                assert_eq!(res.event_wait_time, dt);
                assert_eq!(res.event_text, text);
            }
            _ => panic!("Wrong command type"),
        };
    }

    fn time_moment_eq(t1: DateTime<Utc>, t2: DateTime<Utc>) -> bool {
        t1.signed_duration_since(t2).num_milliseconds().abs() < 100
    }
}
