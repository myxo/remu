extern crate chrono;

use chrono::prelude::*;
use regex::{Regex, Captures};

#[derive(Debug)]
pub enum Command {
    BadCommand,
    OneTimeEvent(OneTimeEventImpl),
}

#[derive(Debug)]
pub struct OneTimeEventImpl{
    pub event_time : DateTime<Utc>,
    pub event_text : String,
}


pub fn parse_command(command_line : String) -> Command {
    let command_line = String::from(command_line.trim());
    debug!("parse incoming text: {}", command_line);
    let mut result;
    result = try_parse_for(&command_line);
    if result.is_some() {
        return result.unwrap();
    }

    result = try_parse_at(&command_line);
    if result.is_some() {
        return result.unwrap();
    }
    Command::BadCommand
}


fn try_parse_at(command_line : &String) -> Option<Command>{
    let time_format = Regex::new(r"[at|в]\s*(?P<day>[\d]*)(?:-(?P<month>[\d]*))?(?:-(?P<year>[\d]*))? (?P<hour>[\d]*).(?P<minute>[\d]*) (?P<main_text>.*)").unwrap();


    let date_captures = time_format.captures(command_line);
    if date_captures.is_none() {
        warn!("parse_command: line {} doesn't match time_format regex", command_line);
        return None;
    }

    let date_captures = date_captures.unwrap();

    let text = date_captures.name("main_text").unwrap().as_str();
    let d = get_datetime_from_capture(&date_captures);

    Some(Command::OneTimeEvent(OneTimeEventImpl 
            { event_text : String::from(text)
            , event_time : d } 
        ))
}


fn try_parse_for(command_line : &String) -> Option<Command>{
    let reg_main    = Regex::new(r"(?P<spec>[\d\w]*)(?P<divider> )(?P<main_text>.*)").unwrap();

    let reg_day     = r"(?P<days>[\d]*)[D|d|Д|д]";
    let reg_hour    = r"(?P<hours>[\d]*)[H|h|Ч|ч]";
    let reg_min     = r"(?P<minuts>[\d]*)[M|m|М|м]";
    let reg_sec     = r"(?P<seconds>[\d]*)[S|s|С|с]";


    let caps = reg_main.captures(command_line);
    if caps.is_none() {
        warn!("parse_command: incoming text doesn't match main regex");
        return None;
    }
    let caps = caps.unwrap();
    let spec = caps.name("spec").unwrap().as_str();
    let text = caps.name("main_text").unwrap().as_str();


    let days    = get_first_regex_group_as_u32(reg_day,  spec);
    let hours   = get_first_regex_group_as_u32(reg_hour, spec);
    let minutes = get_first_regex_group_as_u32(reg_min,  spec);
    let seconds = get_first_regex_group_as_u32(reg_sec,  spec);

    if days == 0 && hours == 0 && minutes == 0 && seconds == 0 {
        warn!("parse_command: incoming text doesn't match main regex");
        return None;
    }

    // TODO: should we a better way to do this
    let dt = chrono::Duration::seconds((days as i64) * (60*60*24) 
                                    + (hours as i64) * (60*60) 
                                    + (minutes as i64) * 60 
                                    + (seconds as i64)
                                    );

    let event_time = Utc::now() + dt;

    Some(Command::OneTimeEvent(OneTimeEventImpl 
            { event_text : String::from(text)
            , event_time : event_time } 
        ))
}


fn get_first_regex_group_as_u32(reg : &str, text : &str) -> u32{
    let reg = Regex::new(reg).expect("get_first_regex_group_as_u32: wrong regex string");
    let number = match reg.captures(text){
        None => "0",
        Some(d) => d.get(1)
            .expect("get_first_regex_group_as_u32: expect regex string with at least 1 group")
            .as_str(), 
    };
    number.parse().unwrap_or(0)
}


fn get_datetime_from_capture(cap: &Captures) -> DateTime<Utc>{
    let day = cap.name("day").unwrap().as_str().parse().unwrap();
    let hour = cap.name("hour").unwrap().as_str().parse().unwrap();
    let minute = cap.name("minute").unwrap().as_str().parse().unwrap();

    let now = Utc::now();

    let month = match cap.name("month"){
        None => now.month(),
        Some(m) => m.as_str().parse().unwrap(),
    };

    let year = match cap.name("year"){
        None => now.year(),
        Some(m) => m.as_str().parse().unwrap(),
    };
    

    const DEFAULT_TZ : i64 = -3;
    let dt = chrono::Duration::seconds(DEFAULT_TZ * 60 * 60);

    Utc.ymd(year, month, day).and_hms(hour, minute, 0) + dt
}
