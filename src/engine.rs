use chrono::prelude::*;
use std::thread;
use std::time;

use command::*;
use database::DataBase;



pub struct Engine {
    next_wakeup: Option<DateTime<Utc>>,
    data_base: DataBase,
    callback : Option<(fn(String))>,
    stop_loop: bool,
}

impl Engine {
    pub fn new() -> Engine {
        info!("Initialize engine");
        Engine {
            stop_loop: false,
            next_wakeup: None,
            callback: None,
            data_base: DataBase::new(),
        }
    }

    // Normally should be run in another thread
    pub fn run(&mut self) {
        self.stop_loop = false;
        self.loop_thread();
    }


    fn loop_thread(&mut self) {
        info!("Start engine loop");
        while !self.stop_loop {
            self.tick();
            thread::sleep(time::Duration::from_millis(1000));
        }
    }

    fn tick(&mut self) {
        if self.next_wakeup.is_none() {
            self.next_wakeup = self.data_base.get_nearest_wakeup();
            return;
        }
        let next_wakeup = self.next_wakeup.unwrap();

        if Utc::now() > next_wakeup {
            if let Some(command) = self.data_base.pop(next_wakeup) {
                match command {
                    Command::OneTimeEvent(ev) => self.on_one_time_event(ev),
                    Command::RepetitiveEvent(ev) => self.on_repetitive_event(ev),
                    Command::BadCommand => warn!("Database::pop return BadCommand"),
                }
            }
            self.next_wakeup = self.data_base.get_nearest_wakeup();
        }
    }

    pub fn handle_text_message(&mut self, text_message: &str) -> String {
        info!("Handle text message : {}", text_message);
        let com = parse_command(String::from(text_message));
        match com {
            Command::BadCommand => self.process_bad_command(),
            Command::OneTimeEvent(ev) => self.process_one_time_event_command(ev),
            Command::RepetitiveEvent(ev) => self.process_repetitive_event_command(ev),
        }
    }

    pub fn register_callback(&mut self, f: fn(String)){
        self.callback = Some(f);
    }

    fn process_bad_command(&self) -> String {
        String::from("Can't parse input string")
    }

    fn process_one_time_event_command(&mut self, c: OneTimeEventImpl) -> String {
        let mut return_string = self.format_return_message_header(&c.event_time);
        return_string.push('\n');
        return_string.push_str(&c.event_text);
        self.data_base.put(Command::OneTimeEvent(c));
        self.next_wakeup = self.data_base.get_nearest_wakeup();
        
        // delete newline char to write to log
        let tmp_string = str::replace(&return_string[..], "\n", " ");
        info!("Successfully process command, return string - <{}>", tmp_string);

        return_string
    }

    fn process_repetitive_event_command(&mut self, c: RepetitiveEventImpl) -> String {
        String::from("Not implemented yet")
    }

    fn on_one_time_event(&self, event: OneTimeEventImpl){
        info!("Event time, text - <{}>", &event.event_text);
        (self.callback.unwrap())(event.event_text);
    }

    fn on_repetitive_event(&self, event: RepetitiveEventImpl){
        info!("Event time, text - <{}>", &event.event_text);
        (self.callback.unwrap())(event.event_text);
    }

    fn format_return_message_header(&self, event_time: &DateTime<Utc>) -> String {
        const DEFAULT_TZ : i32 = 3;
        let tz = FixedOffset::east(DEFAULT_TZ * 3600);
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

    pub fn stop(&mut self) {
        info!("Stoping engine");
        self.stop_loop = true;
    }

    pub fn get_active_event_list(&self) -> Vec<String> {
        let mut result = Vec::new();
        let command_vector = self.data_base.get_all_active_events();
        for command in command_vector {
            match command {
                Command::OneTimeEvent(c) => {
                    let text: String = c.event_text.chars().take(40).collect();
                    let date: String = c.event_time.format("%c").to_string();
                    result.push(format!("{} : {}", date, text));
                }
                Command::BadCommand => {}
                Command::RepetitiveEvent(_ev) => {}
            }
        }
        result
    }
}
