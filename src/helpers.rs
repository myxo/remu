use chrono;
use chrono::prelude::*;
use crate::command::*;
use crate::database::DataBase;
use crate::time::now;

// TODO: make test
pub fn format_return_message_header(event_time: &DateTime<Utc>, tz: i32) -> String {
    let tz = FixedOffset::west(tz * 3600);
    let t_event = event_time.with_timezone(&tz);
    let t_now = now().with_timezone(&tz);

    if t_event < t_now {
        return String::from("Event time is in the past. Is it right?");
    }

    // days from year 1 TODO: maybe day of the year?
    let day_event = t_event.num_days_from_ce();
    let day_now = t_now.num_days_from_ce();
    let dt = day_event - day_now;

    if dt == 0 {
        return t_event.format("I'll remind you today at %H:%M").to_string();
    } else if dt == 1 {
        return t_event
            .format("I'll remind you tomorrow at %H:%M")
            .to_string();
    }
    t_event.format("I'll remind you %B %e at %H:%M").to_string()
}

pub fn process_text_command(uid: i64, text_message: &str, db: &mut DataBase) -> Option<String> {
    let tz = db.get_user_timezone(uid);
    match parse_command(String::from(text_message), tz)? {
        Command::OneTimeEvent(ev) => {
            Some(process_one_time_event_command(uid, ev, db))
        }
        Command::RepetitiveEvent(ev) => {
            Some(process_repetitive_event_command(uid, ev, db))
        }
    }
}

fn process_one_time_event_command(
    uid: i64,
    c: OneTimeEventImpl,
    db: &mut DataBase,
) -> String {
    let tz = db.get_user_timezone(uid);
    let mut return_string = format_return_message_header(&c.event_time, tz);
    return_string.push('\n');
    return_string.push_str(&c.event_text);
    db.put(uid, Command::OneTimeEvent(c));

    // delete newline char to write to log
    let tmp_string = str::replace(&return_string[..], "\n", " ");
    debug!(
        "Successfully process command, return string - <{}>",
        tmp_string
    );

    return_string
}

fn process_repetitive_event_command(
    uid: i64,
    c: RepetitiveEventImpl,
    db: &mut DataBase,
) -> String {
    let tz = db.get_user_timezone(uid);
    let mut return_string = format_return_message_header(&c.event_start_time, tz);
    return_string.push('\n');
    return_string.push_str(&c.event_text);
    db.put(uid, Command::RepetitiveEvent(c));

    // delete newline char to write to log
    let tmp_string = str::replace(&return_string[..], "\n", " ");
    debug!(
        "Successfully process command, return string - <{}>",
        tmp_string
    );

    return_string
}

pub fn get_active_event_list(uid: i64, db: &mut DataBase) -> Vec<String> {
    let mut result = Vec::new();
    let command_vector = db.get_all_active_events(uid);
    let tz = db.get_user_timezone(uid) as i64;
    let dt = chrono::Duration::seconds(-tz * 60 * 60);
    for command in command_vector {
        match command {
            Command::OneTimeEvent(c) => {
                let text: String = c.event_text.chars().take(40).collect();
                let date: String = (c.event_time + dt).format("%e %b %k.%M").to_string();
                result.push(format!("{} : _{}_", text, date));
            }
            Command::RepetitiveEvent(_ev) => {}
        }
    }
    result
}

pub fn get_rep_event_list(uid: i64, db: &mut DataBase) -> (Vec<String>, Vec<i64>) {
    let mut result_str = Vec::new();
    let mut result_id = Vec::new();
    let command_vector = db.get_all_rep_events(uid);
    // const DEFAULT_TZ: i64 = 3;
    // let dt = chrono::Duration::seconds(DEFAULT_TZ * 60 * 60);
    for line in command_vector {
        let id: i64 = line.1;
        let command = line.0;
        match command {
            Command::RepetitiveEvent(ev) => {
                let text: String = ev.event_text.chars().take(40).collect();
                result_str.push(text);
                result_id.push(id);
            }
            Command::OneTimeEvent(_ev) => {}
        }
    }
    (result_str, result_id)
}
