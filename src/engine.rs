use anyhow::{Result, anyhow};
use chrono::{DateTime, Utc};
use log::{debug, info};
use std::collections::HashMap;

use crate::command::*;
use crate::database::{DataBase, DbMode, UserInfo};
use crate::keyboards::OK_BUTTON_TEXT;
use crate::state::*;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct CmdFromEngine {
    pub(crate) uid: i64,
    pub(crate) to_msg: Option<i64>,
    pub(crate) cmd_vec: Vec<FrontendCommand>,
}

#[derive(Clone, Debug)]
struct UserState {
    last_msg_id: i32,
    state: StateMachine,
}

pub(crate) struct Engine {
    db: DataBase,
    user_states: HashMap<i64, UserState>,
}

pub(crate) struct ProcessResult {
    pub(crate) frontend_command: Vec<FrontendCommand>,
    pub(crate) next_state: Option<StateMachine>,
}

impl ProcessResult {
    pub(crate) fn single(cmd: FrontendCommand, next: Option<StateMachine>) -> Self {
        Self {
            frontend_command: vec![cmd],
            next_state: next,
        }
    }

    pub(crate) fn msg_send(text: String, next: StateMachine) -> Self {
        Self {
            frontend_command: vec![FrontendCommand::send { text }],
            next_state: Some(next),
        }
    }
}

impl Engine {
    pub(crate) fn new(mode: DbMode) -> Engine {
        info!("Initialize engine");
        let mut engine = Engine {
            db: DataBase::new(mode),
            user_states: HashMap::new(),
        };

        for id in engine.get_user_chat_id_all() {
            engine.user_states.insert(
                id,
                UserState {
                    last_msg_id: 0,
                    state: StateMachine::ReadyToProcess,
                },
            );
        }
        engine
    }

    pub(crate) fn handle_text_message(
        &mut self,
        uid: i64,
        msg_id: i32,
        text_message: &str,
        now: DateTime<Utc>,
    ) -> (Result<()>, Vec<FrontendCommand>) {
        let state = self.user_states.get(&uid);
        if state.is_none() {
            let err_msg = "no /start command was processed";
            return (
                Err(anyhow!(err_msg)),
                vec![FrontendCommand::send {
                    text: err_msg.to_owned(),
                }],
            );
        };
        let state = state.unwrap();
        let data = TextEventData {
            uid,
            msg_id: 0,
            input: text_message.to_owned(),
        };
        info!(
            "handle text message, current state: {} user: {uid}, msg_id: {msg_id}",
            state.state.str()
        );
        let result = state.state.process(data, now, &mut self.db);
        match result {
            Ok(ProcessResult {
                frontend_command,
                next_state,
            }) => {
                debug!(
                    "Finish processing, next_state: {next_state:?}, frontend_commands: {frontend_command:?}"
                );
                self.user_states.insert(
                    uid,
                    UserState {
                        last_msg_id: msg_id,
                        state: next_state.unwrap_or(state.state.clone()),
                    },
                );
                (Ok(()), frontend_command)
            }
            Err(e) => {
                debug!("Finish processing with error (return to default state): {e}");
                self.user_states.insert(
                    uid,
                    UserState {
                        last_msg_id: msg_id,
                        state: StateMachine::ReadyToProcess,
                    },
                );
                let err_msg = format!("cannot handle text, return to default state: {e}");
                (
                    Err(anyhow!(err_msg.clone())),
                    vec![FrontendCommand::send { text: err_msg }],
                )
            }
        }
    }

    pub(crate) fn handle_keyboard_responce(
        &mut self,
        uid: i64,
        msg_id: i32,
        call_data: &str,
        msg_text: &str,
        now: DateTime<Utc>,
    ) -> (Result<()>, Vec<FrontendCommand>) {
        info!("handle button push for {uid}, msg_id: {msg_id}");
        debug!("Handle Keyboard data : {}, text: {}", call_data, msg_text);
        let state = self.user_states.get(&uid).unwrap().clone();
        let data = KeyboardEventData {
            uid,
            msg_id,
            actual_last_msg_id: state.last_msg_id,
            callback_data: call_data.to_owned(),
            msg_text: msg_text.to_owned(),
        };

        let result = match data.callback_data.as_ref() {
            "ignore" => Ok(ProcessResult {
                frontend_command: vec![FrontendCommand::delete_keyboard(msg_id)],
                next_state: Some(StateMachine::ReadyToProcess),
            }),
            OK_BUTTON_TEXT => Ok(ProcessResult {
                frontend_command: vec![FrontendCommand::delete_keyboard(msg_id)],
                next_state: Some(StateMachine::ReadyToProcess),
            }),
            _ => state.state.process_keyboard(data, now, &mut self.db),
        };
        match result {
            Ok(ProcessResult {
                frontend_command,
                next_state,
            }) => {
                debug!(
                    "Finish processing button press, next state: {:?}, frontend_commands: {:?}",
                    next_state, frontend_command
                );
                self.user_states.insert(
                    uid,
                    UserState {
                        last_msg_id: msg_id,
                        state: next_state.unwrap_or(state.state),
                    },
                );
                (Ok(()), frontend_command)
            }
            Err(e) => {
                debug!("Finish processing with error (return to default state): {e}");
                self.user_states.insert(
                    uid,
                    UserState {
                        last_msg_id: msg_id,
                        state: StateMachine::ReadyToProcess,
                    },
                );
                let err_msg = format!("cannot handle button press, return to default state: {e}");
                (
                    Err(anyhow!(err_msg.clone())),
                    vec![
                        FrontendCommand::send { text: err_msg },
                        FrontendCommand::delete_keyboard(state.last_msg_id),
                    ],
                )
            }
        }
    }

    pub(crate) fn add_user(&mut self, user_info: UserInfo, msg_id: i32) -> Result<()> {
        info!(
            "Add new user id - {}, username - {} {}",
            user_info.chat_id, user_info.first_name, user_info.last_name
        );
        let chat_id = user_info.chat_id;
        self.db.add_user(user_info)?;
        self.user_states.insert(
            chat_id,
            UserState {
                last_msg_id: msg_id,
                state: StateMachine::ReadyToProcess,
            },
        );
        Ok(())
    }

    pub(crate) fn get_user_chat_id_all(&self) -> Vec<i64> {
        self.db.get_user_chat_id_all()
    }

    pub(crate) fn tick(&mut self, now: DateTime<Utc>) -> Vec<CmdFromEngine> {
        let mut result: Vec<CmdFromEngine> = Vec::new();
        for ev in self.db.extract_events_happens_already(now) {
            let event_text = match &ev.command {
                Command::OneTimeEvent(ev) => ev.event_text.clone(),
                Command::RepetitiveEvent(ev) => ev.event_text.clone(),
            };
            let cmd = FrontendCommand::keyboard {
                action_type: KeyboardType::Main,
                text: event_text,
            };
            result.push(CmdFromEngine {
                uid: ev.uid,
                to_msg: None,
                cmd_vec: vec![cmd],
            });
        }
        result
    }

    pub(crate) fn get_time_until_next_wakeup(
        &self,
        now: chrono::DateTime<chrono::Utc>,
    ) -> Option<std::time::Duration> {
        self.db.get_nearest_wakeup().map(|ts| {
            ts.signed_duration_since(now)
                .to_std()
                .unwrap_or(std::time::Duration::ZERO) // we got negative, so we should wake up immediately
        })
    }
}
