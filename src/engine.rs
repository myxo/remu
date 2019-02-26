use chrono::prelude::*;
use chrono;
use std::thread;
use std::time;
use std::sync::atomic::{AtomicBool, Ordering};
use std::collections::HashMap;
use serde_json::json;
use serde::{Deserialize, Serialize};
use serde_json::Result;

use crate::command::*;
use crate::database::DataBase;



pub struct Engine {
    next_wakeup: Option<DateTime<Utc>>,
    data_base: DataBase,
    frontend_callback : Option<(fn(String, i64))>,
    stop_loop: AtomicBool,
    user_states: HashMap<i32, Box<UserState>>,
}

struct ProcessResult {
    frontend_command: Vec<FrontendCommand>,
    next_state: Box<UserState>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct SendMessageCommand {
    text: String,
}

// FIXME: remove clone trait
#[derive(Clone, Debug, Serialize, Deserialize)]
struct AtCalendarCommand{
    action: String,
    action_type: String,
    text: String,
    year : i32,
    month : i32,
    tz: i32,
    edit_msg: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct KeyboardCommand {
    action: String,
    action_type: String,
    text: String,
}

#[allow(non_camel_case_types)]
#[derive(Clone, Debug, Deserialize, Serialize)]
enum FrontendCommand {
    send(SendMessageCommand),
    calendar(AtCalendarCommand),
    keyboard(KeyboardCommand),
}

trait UserState {
    fn process(&self, _id: i64, _input: &str, _db : &mut DataBase) -> ProcessResult {
        panic!("Default UserState::process")
    }
    
    fn process_keyboard(&self, _id: i64, _input: &str, _db : &mut DataBase) -> ProcessResult {
        panic!("Default UserState::process")
    }
}

struct ReadyToProcess{
    
}

impl ReadyToProcess {

    fn process_text_command(uid: i64, text_message: &str, db: &mut DataBase) -> (String, i32){
        let tz = db.get_user_timezone(uid);
        let com = parse_command(String::from(text_message), tz);
        match com {
            Command::OneTimeEvent(ev) => ReadyToProcess::process_one_time_event_command(uid, ev, db),
            Command::RepetitiveEvent(ev) => ReadyToProcess::process_repetitive_event_command(uid, ev, db),
            Command::BadCommand => ReadyToProcess::process_bad_command(),
        }
    }

    fn process_one_time_event_command(uid:i64, c: OneTimeEventImpl, db: &mut DataBase) -> (String, i32) {
        let tz = db.get_user_timezone(uid);
        let mut return_string = format_return_message_header(&c.event_time, tz);
        return_string.push('\n');
        return_string.push_str(&c.event_text);
        db.put(uid, Command::OneTimeEvent(c));
        
        // delete newline char to write to log
        let tmp_string = str::replace(&return_string[..], "\n", " ");
        info!("Successfully process command, return string - <{}>", tmp_string);

        (return_string, 0)
    }

    fn process_repetitive_event_command(uid: i64, c: RepetitiveEventImpl, db: &mut DataBase) -> (String, i32) {
        let tz = db.get_user_timezone(uid);
        let mut return_string = format_return_message_header(&c.event_start_time, tz);
        return_string.push('\n');
        return_string.push_str(&c.event_text);
        db.put(uid, Command::RepetitiveEvent(c));
        
        // delete newline char to write to log
        let tmp_string = str::replace(&return_string[..], "\n", " ");
        info!("Successfully process command, return string - <{}>", tmp_string);

        (return_string, 0)
    }

    fn process_bad_command() -> (String, i32) {
        (String::from(""), 1)
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
                Command::BadCommand => {}
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
                Command::BadCommand => {}
                Command::OneTimeEvent(_ev) => {}
            }
        }
        (result_str, result_id)
    }

    fn start_calendar(&self, id: i64, input: &str, db : &mut DataBase) -> ProcessResult {
        info!("ReadyToProcess::start_Calendar");
        let tz = db.get_user_timezone(id);
        let dt = chrono::Duration::seconds((tz as i64) * 60 * 60);
        let now = Utc::now() - dt;
        let edit_msg: bool = !input.starts_with("/at");

        let command = AtCalendarCommand{
            action: "Keyboard".to_string(),
            action_type: "Calendar".to_string(),
            month: now.month() as i32,
            year: now.year() as i32,
            text: input.to_owned(),
            tz: tz,
            edit_msg: edit_msg,
        };

        ProcessResult {
            frontend_command: vec![ FrontendCommand::calendar(command.clone())],
            next_state: Box::new(AtCalendar{
                command: command
            })
        }
    }
}

impl UserState for ReadyToProcess {
    fn process(&self, id: i64, input: &str, db : &mut DataBase) -> ProcessResult {
        info!("ReadyToProcess::process, input: {}", input);
        if input.starts_with("/help more"){ // TODO: should be exact matching
            let command = SendMessageCommand {
                text: "Detailed help message".to_owned(),
            };
            return ProcessResult {
                frontend_command: vec![ FrontendCommand::send(command) ],
                next_state: Box::new(ReadyToProcess{})
            }
        }
        if input.starts_with("/help"){ // TODO: should be exact matching
            let command = SendMessageCommand {
                text: "Simple help message".to_owned(),
            };
            return ProcessResult {
                frontend_command: vec![ FrontendCommand::send(command) ],
                next_state: Box::new(ReadyToProcess{})
            }
        }
        if input.starts_with("/list"){
            let list = ReadyToProcess::get_active_event_list(id, db);
            let mut ret_text = String::from("No current active event");
            if !list.is_empty() {
                ret_text = String::from("Have some events!");
            }
            let command = SendMessageCommand {
                text: ret_text,
            };
            return ProcessResult {
                frontend_command: vec![ FrontendCommand::send(command) ],
                next_state: Box::new(ReadyToProcess{})
            }
        }
        if input.starts_with("/at"){
            return self.start_calendar(id, input, db);
        }
        if input.starts_with("/delete_rep"){
            let (list_str, list_id) = ReadyToProcess::get_rep_event_list(id, db);
            if list_str.is_empty() {
                let command = SendMessageCommand {
                    text: "No current rep event".to_owned(),
                };
                return ProcessResult {
                    frontend_command: vec![ FrontendCommand::send(command) ],
                    next_state: Box::new(ReadyToProcess{})
                }
            }
            let ret_str = "Here is yout rep events list. Choose witch to delete:\n".to_string()
                + &list_str.join("\n");
            let command = SendMessageCommand {
                text: ret_str,
            };
            return ProcessResult {
                frontend_command: vec![ FrontendCommand::send(command) ],
                next_state: Box::new(RepDeleteChoose{
                    list_id : list_id
                })
            }
        }
        let (ret_text, er) = ReadyToProcess::process_text_command(id, input, db);
        let command = SendMessageCommand {
            text: ret_text,
        };
        ProcessResult {
            frontend_command: vec![ FrontendCommand::send(command) ],
            next_state: Box::new(ReadyToProcess{})
        }
    }
    
    fn process_keyboard(&self, id: i64, input: &str, db : &mut DataBase) -> ProcessResult {
        info!("State ReadyToProcess: process_keyboard function called");
        if input.starts_with("at") {
            return self.start_calendar(id, input, db);
        } else if input.starts_with("after"){

            let command = SendMessageCommand {
                text: "Ok, now write time duration.".to_owned(),
            };
                
            return ProcessResult {
                frontend_command: vec![ FrontendCommand::send(command) ],
                next_state: Box::new(AfterInput{
                    ev_text: input.to_owned(),
                })
            }
        }

        let command = SendMessageCommand {
            text: "baka".to_owned(),
        };
        return ProcessResult {
            frontend_command: vec![ FrontendCommand::send(command) ],
            next_state: Box::new(ReadyToProcess{})
        }
    }
}


#[derive(Clone, Debug)]
struct AtCalendar{
    command: AtCalendarCommand,
}

impl UserState for AtCalendar {
    fn process(&self, _id: i64, _input: &str, db : &mut DataBase) -> ProcessResult {
        panic!("Default UserState::process")
    }
    
    fn process_keyboard(&self, id: i64, callback_data: &str, _db : &mut DataBase) -> ProcessResult {
        if callback_data == "next-month" || callback_data == "previous-month" {
            let mut month = self.command.month;
            let mut year = self.command.year;
            if callback_data == "next-month" {
                month += 1;
                if month > 12 {
                    month = 1;
                    year += 1;
                }
            }
            if callback_data == "previous-month" {
                month -= 1;
                if month < 1 {
                    month = 12;
                    year -= 1;
                }
            }
            let mut new_command = self.command.clone();
            new_command.month = month;
            new_command.year = year;
            new_command.edit_msg = true;
            
            // FIXME: check to_string result
            return ProcessResult {
                frontend_command: vec![FrontendCommand::calendar(new_command.clone())],
                next_state: Box::new(AtCalendar{
                    command : new_command,
                })
            }    
        } else if callback_data.starts_with("Calendar-day-") {
            let day = callback_data[13..].parse::<i32>().unwrap();
            let command = KeyboardCommand {
                action: "Keyboard".to_owned(),
                action_type: "hour".to_owned(),
                text: "Ok, now write the time of event".to_string(), 
            };
            return ProcessResult {
                frontend_command: vec![FrontendCommand::keyboard(command)],
                next_state: Box::new(AtTimeHour{
                    year: self.command.year,
                    month: self.command.month,
                    day: day
                })
            }
        }
        let command = SendMessageCommand {
            text: "Baka!".to_string(), 
        };
        return ProcessResult {
            frontend_command: vec![FrontendCommand::send(command)],
            next_state: Box::new(ReadyToProcess{})
        }
    }
}

#[derive(Clone, Debug)]
struct AtTimeHour{
    year: i32,
    month: i32,
    day: i32,
}
impl UserState for AtTimeHour {
    fn process(&self, _id: i64, _input: &str, _db : &mut DataBase) -> ProcessResult {
        panic!("Default UserState::process")
    }
    
    fn process_keyboard(&self, _id: i64, _input: &str, _db : &mut DataBase) -> ProcessResult {
        panic!("Default UserState::process")
    }
}

#[derive(Clone, Debug)]
struct AtTimeMinute{

}

impl UserState for AtTimeMinute {
    fn process(&self, _id: i64, _input: &str, _db : &mut DataBase) -> ProcessResult {
        panic!("Default UserState::process")
    }
    
    fn process_keyboard(&self, _id: i64, _input: &str, _db : &mut DataBase) -> ProcessResult {
        panic!("Default UserState::process")
    }
}

#[derive(Clone, Debug)]
struct AtTimeText{

}

impl UserState for AtTimeText {
    fn process(&self, _id: i64, _input: &str, _db : &mut DataBase) -> ProcessResult {
        panic!("Default UserState::process")
    }
    
    fn process_keyboard(&self, _id: i64, _input: &str, _db : &mut DataBase) -> ProcessResult {
        panic!("Default UserState::process")
    }
}

#[derive(Clone, Debug)]
struct AfterInput{
    ev_text: String,
}

impl UserState for AfterInput {
    fn process(&self, id: i64, input: &str, db : &mut DataBase) -> ProcessResult {
        let message = input.to_string() + " " + &self.ev_text;
        let (ret_text, er) = ReadyToProcess::process_text_command(id, &message, db);

        let command = SendMessageCommand {
            text: "Resulting command:\n".to_string() + &message + "\n" + &ret_text,
        };
        ProcessResult {
            frontend_command: vec![FrontendCommand::send(command)],
            next_state: Box::new(ReadyToProcess{})
        }
    }
    
    fn process_keyboard(&self, _id: i64, _input: &str, _db : &mut DataBase) -> ProcessResult {
        panic!("Default UserState::process")
    }
}

#[derive(Clone, Debug)]
struct RepDeleteChoose{
    list_id : Vec<i64>,
}

impl UserState for RepDeleteChoose {
    fn process(&self, _id: i64, input: &str, db : &mut DataBase) -> ProcessResult {
        let ev_to_del : i32;
        match input.parse::<i32>(){
            // FIXME: ev - 1 ? 
            Ok(ev) => { ev_to_del = ev; }
            Err(_) => {
                let command = SendMessageCommand {
                    text: "You should write number. Operation aborted.".to_string(), 
                };
                return ProcessResult {
                    frontend_command: vec![FrontendCommand::send(command)],
                    next_state: Box::new(ReadyToProcess{})
                }
            }
        }
        if ev_to_del < 0 || ev_to_del >= self.list_id.len() as i32{
            let command = SendMessageCommand {
                text: "Number is out of limit. Operation aborted.".to_string(), 
            };
            return ProcessResult {
                frontend_command: vec![FrontendCommand::send(command)],
                next_state: Box::new(ReadyToProcess{})
            }
        }
        db.delete_rep_event(ev_to_del as i64);
        let command = SendMessageCommand {
            text: "Done.".to_string(),
        };
        return ProcessResult {
            frontend_command: vec![FrontendCommand::send(command)],
            next_state: Box::new(ReadyToProcess{})
        }
    }
    fn process_keyboard(&self, _id: i64, _input: &str, _db : &mut DataBase) -> ProcessResult {
        error!("State RepDeleteChoose: process_keyboard function called");
        let command = SendMessageCommand {
            text: "Internal logic failed".to_string(), 
        };
        return ProcessResult {
            frontend_command: vec![FrontendCommand::send(command)],
            next_state: Box::new(ReadyToProcess{})
        }
    }
}


impl Engine {
    pub fn new(open_in_memory: bool) -> Engine {
        info!("Initialize engine");
        let mut engine = Engine {
            stop_loop: AtomicBool::new(false),
            next_wakeup: None,
            frontend_callback: None,
            data_base: DataBase::new(open_in_memory),
            user_states: HashMap::new(),
        };
        
        for id in engine.get_user_chat_id_all(){
            engine.user_states.insert(id, Box::new(ReadyToProcess{}));
        }
        engine
    }

    // Normally should be run in another thread
    pub fn run(&mut self) {
        self.stop_loop.store(false, Ordering::Relaxed);
        self.loop_thread();
    }

    fn loop_thread(&mut self) {
        info!("Start engine loop");
        while !self.stop_loop.load(Ordering::Relaxed) {
            self.tick();
            thread::sleep(time::Duration::from_millis(500));
        }
    }

    pub fn handle_text_message(&mut self, uid: i64, text_message: &str) -> String {
        info!("Handle text message : {}", text_message);

        let state = self.user_states.get(&(uid as i32)).unwrap();
        let result = state.process(uid, text_message, &mut self.data_base);
        self.user_states.insert(uid as i32, result.next_state);
        self.next_wakeup = self.data_base.get_nearest_wakeup();
        
        return serde_json::to_string(&result.frontend_command).unwrap_or("".to_owned());
    }

    pub fn handle_keyboard_responce(&mut self, uid: i64, text_message: &str) -> String {
        info!("Handle Keyboard data : {}", text_message);
        let state = self.user_states.get(&(uid as i32)).unwrap();
        let result = state.process_keyboard(uid, text_message, &mut self.data_base);
        self.user_states.insert(uid as i32, result.next_state);
        self.next_wakeup = self.data_base.get_nearest_wakeup();
        return serde_json::to_string(&result.frontend_command).unwrap_or("".to_owned());
    }

    pub fn register_callback(&mut self, f: fn(String, i64)){
        self.frontend_callback = Some(f);
    }

    pub fn stop(&mut self) {
        info!("Stoping engine");
        self.stop_loop.store(false, Ordering::Relaxed);
    }

    pub fn add_user(&mut self, uid: i64, username: &str, chat_id: i64, first_name: &str, last_name: &str, tz: i32) -> bool{
        info!("Add new user id - {}, username - {}", uid, username);
        self.data_base.add_user(uid, username, chat_id, first_name, last_name, tz)
    }

    pub fn get_user_chat_id_all(&self) -> Vec<i32> {
        self.data_base.get_user_chat_id_all()
    }

    fn on_one_time_event(&self, event: OneTimeEventImpl, uid: i64){
        info!("Event time, text - <{}>", &event.event_text);
        (self.frontend_callback.unwrap())(event.event_text, uid);
    }

    fn on_repetitive_event(&self, event: RepetitiveEventImpl, uid: i64){
        info!("Event time, text - <{}>", &event.event_text);
        (self.frontend_callback.unwrap())(event.event_text, uid);
    }

    fn tick(&mut self) {
        if self.next_wakeup.is_none() {
            self.next_wakeup = self.data_base.get_nearest_wakeup();
            return;
        }
        let next_wakeup = self.next_wakeup.unwrap();

        if Utc::now() > next_wakeup {
            if let Some((command, uid)) = self.data_base.pop(next_wakeup) {
                match command {
                    Command::OneTimeEvent(ev) => self.on_one_time_event(ev, uid),
                    Command::RepetitiveEvent(ev) => self.on_repetitive_event(ev, uid),
                    Command::BadCommand => warn!("Database::pop return BadCommand"),
                }
            }
            self.next_wakeup = self.data_base.get_nearest_wakeup();
        }
    }
}

fn format_return_message_header(event_time: &DateTime<Utc>, tz: i32) -> String {
        let tz = FixedOffset::west(tz * 3600);
        let t_event = event_time.with_timezone(&tz);
        let t_now = Utc::now().with_timezone(&tz);

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
            return t_event.format("I'll remind you tomorrow at %H:%M").to_string();
        }
        t_event.format("I'll remind you %B %e at %H:%M").to_string()
    }