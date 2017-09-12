extern crate chrono;
extern crate regex;

use chrono::prelude::*;
use regex::Regex;

#[derive(Debug)]
struct Command{
    event_time : DateTime<Local>,
    event_text : String,
}


impl Command{
    fn parse_command(commind_line : String) -> Command {
        let reg_main    = Regex::new(r"(?P<spec>[\d\w]*)(?P<divider> )(?P<main_text>.*)").unwrap();

        let reg_day     = r"(?P<days>[\d]*)[D|d]";
        let reg_hour    = r"(?P<hours>[\d]*)[H|h]";
        let reg_min     = r"(?P<minuts>[\d]*)[M|m]";
        let reg_sec     = r"(?P<seconds>[\d]*)[S|s]";


        let caps = reg_main.captures(&commind_line).unwrap();
        let spec = caps.name("spec").unwrap().as_str();
        let text = caps.name("main_text").unwrap().as_str();


        let days    = get_first_regex_group_as_u32(reg_day,  spec);
        let hours   = get_first_regex_group_as_u32(reg_hour, spec);
        let minutes = get_first_regex_group_as_u32(reg_min,  spec);
        let seconds = get_first_regex_group_as_u32(reg_sec,  spec);

        // TODO: should we a better way to do this
        let dt = chrono::Duration::seconds((days as i64) * (60*60*24) 
                                        + (hours as i64) * (60*60) 
                                        + (minutes as i64) * 60 
                                        + (seconds as i64)
                                        );

        let event_time = Local::now() + dt;
        // println!("{}\n{}\n{}", dt, Local::now(), event_time);

        Command { event_text : String::from(text), event_time : event_time } 
    }

} // impl Command

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


fn main() {
    // let st : time::SteadyTime = time::SteadyTime::now();
    Command::parse_command(String::from("5s test"));
    Command::parse_command(String::from("5d test"));
    Command::parse_command(String::from("3m5s test"));
}
