use anyhow::{Context, Result};
use log::{debug, error, info};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
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

pub struct Engine {
    data_base: DataBase,
    stop_loop: AtomicBool,
    user_states: HashMap<i32, UserState>,
    clock: Box<dyn Clock + Send>,
}

pub struct ProcessResult {
    pub frontend_command: Vec<FrontendCommand>,
    pub next_state: Option<UserState>,
}

impl ProcessResult {
    pub(crate) fn single(cmd: FrontendCommand, next: Option<UserState>) -> Self {
        Self {
            frontend_command: vec![cmd],
            next_state: next,
        }
    }

    pub(crate) fn msg_send(text: String, next: UserState) -> Self {
        Self {
            frontend_command: vec![FrontendCommand::send(SendMessageCommand { text })],
            next_state: Some(next),
        }
    }
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
            engine.user_states.insert(id, UserState::ReadyToProcess);
        }
        engine
    }

    #[must_use]
    pub fn handle_text_message(
        &mut self,
        uid: i64,
        text_message: &str,
    ) -> Result<Vec<FrontendCommand>> {
        debug!("Handle text message : {}", text_message);

        let state = self
            .user_states
            .get(&(uid as i32))
            .context("no /start command was processed")?;
        let data = TextEventData {
            uid,
            msg_id: 0,
            input: text_message.to_owned(),
        };
        debug!("current state: {}", state.str());
        let result = state.process(data, self.clock.now(), &mut self.data_base)?;
        let ProcessResult {
            frontend_command,
            next_state,
        } = result;
        if let Some(next_state) = next_state {
            debug!("update state to: {:?}", next_state);
            self.user_states.insert(uid as i32, next_state);
        }
        debug!("send frontend_commands: {:?}", frontend_command);
        Ok(frontend_command)
    }

    #[must_use]
    pub fn handle_keyboard_responce(
        &mut self,
        uid: i64,
        msg_id: i32,
        call_data: &str,
        msg_text: &str,
    ) -> Result<Vec<FrontendCommand>> {
        debug!("Handle Keyboard data : {}, text: {}", call_data, msg_text);
        let state = self.user_states.get(&(uid as i32)).unwrap();
        let data = KeyboardEventData {
            uid,
            msg_id,
            callback_data: call_data.to_owned(),
            msg_text: msg_text.to_owned(),
        };

        let result = match data.callback_data.as_ref() {
            "ignore" => Ok(ProcessResult {
                frontend_command: vec![],
                next_state: None,
            }),
            "Ok" => Ok(ProcessResult {
                frontend_command: vec![],
                next_state: None,
            }),
            _ => state.process_keyboard(data, self.clock.now(), &mut self.data_base),
        };
        let (front_cmd, next) = match result {
            Ok(ProcessResult {
                mut frontend_command,
                next_state,
            }) => {
                frontend_command.push(FrontendCommand::delete_keyboard(msg_id));
                (frontend_command, next_state)
            }
            Err(e) => (
                vec![FrontendCommand::send(SendMessageCommand {
                    text: format!("error while processing keyboard, return to default state: {e}"),
                })],
                Some(UserState::ReadyToProcess),
            ),
        };
        if let Some(next_state) = next {
            self.user_states.insert(uid as i32, next_state);
        }
        Ok(front_cmd)
    }

    fn stop(&mut self) {
        info!("Stoping engine");
        self.stop_loop.store(false, Ordering::Relaxed);
    }

    pub fn is_stop(&self) -> bool {
        self.stop_loop.load(Ordering::Relaxed)
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
                    .insert(uid as i32, UserState::ReadyToProcess);
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

    pub fn tick(&mut self) -> Vec<CmdFromEngine> {
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
                action_type: KeyboardCommandType::Main,
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
