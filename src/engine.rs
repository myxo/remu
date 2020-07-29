use chrono;
use chrono::prelude::*;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::thread;
use std::time;
use serde::{Deserialize, Serialize};

use crate::command::*;
use crate::database::{DataBase, DbMode, UserInfo};
use crate::state::*;
use crate::time::now;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum CmdToEngine {
    AddUser {uid: i64, username: String, chat_id: i64, first_name: String, last_name: String, tz: i32},
    TextMessage {uid: i64, msg_id: i64, message: String},
    KeyboardMessage {uid: i64, msg_id: i64, call_data: String, msg_text: String},
    Terminate,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CmdFromEngine {
    pub uid: i64,
    pub to_msg: Option<i64>,
    pub cmd_vec: Vec<FrontendCommand>,
}

struct Engine {
    next_wakeup: Option<DateTime<Utc>>,
    data_base: DataBase,
    stop_loop: AtomicBool,
    user_states: HashMap<i32, Box<dyn UserState>>,
}

pub struct ProcessResult {
    pub frontend_command: Vec<FrontendCommand>,
    pub next_state: Option<Box<dyn UserState>>,
}

pub fn engine_run(mode: DbMode) -> (mpsc::Sender<CmdToEngine>, mpsc::Receiver<CmdFromEngine>) 
{
    let (tx_to_engine, rx_in_engine) = mpsc::channel();
    let (tx_from_engine, rx_out_engine) = mpsc::channel();
    thread::spawn(move || {
        let mut engine = Engine::new(mode);

        while !engine.stop_loop.load(Ordering::Relaxed) {
            if let Some(cmd) = engine.tick() {
                tx_from_engine.send(cmd).unwrap();
            }

            if let Ok(message) = rx_in_engine.try_recv(){
                info!("{}", serde_json::to_string(&message).unwrap());
                match message {
                    CmdToEngine::AddUser {uid, username, chat_id, first_name, last_name, tz} => { 
                        engine.add_user(uid, &username, chat_id, &first_name, &last_name, tz);
                    },

                    CmdToEngine::TextMessage {uid, msg_id, message} => { 
                        let res = engine.handle_text_message(uid, &message);
                        tx_from_engine.send(CmdFromEngine{uid, to_msg: Some(msg_id), cmd_vec: res}).unwrap();
                    },

                    CmdToEngine::KeyboardMessage {uid, msg_id, call_data, msg_text} => {
                        let res = engine.handle_keyboard_responce(uid, &call_data, &msg_text);
                        tx_from_engine.send(CmdFromEngine{uid, to_msg: Some(msg_id), cmd_vec: res}).unwrap();
                    }

                    CmdToEngine::Terminate => {
                        engine.stop();
                        break;
                    }
                };
            }

            thread::sleep(time::Duration::from_millis(500));
        }
    });

    (tx_to_engine, rx_out_engine)
}

impl Engine {
    pub fn new(mode: DbMode) -> Engine {
        info!("Initialize engine");
        let mut engine = Engine {
            stop_loop: AtomicBool::new(false),
            next_wakeup: None,
            data_base: DataBase::new(mode),
            user_states: HashMap::new(),
        };

        for id in engine.get_user_chat_id_all() {
            engine.user_states.insert(id, Box::new(ReadyToProcess {}));
        }
        engine
    }

    pub fn handle_text_message(&mut self, uid: i64, text_message: &str) -> Vec<FrontendCommand> {
        info!("Handle text message : {}", text_message);

        let state = self.user_states.get(&(uid as i32)).expect("No user state!");
        let result = state.process(uid, text_message, &mut self.data_base);
        if result.next_state.is_some() {
            self.user_states
                .insert(uid as i32, result.next_state.unwrap());
        }
        self.next_wakeup = self.data_base.get_nearest_wakeup();

        result.frontend_command
    }

    pub fn handle_keyboard_responce(
        &mut self,
        uid: i64,
        call_data: &str,
        msg_text: &str,
    ) -> Vec<FrontendCommand> {
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
        result.frontend_command
    }

    fn stop(&mut self) {
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
    ){
        info!("Add new user id - {}, username - {}", uid, username);
        let user_info = UserInfo{uid, name:username, chat_id, first_name, last_name, tz};
        match self.data_base.add_user(user_info) {
            Ok(_) => {
                self.user_states.insert(uid as i32, Box::new(ReadyToProcess {})); 
            },
            Err(err_msg) => error!(
                    "Can't insert user in db. UID - <{}>, username - <{}>, chat_id - <{}>. Reason: {}",
                    uid,
                    username,
                    chat_id,
                    err_msg
            ),
        };
    }

    pub fn get_user_chat_id_all(&self) -> Vec<i32> {
        self.data_base.get_user_chat_id_all()
    }

    fn tick(&mut self) -> Option<CmdFromEngine> {
        if self.next_wakeup.is_none() {
            self.next_wakeup = self.data_base.get_nearest_wakeup();
            return None;
        }
        let next_wakeup = self.next_wakeup.unwrap();

        if now() >= next_wakeup {
            self.next_wakeup = self.data_base.get_nearest_wakeup();
            if let Some((command, uid)) = self.data_base.pop(next_wakeup) {
                let event_text = match command {
                    Command::OneTimeEvent(ev) => ev.event_text,
                    Command::RepetitiveEvent(ev) => ev.event_text,
                };
                info!("Event time, text - <{}>", &event_text);
                let cmd = FrontendCommand::keyboard(KeyboardCommand{
                    action_type: "main".to_owned(),
                    text: event_text,
                });
                return Some(CmdFromEngine{
                    uid,
                    to_msg: None,
                    cmd_vec: vec![cmd],
                });
            };
        };
        None
    }
}
