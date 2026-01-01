use anyhow::{Result, anyhow, bail};
use chrono::prelude::*;
use log::debug;
use log::error;
use log::warn;

use crate::database::DataBase;
use crate::engine::ProcessResult;
use crate::helpers::*;
use crate::text_data;

pub(crate) const EXPECT_DURATION_MSG: &str = "Ok, now write time duration.";
pub(crate) const EXPECT_TIME_MSG: &str = "Ok, now write the time of event";
pub(crate) const EXPECT_BUTTON_PUSH: &str = "Ok, now choose";

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum MsgOperation {
    Edit { id: i32 },
    DeleteAndSendNew { old_id: i32 },
    Sendnew,
}

// FIXME: remove clone trait
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct AtCalendarCommand {
    pub(crate) year: i32,
    pub(crate) month: i32,
    pub(crate) message: String,
    pub(crate) msg_op: MsgOperation,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum KeyboardType {
    Main,
    Hour,
    Minute,
}

#[allow(non_camel_case_types)]
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum FrontendCommand {
    send {
        text: String,
    },
    calendar(AtCalendarCommand),
    keyboard {
        action_type: KeyboardType,
        text: String,
    },
    delete_message(i32),
    delete_keyboard(i32),
}

pub(crate) struct KeyboardEventData {
    pub(crate) uid: i64,
    pub(crate) msg_id: i32,
    pub(crate) actual_last_msg_id: i32,
    pub(crate) callback_data: String,
    pub(crate) msg_text: String,
}

pub(crate) struct TextEventData {
    pub(crate) uid: i64,
    pub(crate) msg_id: i32,
    pub(crate) input: String,
}

#[derive(Clone, Debug)]
pub(crate) enum StateMachine {
    ReadyToProcess,
    AtCalendar(AtCalendar),
    AtTimeHour(AtTimeHour),
    AtTimeMinute(AtTimeMinute),
    AtTimeText(AtTimeText),
    AfterInput(AfterInput),
    RepDeleteChoose(RepDeleteChoose),
}

impl StateMachine {
    pub(crate) fn process(
        &self,
        data: TextEventData,
        now: DateTime<Utc>,
        db: &mut DataBase,
    ) -> Result<ProcessResult> {
        debug!("UserState::process");
        match self {
            StateMachine::ReadyToProcess => ready_process(data, now, db),
            StateMachine::AtCalendar(_) => Err(anyhow!(
                "I am at AtCalendar state, I cannot handle text input"
            )),
            StateMachine::AtTimeHour(state) => state.process(data),
            StateMachine::AtTimeMinute(state) => state.process(data, now, db),
            StateMachine::AtTimeText(state) => state.process(data, now, db),
            StateMachine::AfterInput(state) => state.process(data, now, db),
            StateMachine::RepDeleteChoose(state) => Ok(state.process(data, db)),
        }
    }

    pub(crate) fn process_keyboard(
        &self,
        data: KeyboardEventData,
        now: DateTime<Utc>,
        db: &mut DataBase,
    ) -> Result<ProcessResult> {
        match self {
            StateMachine::ReadyToProcess => ready_process_keyboard(data, now, db),
            StateMachine::AtCalendar(state) => state.process_keyboard(data, now),
            StateMachine::AtTimeHour(state) => state.process_keyboard(data),
            StateMachine::AtTimeMinute(state) => state.process_keyboard(data, now, db),
            StateMachine::AtTimeText(state) => Ok(state.process_keyboard()),
            StateMachine::AfterInput(_) => Err(anyhow!("expect not button, but text")),
            StateMachine::RepDeleteChoose(state) => Ok(state.process_keyboard()),
        }
    }

    pub(crate) fn str(&self) -> &'static str {
        match self {
            StateMachine::ReadyToProcess => "ready_to_process",
            StateMachine::AtCalendar(_) => "at_calendar",
            StateMachine::AtTimeHour(_) => "at_time_hour",
            StateMachine::AtTimeMinute(_) => "at_time_minute",
            StateMachine::AtTimeText(_) => "at_time_text",
            StateMachine::AfterInput(_) => "after_input",
            StateMachine::RepDeleteChoose(_) => "rep_delete_choose",
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct AtCalendar {
    command: AtCalendarCommand,
    ev_text: Option<String>,
}

#[derive(Clone, Debug)]
pub(crate) struct AtTimeHour {
    year: i32,
    month: i32,
    day: i32,
    ev_text: Option<String>,
}

#[derive(Clone, Debug)]
pub(crate) struct AtTimeMinute {
    year: i32,
    month: i32,
    day: i32,
    hour: i32,
    ev_text: Option<String>,
}

#[derive(Clone, Debug)]
pub(crate) struct AtTimeText {
    year: i32,
    month: i32,
    day: i32,
    hour: i32,
    minute: i32,
}

#[derive(Clone, Debug)]
pub(crate) struct AfterInput {
    ev_text: String,
}

#[derive(Clone, Debug)]
pub(crate) struct RepDeleteChoose {
    list_id: Vec<i64>,
}

fn ready_start_calendar(
    id: i64,
    _input: &str,
    msg_text: Option<String>,
    msg_op: MsgOperation,
    db: &mut DataBase,
    now: DateTime<Utc>,
) -> ProcessResult {
    debug!("ReadyToProcess::start_Calendar");
    let tz = db.get_user_timezone(id);
    let dt = chrono::Duration::seconds((tz as i64) * 60 * 60);
    let prev = now - dt;
    let message =
        msg_text.clone().map(|m| m + "\n").unwrap_or_default() + EXPECT_BUTTON_PUSH + " date";

    let command = AtCalendarCommand {
        month: prev.month() as i32,
        year: prev.year(),
        msg_op,
        message,
    };

    ProcessResult::single(
        FrontendCommand::calendar(command.clone()),
        Some(StateMachine::AtCalendar(AtCalendar {
            command,
            ev_text: msg_text,
        })),
    )
}

fn ready_process(
    data: TextEventData,
    now: DateTime<Utc>,
    db: &mut DataBase,
) -> Result<ProcessResult> {
    if !data.input.starts_with('/') {
        if let Some(ret_text) = process_text_command(data.uid, &data.input, now, db) {
            return Ok(ProcessResult::msg_send(
                ret_text,
                StateMachine::ReadyToProcess,
            ));
        } else {
            return Ok(ProcessResult::single(
                FrontendCommand::keyboard {
                    action_type: KeyboardType::Main,
                    text: data.input.to_string(),
                },
                Some(StateMachine::ReadyToProcess),
            ));
        }
    }
    let result = match data.input.as_ref() {
        "/help more" => ProcessResult::msg_send(
            text_data::DETAILED_HELP_MESSAGE_RU.to_owned(),
            StateMachine::ReadyToProcess,
        ),

        "/help" => ProcessResult::msg_send(
            text_data::MAIN_HELP_MESSAGE_RU.to_owned(),
            StateMachine::ReadyToProcess,
        ),

        "/list" => {
            let list = get_active_event_list(data.uid, db);
            let ret_text = if list.is_empty() {
                "No current active event".to_owned()
            } else {
                list.iter()
                    .enumerate()
                    .fold(String::from(""), |s, (i, val)| {
                        s + &format!("{}) {}\n", i + 1, val)
                    })
            };
            ProcessResult::msg_send(ret_text, StateMachine::ReadyToProcess)
        }

        "/at" => ready_start_calendar(data.uid, &data.input, None, MsgOperation::Sendnew, db, now),

        "/delete_rep" => {
            let (list_str, list_id) = get_rep_event_list(data.uid, db);
            if list_str.is_empty() {
                return Ok(ProcessResult::msg_send(
                    "No current rep event".to_owned(),
                    StateMachine::ReadyToProcess,
                ));
            }
            let ret_str = "Here is yout rep events list. Choose witch to delete:\n".to_string()
                + &list_str.join("\n");
            ProcessResult::msg_send(
                ret_str,
                StateMachine::RepDeleteChoose(RepDeleteChoose { list_id }),
            )
        }

        _ => {
            bail!("Unknown command: {}", data.input);
        }
    };
    Ok(result)
}

fn ready_process_keyboard(
    data: KeyboardEventData,
    now: DateTime<Utc>,
    db: &mut DataBase,
) -> Result<ProcessResult> {
    debug!("State ReadyToProcess: process_keyboard function called");
    if data.callback_data.starts_with("at") {
        let msg_op = if data.msg_id == data.actual_last_msg_id {
            MsgOperation::Edit { id: data.msg_id }
        } else {
            MsgOperation::DeleteAndSendNew {
                old_id: data.msg_id,
            }
        };
        Ok(ready_start_calendar(
            data.uid,
            &data.callback_data,
            Some(data.msg_text.to_owned()),
            msg_op,
            db,
            now,
        ))
    } else if data.callback_data.starts_with("after") {
        Ok(ProcessResult::msg_send(
            EXPECT_DURATION_MSG.to_owned(),
            StateMachine::AfterInput(AfterInput {
                ev_text: data.msg_text,
            }),
        ))
    } else {
        let cmd_option = data.callback_data + " " + &data.msg_text;

        if let Some(ret_text) = process_text_command(data.uid, &cmd_option, now, db) {
            Ok(ProcessResult {
                frontend_command: vec![
                    FrontendCommand::send { text: ret_text },
                    FrontendCommand::delete_message(data.msg_id),
                ],
                next_state: Some(StateMachine::ReadyToProcess),
            })
        } else {
            warn!("incorrect query data, merged command: {}", cmd_option);
            bail!("incorrect query data")
        }
    }
}

impl AtCalendar {
    fn process_keyboard(
        &self,
        data: KeyboardEventData,
        now: DateTime<Utc>,
    ) -> Result<ProcessResult> {
        if data.callback_data == "next-month" || data.callback_data == "previous-month" {
            let mut month = self.command.month;
            let mut year = self.command.year;
            // FIXME: make normal time arithmetic
            if data.callback_data == "next-month" {
                month += 1;
                if month > 12 {
                    month = 1;
                    year += 1;
                }
            }
            if data.callback_data == "previous-month" {
                month -= 1;
                if month < 1 {
                    month = 12;
                    year -= 1;
                }
            }
            let mut new_command = self.command.clone();
            new_command.month = month;
            new_command.year = year;
            new_command.msg_op = if data.msg_id == data.actual_last_msg_id {
                MsgOperation::Edit { id: data.msg_id }
            } else {
                MsgOperation::DeleteAndSendNew {
                    old_id: data.msg_id,
                }
            };

            // FIXME: chck to_string result
            return Ok(ProcessResult::single(
                FrontendCommand::calendar(new_command.clone()),
                Some(StateMachine::AtCalendar(AtCalendar {
                    command: new_command,
                    ev_text: self.ev_text.as_ref().cloned(),
                })),
            ));
        } else if data.callback_data.starts_with("calendar-day-") {
            // TODO: bot.send_message(chat_id, 'Ok, ' + date.strftime(r'%b %d') + '. Now write the time of event.')
            let day = data.callback_data[13..].parse::<i32>().unwrap();
            let del_cmd = FrontendCommand::delete_message(data.msg_id);
            let keyboard_command = FrontendCommand::keyboard {
                action_type: KeyboardType::Hour,
                text: EXPECT_TIME_MSG.to_string(),
            };
            return Ok(ProcessResult {
                frontend_command: vec![del_cmd, keyboard_command],
                next_state: Some(StateMachine::AtTimeHour(AtTimeHour {
                    year: self.command.year,
                    month: self.command.month,
                    day,
                    ev_text: self.ev_text.as_ref().cloned(),
                })),
            });
        } else if data.callback_data == "today" || data.callback_data == "tomorrow" {
            // TODO: bot.send_message(chat_id, 'Ok, ' + date.strftime(r'%b %d') + '. Now write the time of event.')
            let keyboard_cmd = FrontendCommand::keyboard {
                action_type: KeyboardType::Hour,
                text: EXPECT_TIME_MSG.to_string(),
            };
            let delete_cmd = FrontendCommand::delete_message(data.msg_id);
            let now = if data.callback_data == "today" {
                now
            } else {
                now + chrono::Duration::days(1)
            };
            return Ok(ProcessResult {
                frontend_command: vec![delete_cmd, keyboard_cmd],
                next_state: Some(StateMachine::AtTimeHour(AtTimeHour {
                    year: now.year(),
                    month: now.month() as i32,
                    day: now.day() as i32,
                    ev_text: self.ev_text.as_ref().cloned(),
                })),
            });
        } else if data.callback_data == "ignore" {
            return Ok(ProcessResult::single(
                FrontendCommand::delete_message(data.msg_id),
                Some(StateMachine::ReadyToProcess),
            ));
        }
        error!("Incorrect callback data format: {}", data.callback_data);
        Err(anyhow!("Internal error: incorrect callback data format"))
    }
}

impl AtTimeHour {
    fn proceed_next_stage(&self, hour: i32, msg_id: i32) -> ProcessResult {
        let del_cmd = FrontendCommand::delete_message(msg_id);
        let keyboard_command = FrontendCommand::keyboard {
            action_type: KeyboardType::Minute,
            text: format!("Ok, {}. Now choose minute", hour),
        };

        ProcessResult {
            frontend_command: vec![del_cmd, keyboard_command],
            next_state: Some(StateMachine::AtTimeMinute(AtTimeMinute {
                year: self.year,
                month: self.month,
                day: self.day,
                hour,
                ev_text: self.ev_text.as_ref().cloned(),
            })),
        }
    }
}

impl AtTimeHour {
    fn process(&self, data: TextEventData) -> Result<ProcessResult> {
        if let Ok(hour) = data.input.parse::<i32>() {
            Ok(self.proceed_next_stage(hour, 0))
        } else {
            Err(anyhow!("Incorrect format, expect number of hours"))
        }
    }

    fn process_keyboard(&self, data: KeyboardEventData) -> Result<ProcessResult> {
        if data.callback_data.starts_with("time_hour:") {
            let hour = data.callback_data[10..].parse::<i32>().unwrap();
            Ok(self.proceed_next_stage(hour, data.msg_id))
        } else {
            error!(
                "internal error: incorrect button data in state AtTimeHour: expect time_hour, get: {}",
                data.callback_data
            );
            Err(anyhow!("Internal error: incorrect button format"))
        }
    }
}

impl AtTimeMinute {
    fn proceed_next_stage(
        &self,
        minute: i32,
        uid: i64,
        msg_id: i32,
        db: &mut DataBase,
        now: DateTime<Utc>,
    ) -> Result<ProcessResult> {
        if let Some(text) = self.ev_text.as_ref().cloned() {
            // Make result command
            let result_command = format!(
                "{}-{}-{} at {}.{} {}",
                self.day, self.month, self.year, self.hour, minute, text
            );

            let ret_text = process_text_command(uid, &result_command, now, db)
                .ok_or(anyhow!("expected time format spec"))?;
            Ok(ProcessResult::msg_send(
                ret_text,
                StateMachine::ReadyToProcess,
            ))
        } else {
            let send_command = FrontendCommand::send {
                text: "Now write event message".to_owned(),
            };
            let delete_command = FrontendCommand::delete_message(msg_id);
            Ok(ProcessResult {
                frontend_command: vec![delete_command, send_command],
                next_state: Some(StateMachine::AtTimeText(AtTimeText {
                    year: self.year,
                    month: self.month,
                    day: self.day,
                    hour: self.hour,
                    minute,
                })),
            })
        }
    }
}

impl AtTimeMinute {
    fn process(
        &self,
        data: TextEventData,
        now: DateTime<Utc>,
        db: &mut DataBase,
    ) -> Result<ProcessResult> {
        if let Ok(minute) = data.input.parse::<i32>() {
            self.proceed_next_stage(minute, data.uid, data.msg_id, db, now)
        } else {
            Ok(ProcessResult::msg_send(
                "Incorrect format, expect number of minute".to_string(),
                StateMachine::ReadyToProcess,
            ))
        }
    }

    fn process_keyboard(
        &self,
        data: KeyboardEventData,
        now: DateTime<Utc>,
        db: &mut DataBase,
    ) -> Result<ProcessResult> {
        if data.callback_data.starts_with("time_minute:") {
            let minute = data.callback_data[12..].parse::<i32>().unwrap();
            self.proceed_next_stage(minute, data.uid, data.msg_id, db, now)
        } else {
            Ok(ProcessResult::msg_send(
                "Incorrect keyboard format".to_string(),
                StateMachine::ReadyToProcess,
            ))
        }
    }
}

impl AtTimeText {
    fn process(
        &self,
        data: TextEventData,
        now: DateTime<Utc>,
        db: &mut DataBase,
    ) -> Result<ProcessResult> {
        let result_command = format!(
            "{}-{}-{} at {}.{} {}",
            self.day, self.month, self.year, self.hour, self.minute, &data.input
        );

        let ret_text = process_text_command(data.uid, &result_command, now, db)
            .ok_or(anyhow!("expect time spec format"))?;
        Ok(ProcessResult::msg_send(
            ret_text,
            StateMachine::ReadyToProcess,
        ))
    }

    fn process_keyboard(&self) -> ProcessResult {
        // This mean user just push some old button. Ignore.
        ProcessResult {
            frontend_command: vec![],
            next_state: None,
        }
    }
}

impl AfterInput {
    fn process(
        &self,
        data: TextEventData,
        now: DateTime<Utc>,
        db: &mut DataBase,
    ) -> Result<ProcessResult> {
        let message = data.input + " " + &self.ev_text;
        let ret_text = process_text_command(data.uid, &message, now, db).ok_or(anyhow!(
            "expected duration formatted string, abort operation"
        ))?;

        Ok(ProcessResult::msg_send(
            format!("Resulting command:\n{message}\n{ret_text}"),
            StateMachine::ReadyToProcess,
        ))
    }
}

impl RepDeleteChoose {
    fn process(&self, data: TextEventData, db: &mut DataBase) -> ProcessResult {
        let ev_to_del: i32 = match data.input.parse::<i32>() {
            // FIXME: ev - 1 ?
            Ok(ev) => ev,
            Err(_) => {
                return ProcessResult::msg_send(
                    "You should write number. Operation aborted.".to_string(),
                    StateMachine::ReadyToProcess,
                );
            }
        };
        if ev_to_del < 0 || ev_to_del >= self.list_id.len() as i32 {
            return ProcessResult::msg_send(
                "Number is out of limit. Operation aborted.".to_string(),
                StateMachine::ReadyToProcess,
            );
        }
        db.delete_rep_event(ev_to_del as i64);
        ProcessResult::msg_send("Done.".to_string(), StateMachine::ReadyToProcess)
    }
    fn process_keyboard(&self) -> ProcessResult {
        error!("State RepDeleteChoose: process_keyboard function called");
        ProcessResult::msg_send(
            "Internal logic failed".to_string(),
            StateMachine::ReadyToProcess,
        )
    }
}

#[cfg(test)]
pub(crate) fn is_state_message(msg: &str) -> bool {
    if msg.starts_with(EXPECT_DURATION_MSG) {
        true
    } else if msg.starts_with(EXPECT_TIME_MSG) {
        true
    } else if msg.starts_with(EXPECT_BUTTON_PUSH) {
        true
    } else {
        false
    }
}
