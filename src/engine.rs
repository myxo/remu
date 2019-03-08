use chrono;
use chrono::prelude::*;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time;

use crate::command::*;
use crate::database::DataBase;
use crate::state::*;

pub struct Engine {
    next_wakeup: Option<DateTime<Utc>>,
    data_base: DataBase,
    frontend_callback: Option<(fn(String, i64))>,
    stop_loop: AtomicBool,
    user_states: HashMap<i32, Box<UserState>>,
}

pub struct ProcessResult {
    pub frontend_command: Vec<FrontendCommand>,
    pub next_state: Option<Box<UserState>>,
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

        for id in engine.get_user_chat_id_all() {
            engine.user_states.insert(id, Box::new(ReadyToProcess {}));
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
        if result.next_state.is_some() {
            self.user_states
                .insert(uid as i32, result.next_state.unwrap());
        }
        self.next_wakeup = self.data_base.get_nearest_wakeup();

        serde_json::to_string(&result.frontend_command).unwrap_or("".to_owned())
    }

    pub fn handle_keyboard_responce(
        &mut self,
        uid: i64,
        call_data: &str,
        msg_text: &str,
    ) -> String {
        info!("Handle Keyboard data : {}, text: {}", call_data, msg_text);
        let state = self.user_states.get(&(uid as i32)).unwrap();

        let result = match common_process_keyboard(uid, call_data, msg_text, &mut self.data_base) {
            Some(res) => res,
            None => state.process_keyboard(uid, call_data, msg_text, &mut self.data_base),
        };
        if result.next_state.is_some() {
            self.user_states
                .insert(uid as i32, result.next_state.unwrap());
        }
        self.next_wakeup = self.data_base.get_nearest_wakeup();
        serde_json::to_string(&result.frontend_command).unwrap_or("".to_owned())
    }

    pub fn register_callback(&mut self, f: fn(String, i64)) {
        self.frontend_callback = Some(f);
    }

    pub fn stop(&mut self) {
        info!("Stoping engine");
        self.stop_loop.store(false, Ordering::Relaxed);
    }

    pub fn add_user(
        &mut self,
        uid: i64,
        username: &str,
        chat_id: i64,
        first_name: &str,
        last_name: &str,
        tz: i32,
    ) -> bool {
        info!("Add new user id - {}, username - {}", uid, username);
        let result = self.data_base
            .add_user(uid, username, chat_id, first_name, last_name, tz);
        if result {
            self.user_states.insert(uid as i32, Box::new(ReadyToProcess {}));
        }
        result
    }

    pub fn get_user_chat_id_all(&self) -> Vec<i32> {
        self.data_base.get_user_chat_id_all()
    }

    fn on_one_time_event(&mut self, event: OneTimeEventImpl, uid: i64) {
        info!("Event time, text - <{}>", &event.event_text);
        (self.frontend_callback.unwrap())(event.event_text, uid);
    }

    fn on_repetitive_event(&mut self, event: RepetitiveEventImpl, uid: i64) {
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
                    Command::BadCommand => error!("Database::pop return BadCommand"),
                }
            }
            self.next_wakeup = self.data_base.get_nearest_wakeup();
        }
    }
}
