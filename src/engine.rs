use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::thread;
use std::time;

use crate::command::*;
use crate::database::{DataBase, DbMode, UserInfo};
use crate::state::*;
use crate::time::Clock;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum CmdToEngine {
    AddUser {
        uid: i64,
        username: String,
        chat_id: i64,
        first_name: String,
        last_name: String,
        tz: i32,
    },
    TextMessage {
        uid: i64,
        msg_id: i64,
        message: String,
    },
    KeyboardMessage {
        uid: i64,
        msg_id: i64,
        call_data: String,
        msg_text: String,
    },
    AdvanceTime(i64), // used in test enviroment so engine known that time has been advanced
    Terminate,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CmdFromEngine {
    pub uid: i64,
    pub to_msg: Option<i64>,
    pub cmd_vec: Vec<FrontendCommand>,
}

struct Engine {
    data_base: DataBase,
    stop_loop: AtomicBool,
    user_states: HashMap<i32, Box<dyn UserState>>,
    clock: Box<dyn Clock>,
}

pub struct ProcessResult {
    pub frontend_command: Vec<FrontendCommand>,
    pub next_state: Option<Box<dyn UserState>>,
}

pub fn engine_run(
    mode: DbMode,
    clock: Box<dyn Clock + Send>,
) -> (mpsc::Sender<CmdToEngine>, mpsc::Receiver<CmdFromEngine>) {
    let (tx_to_engine, rx_in_engine) = mpsc::channel();
    let (tx_from_engine, rx_out_engine) = mpsc::channel();
    thread::spawn(move || {
        let mut engine = Engine::new(mode, clock);

        while !engine.stop_loop.load(Ordering::Relaxed) {
            for event in engine.tick() {
                tx_from_engine.send(event).unwrap();
            }

            let dt = engine.get_time_until_next_wakeup();
            if let Ok(message) = rx_in_engine.recv_timeout(dt) {
                info!("SEND: {}", serde_json::to_string(&message).unwrap());
                match message {
                    CmdToEngine::AddUser {
                        uid,
                        username,
                        chat_id,
                        first_name,
                        last_name,
                        tz,
                    } => {
                        engine.add_user(uid, &username, chat_id, &first_name, &last_name, tz);
                    }

                    CmdToEngine::TextMessage {
                        uid,
                        msg_id,
                        message,
                    } => {
                        let res = engine.handle_text_message(uid, &message);
                        if let Err(error) = tx_from_engine.send(CmdFromEngine {
                            uid,
                            to_msg: Some(msg_id),
                            cmd_vec: res,
                        }) {
                            error!("Cannot send CmdFromEngine: {}", error);
                        }
                    }

                    CmdToEngine::KeyboardMessage {
                        uid,
                        msg_id,
                        call_data,
                        msg_text,
                    } => {
                        let res =
                            engine.handle_keyboard_responce(uid, msg_id, &call_data, &msg_text);
                        if let Err(error) = tx_from_engine.send(CmdFromEngine {
                            uid,
                            to_msg: Some(msg_id),
                            cmd_vec: res,
                        }) {
                            error!("Cannot send CmdFromEngine: {}", error);
                        }
                    }

                    CmdToEngine::AdvanceTime(seconds) => {
                        engine
                            .clock
                            .set_time(engine.clock.now() + chrono::Duration::seconds(seconds));
                    }

                    CmdToEngine::Terminate => {
                        engine.stop();
                        break;
                    }
                };
            }
        }
    });

    (tx_to_engine, rx_out_engine)
}

impl Engine {
    pub fn new(mode: DbMode, clock: Box<dyn Clock + Send>) -> Engine {
        info!("Initialize engine");
        let mut engine = Engine {
            stop_loop: AtomicBool::new(false),
            data_base: DataBase::new(mode),
            user_states: HashMap::new(),
            clock,
        };

        for id in engine.get_user_chat_id_all() {
            engine.user_states.insert(id, Box::new(ReadyToProcess {}));
        }
        engine
    }

    pub fn handle_text_message(&mut self, uid: i64, text_message: &str) -> Vec<FrontendCommand> {
        debug!("Handle text message : {}", text_message);

        let state = self.user_states.get(&(uid as i32)).expect("No user state!");
        let data = TextEventData {
            uid,
            msg_id: 0,
            input: text_message.to_owned(),
        };
        let result = state.process(data, self.clock.now(), &mut self.data_base);
        if result.next_state.is_some() {
            self.user_states
                .insert(uid as i32, result.next_state.unwrap());
        }
        result.frontend_command
    }

    pub fn handle_keyboard_responce(
        &mut self,
        uid: i64,
        msg_id: i64,
        call_data: &str,
        msg_text: &str,
    ) -> Vec<FrontendCommand> {
        debug!("Handle Keyboard data : {}, text: {}", call_data, msg_text);
        let state = self.user_states.get(&(uid as i32)).unwrap();
        let data = KeyboardEventData {
            uid,
            msg_id,
            callback_data: call_data.to_owned(),
            msg_text: msg_text.to_owned(),
        };

        let result = match common_process_keyboard(&data, &mut self.data_base) {
            Some(res) => res,
            None => state.process_keyboard(data, self.clock.now(), &mut self.data_base),
        };
        if result.next_state.is_some() {
            self.user_states
                .insert(uid as i32, result.next_state.unwrap());
        }
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
    ) {
        info!("Add new user id - {}, username - {}", uid, username);
        let user_info = UserInfo {
            uid,
            name: username,
            chat_id,
            first_name,
            last_name,
            tz,
        };
        match self.data_base.add_user(user_info) {
            Ok(_) => {
                self.user_states
                    .insert(uid as i32, Box::new(ReadyToProcess {}));
            }
            Err(err_msg) => error!(
                "Can't insert user in db. UID - <{}>, username - <{}>, chat_id - <{}>. Reason: {}",
                uid, username, chat_id, err_msg
            ),
        };
    }

    pub fn get_user_chat_id_all(&self) -> Vec<i32> {
        self.data_base.get_user_chat_id_all()
    }

    fn tick(&mut self) -> Vec<CmdFromEngine> {
        let mut result: Vec<CmdFromEngine> = Vec::new();
        for ev in self
            .data_base
            .extract_events_happens_already(self.clock.now())
        {
            let event_text = match &ev.command {
                Command::OneTimeEvent(ev) => ev.event_text.clone(),
                Command::RepetitiveEvent(ev) => ev.event_text.clone(),
            };
            let cmd = FrontendCommand::keyboard(KeyboardCommand {
                action_type: "main".to_owned(),
                text: event_text,
            });
            result.push(CmdFromEngine {
                uid: ev.uid,
                to_msg: None,
                cmd_vec: vec![cmd],
            });
        }
        result
    }

    fn get_time_until_next_wakeup(&self) -> std::time::Duration {
        if let Some(ts) = self.data_base.get_nearest_wakeup() {
            ts.signed_duration_since(self.clock.now())
                .to_std()
                .unwrap_or(time::Duration::from_secs(0))
        } else {
            // If no current event, give some number. Big enough to not waste much cpu cycles,
            // but small enough for not to miss event if there is some bug here...
            time::Duration::from_secs(60)
        }
    }
}
