use chrono::prelude::*;
use chrono;
use std::thread;
use std::time;
use std::sync::atomic::{AtomicBool, Ordering};

use command::*;
use database::DataBase;



pub struct Engine {
    next_wakeup: Option<DateTime<Utc>>,
    data_base: DataBase,
    callback : Option<(fn(String, i64))>,
    stop_loop: AtomicBool,
}

impl Engine {
    pub fn new(open_in_memory: bool) -> Engine {
        info!("Initialize engine");
        Engine {
            stop_loop: AtomicBool::new(false),
            next_wakeup: None,
            callback: None,
            data_base: DataBase::new(open_in_memory),
        }
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

    pub fn handle_text_message(&mut self, uid: i64, text_message: &str) -> (String, i32) {
        info!("Handle text message : {}", text_message);
        let tz = self.data_base.get_user_timezone(uid);
        let com = parse_command(String::from(text_message), tz);
        match com {
            Command::BadCommand => self.process_bad_command(),
            Command::OneTimeEvent(ev) => self.process_one_time_event_command(uid, ev),
            Command::RepetitiveEvent(ev) => self.process_repetitive_event_command(uid, ev),
        }
    }

    pub fn register_callback(&mut self, f: fn(String, i64)){
        self.callback = Some(f);
    }


    pub fn stop(&mut self) {
        info!("Stoping engine");
        self.stop_loop.store(false, Ordering::Relaxed);
    }

    pub fn get_active_event_list(&self, uid: i64) -> Vec<String> {
        let mut result = Vec::new();
        let command_vector = self.data_base.get_all_active_events(uid);
        let tz = self.data_base.get_user_timezone(uid) as i64;
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

    pub fn get_rep_event_list(&self, uid: i64) -> Vec<(String, i64)> {
        let mut result = Vec::new();
        let command_vector = self.data_base.get_all_rep_events(uid);
        // const DEFAULT_TZ: i64 = 3;
        // let dt = chrono::Duration::seconds(DEFAULT_TZ * 60 * 60);
        for line in command_vector {
            let id: i64 = line.1;
            let command = line.0;
            match command {
                Command::RepetitiveEvent(ev) => {
                    let text: String = ev.event_text.chars().take(40).collect();
                    result.push((format!("{}", text), id));
                }
                Command::BadCommand => {}
                Command::OneTimeEvent(_ev) => {}
            }
        }
        result
    }

    pub fn delete_rep_event(&mut self, event_id: i64) -> bool{
        info!("Delete {} rep event", event_id);
        self.data_base.delete_rep_event(event_id)
    }

    pub fn add_user(&mut self, uid: i64, username: &str, chat_id: i64, first_name: &str, last_name: &str, tz: i32) -> bool{
        info!("Add new user id - {}, username - {}", uid, username);
        self.data_base.add_user(uid, username, chat_id, first_name, last_name, tz)
    }

    pub fn get_user_chat_id_all(&self) -> Vec<i32> {
        self.data_base.get_user_chat_id_all()
    }

    pub fn get_user_groups(&self, uid: i64) -> Vec<(String, i64)> {
        error!("Called deprecated group function!");
        self.data_base.get_groups_names(uid)
    }

    pub fn add_user_group(&self, uid: i64, group_name: &str) -> bool{
        error!("Called deprecated group function!");
        if group_name == ""{
            warn!("{} try to add group without name", uid);
            return false;
        }
        info!("<{}> add user group {}.", uid, group_name);
        self.data_base.add_group(uid, group_name)
    }

    pub fn delete_user_group(&self, gid: i64) -> bool{
        error!("Called deprecated group function!");
        info!("Delete {} group", gid);
        self.data_base.delete_group(gid)
    }

    pub fn get_group_items(&self, gid: i64) -> Vec<(String, i64)> {
        error!("Called deprecated group function!");
        self.data_base.get_group_items(gid)
    }

    pub fn add_group_item(&self, gid: i64, group_item: &str) -> bool{
        error!("Called deprecated group function!");
        info!("Add item {} to {}", group_item, gid);
        self.data_base.add_group_item(gid, group_item)
    }

    pub fn delete_group_item(&self, id: i64) -> bool{
        error!("Called deprecated group function!");
        info!("Delete group item {} ", id);
        self.data_base.delete_group_item(id)
    }

    fn process_bad_command(&self) -> (String, i32) {
        (String::from(""), 1)
    }

    fn process_one_time_event_command(&mut self, uid:i64, c: OneTimeEventImpl) -> (String, i32) {
        let tz = self.data_base.get_user_timezone(uid);
        let mut return_string = self.format_return_message_header(&c.event_time, tz);
        return_string.push('\n');
        return_string.push_str(&c.event_text);
        self.data_base.put(uid, Command::OneTimeEvent(c));
        self.next_wakeup = self.data_base.get_nearest_wakeup();
        
        // delete newline char to write to log
        let tmp_string = str::replace(&return_string[..], "\n", " ");
        info!("Successfully process command, return string - <{}>", tmp_string);

        (return_string, 0)
    }

    fn process_repetitive_event_command(&mut self, uid: i64, c: RepetitiveEventImpl) -> (String, i32) {
        let tz = self.data_base.get_user_timezone(uid);
        let mut return_string = self.format_return_message_header(&c.event_start_time, tz);
        return_string.push('\n');
        return_string.push_str(&c.event_text);
        self.data_base.put(uid, Command::RepetitiveEvent(c));
        self.next_wakeup = self.data_base.get_nearest_wakeup();
        
        // delete newline char to write to log
        let tmp_string = str::replace(&return_string[..], "\n", " ");
        info!("Successfully process command, return string - <{}>", tmp_string);

        (return_string, 0)
    }

    fn on_one_time_event(&self, event: OneTimeEventImpl, uid: i64){
        info!("Event time, text - <{}>", &event.event_text);
        (self.callback.unwrap())(event.event_text, uid);
    }

    fn on_repetitive_event(&self, event: RepetitiveEventImpl, uid: i64){
        info!("Event time, text - <{}>", &event.event_text);
        (self.callback.unwrap())(event.event_text, uid);
    }

    fn format_return_message_header(&self, event_time: &DateTime<Utc>, tz: i32) -> String {
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
