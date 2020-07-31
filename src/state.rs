use chrono;
use chrono::prelude::*;
use serde::{Deserialize, Serialize};

use crate::database::DataBase;
use crate::engine::ProcessResult;
use crate::helpers::*;
use crate::time::now;

// FIXME: make struct derive from String
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct SendMessageCommand {
    pub text: String,
}

// FIXME: remove clone trait
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct AtCalendarCommand {
    action_type: String,
    year: i32,
    month: i32,
    tz: i32,
    edit_cur_msg: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct KeyboardCommand {
    pub action_type: String,
    pub text: String,
}

#[allow(non_camel_case_types)]
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub enum FrontendCommand {
    send(SendMessageCommand),
    calendar(AtCalendarCommand),
    keyboard(KeyboardCommand),
    delete_message {},
    delete_keyboard {},
}

pub trait UserState {
    fn process(&self, _id: i64, _input: &str, _db: &mut DataBase) -> ProcessResult {
        panic!("Default UserState::process")
    }

    fn process_keyboard(
        &self,
        _id: i64,
        _call_data: &str,
        _msg_text: &str,
        _db: &mut DataBase,
    ) -> ProcessResult {
        panic!("Default UserState::process")
    }
}

// TODO: delete this pub (make new)
pub struct ReadyToProcess {}

#[derive(Clone, Debug)]
struct AtCalendar {
    command: AtCalendarCommand,
    ev_text: Option<String>,
}

#[derive(Clone, Debug)]
struct AtTimeHour {
    year: i32,
    month: i32,
    day: i32,
    ev_text: Option<String>,
}

#[derive(Clone, Debug)]
struct AtTimeMinute {
    year: i32,
    month: i32,
    day: i32,
    hour: i32,
    ev_text: Option<String>,
}

#[derive(Clone, Debug)]
struct AtTimeText {
    year: i32,
    month: i32,
    day: i32,
    hour: i32,
    minute: i32,
}

#[derive(Clone, Debug)]
struct AfterInput {
    ev_text: String,
}

#[derive(Clone, Debug)]
struct RepDeleteChoose {
    list_id: Vec<i64>,
}

impl ReadyToProcess {
    fn start_calendar(
        &self,
        id: i64,
        input: &str,
        msg_text: Option<String>,
        db: &mut DataBase,
    ) -> ProcessResult {
        info!("ReadyToProcess::start_Calendar");
        let tz = db.get_user_timezone(id);
        let dt = chrono::Duration::seconds((tz as i64) * 60 * 60);
        let now = now() - dt;
        let edit_cur_msg: bool = !input.starts_with("/at");

        let command = AtCalendarCommand {
            action_type: "calendar".to_string(),
            month: now.month() as i32,
            year: now.year() as i32,
            tz,
            edit_cur_msg,
        };

        ProcessResult {
            frontend_command: vec![FrontendCommand::calendar(command.clone())],
            next_state: Some(Box::new(AtCalendar {
                command,
                ev_text: msg_text,
            })),
        }
    }
}

impl UserState for ReadyToProcess {
    fn process(&self, id: i64, input: &str, db: &mut DataBase) -> ProcessResult {
        info!("ReadyToProcess::process, input: {}", input);
        if !input.starts_with("/") {
            if let Some(ret_text) = process_text_command(id, input, db) {
                let command = SendMessageCommand { text: ret_text };
                return ProcessResult {
                    frontend_command: vec![FrontendCommand::send(command)],
                    next_state: Some(Box::new(ReadyToProcess {})),
                };
            } else {
                let command = KeyboardCommand {
                    action_type: "main".to_owned(),
                    text: input.to_string(),
                };
                return ProcessResult {
                    frontend_command: vec![FrontendCommand::keyboard(command)],
                    next_state: Some(Box::new(ReadyToProcess {})),
                };
            }
        }
        match input {
            "/help more" => {
                let command = SendMessageCommand {
                    text: "Detailed help message".to_owned(),
                };
                ProcessResult {
                    frontend_command: vec![FrontendCommand::send(command)],
                    next_state: Some(Box::new(ReadyToProcess {})),
                }
            }

            "/help" => {
                let command = SendMessageCommand {
                    text: "Simple help message".to_owned(),
                };
                ProcessResult {
                    frontend_command: vec![FrontendCommand::send(command)],
                    next_state: Some(Box::new(ReadyToProcess {})),
                }
            }

            "/list" => {
                let list = get_active_event_list(id, db);
                let ret_text = if list.is_empty() {
                    "No current active event".to_owned()
                } else {
                    list.iter()
                        .enumerate()
                        .fold(String::from(""), |s, (i, val)| {
                            s + &format!("{}) {}\n", i + 1, val)
                        })
                };
                let command = SendMessageCommand { text: ret_text };
                ProcessResult {
                    frontend_command: vec![FrontendCommand::send(command)],
                    next_state: Some(Box::new(ReadyToProcess {})),
                }
            }

            "/at" => {
                return self.start_calendar(id, input, None {}, db);
            }

            "/delete_rep" => {
                let (list_str, list_id) = get_rep_event_list(id, db);
                if list_str.is_empty() {
                    let command = SendMessageCommand {
                        text: "No current rep event".to_owned(),
                    };
                    return ProcessResult {
                        frontend_command: vec![FrontendCommand::send(command)],
                        next_state: Some(Box::new(ReadyToProcess {})),
                    };
                }
                let ret_str = "Here is yout rep events list. Choose witch to delete:\n".to_string()
                    + &list_str.join("\n");
                let command = SendMessageCommand { text: ret_str };
                ProcessResult {
                    frontend_command: vec![FrontendCommand::send(command)],
                    next_state: Some(Box::new(RepDeleteChoose { list_id })),
                }
            }

            _ => {
                let command = SendMessageCommand {
                    text: format!("Unknown command: {}", input),
                };
                ProcessResult {
                    frontend_command: vec![FrontendCommand::send(command)],
                    next_state: Some(Box::new(ReadyToProcess {})),
                }
            }
        } // match input.as_ref()
    }

    fn process_keyboard(
        &self,
        id: i64,
        input: &str,
        msg_text: &str,
        db: &mut DataBase,
    ) -> ProcessResult {
        info!("State ReadyToProcess: process_keyboard function called");
        if input.starts_with("at") {
            return self.start_calendar(id, input, Some(msg_text.to_owned()), db);
        } else if input.starts_with("after") {
            let command = SendMessageCommand {
                text: "Ok, now write time duration.".to_owned(),
            };

            return ProcessResult {
                frontend_command: vec![FrontendCommand::send(command)],
                next_state: Some(Box::new(AfterInput {
                    ev_text: msg_text.to_owned(),
                })),
            };
        }

        let command = SendMessageCommand {
            text: "baka".to_owned(),
        };
        ProcessResult {
            frontend_command: vec![FrontendCommand::send(command)],
            next_state: Some(Box::new(ReadyToProcess {})),
        }
    }
}

impl UserState for AtCalendar {
    fn process(&self, _id: i64, _input: &str, _db: &mut DataBase) -> ProcessResult {
        panic!("Default UserState::process")
    }

    fn process_keyboard(
        &self,
        _id: i64,
        callback_data: &str,
        _msg_text: &str,
        _db: &mut DataBase,
    ) -> ProcessResult {
        if callback_data == "next-month" || callback_data == "previous-month" {
            let mut month = self.command.month;
            let mut year = self.command.year;
            // FIXME: make normal time arithmetic
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
            new_command.edit_cur_msg = true;

            // FIXME: check to_string result
            return ProcessResult {
                frontend_command: vec![FrontendCommand::calendar(new_command.clone())],
                next_state: Some(Box::new(AtCalendar {
                    command: new_command,
                    ev_text: self.ev_text.as_ref().cloned(),
                })),
            };
        } else if callback_data.starts_with("calendar-day-") {
            // TODO: bot.send_message(chat_id, 'Ok, ' + date.strftime(r'%b %d') + '. Now write the time of event.')
            let day = callback_data[13..].parse::<i32>().unwrap();
            let command = KeyboardCommand {
                action_type: "hour".to_owned(),
                text: "Ok, now write the time of event".to_string(),
            };
            return ProcessResult {
                frontend_command: vec![FrontendCommand::keyboard(command)],
                next_state: Some(Box::new(AtTimeHour {
                    year: self.command.year,
                    month: self.command.month,
                    day,
                    ev_text: self.ev_text.as_ref().cloned(),
                })),
            };
        } else if callback_data == "today" || callback_data == "tomorrow" {
            // TODO: bot.send_message(chat_id, 'Ok, ' + date.strftime(r'%b %d') + '. Now write the time of event.')
            let command = KeyboardCommand {
                action_type: "hour".to_owned(),
                text: "Ok, now write the time of event".to_string(),
            };
            let now = if callback_data == "today" {
                now()
            } else {
                now() + chrono::Duration::days(1)
            };
            return ProcessResult {
                frontend_command: vec![FrontendCommand::keyboard(command)],
                next_state: Some(Box::new(AtTimeHour {
                    year: now.year(),
                    month: now.month() as i32,
                    day: now.day() as i32,
                    ev_text: self.ev_text.as_ref().cloned(),
                })),
            };
        } else if callback_data == "ignore" {
            return ProcessResult {
                frontend_command: vec![FrontendCommand::delete_message {}],
                next_state: Some(Box::new(ReadyToProcess {})),
            };
        }
        let command = SendMessageCommand {
            text: "Baka!".to_string(),
        };
        ProcessResult {
            frontend_command: vec![FrontendCommand::send(command)],
            next_state: Some(Box::new(ReadyToProcess {})),
        }
    }
}

impl UserState for AtTimeHour {
    fn process(&self, _id: i64, _input: &str, _db: &mut DataBase) -> ProcessResult {
        panic!("Default UserState::process")
    }

    fn process_keyboard(
        &self,
        _id: i64,
        callback_data: &str,
        _msg_text: &str,
        _db: &mut DataBase,
    ) -> ProcessResult {
        if callback_data.starts_with("time_hour:") {
            let hour = callback_data[10..].parse::<i32>().unwrap();
            let command = KeyboardCommand {
                action_type: "minute".to_owned(),
                text: format!("Ok, {}. Now choose minute", hour),
            };

            return ProcessResult {
                frontend_command: vec![FrontendCommand::keyboard(command)],
                next_state: Some(Box::new(AtTimeMinute {
                    year: self.year,
                    month: self.month,
                    day: self.day,
                    hour,
                    ev_text: self.ev_text.as_ref().cloned(),
                })),
            };
        }
        let command = SendMessageCommand {
            text: "Baka!".to_string(),
        };
        ProcessResult {
            frontend_command: vec![FrontendCommand::send(command)],
            next_state: Some(Box::new(ReadyToProcess {})),
        }
    }
}

impl UserState for AtTimeMinute {
    fn process(&self, _id: i64, _input: &str, _db: &mut DataBase) -> ProcessResult {
        panic!("Default UserState::process")
    }

    fn process_keyboard(
        &self,
        id: i64,
        callback_data: &str,
        _msg_text: &str,
        db: &mut DataBase,
    ) -> ProcessResult {
        if callback_data.starts_with("time_minute:") {
            let minute = callback_data[12..].parse::<i32>().unwrap();

            if let Some(text) = self.ev_text.as_ref().cloned() {
                // Make result command
                let result_command = format!(
                    "{}-{}-{} at {}.{} {}",
                    self.day,
                    self.month,
                    self.year,
                    self.hour,
                    minute,
                    text
                );

                let ret_text = process_text_command(id, &result_command, db).unwrap();
                let command = SendMessageCommand { text: ret_text };
                return ProcessResult {
                    frontend_command: vec![FrontendCommand::send(command)],
                    next_state: Some(Box::new(ReadyToProcess {})),
                };
            } else {
                let command = SendMessageCommand { text: "Now write event message".to_owned() };
                return ProcessResult {
                    frontend_command: vec![FrontendCommand::send(command)],
                    next_state: Some(Box::new(AtTimeText {
                        year: self.year,
                        month: self.month,
                        day: self.day,
                        hour: self.hour,
                        minute,
                    })),
                };
            }

            // let command = KeyboardCommand {
            //     action_type: "minute".to_owned(),
            //     text: format!("Ok, {}. Now chiise minute", hour),
            // };

            // return ProcessResult {
            //     frontend_command: vec![FrontendCommand::keyboard(command)],
            //     next_state: Box::new(AtTimeMinute{
            //         year: self.year,
            //         month: self.month,
            //         day: self.day,
            //         hour: hour,
            //     })
            // }
        }
        let command = SendMessageCommand {
            text: "Baka!".to_string(),
        };
        ProcessResult {
            frontend_command: vec![FrontendCommand::send(command)],
            next_state: Some(Box::new(ReadyToProcess {})),
        }
    }
}

impl UserState for AtTimeText {
    fn process(&self, id: i64, input: &str, db: &mut DataBase) -> ProcessResult {

        let result_command = format!(
            "{}-{}-{} at {}.{} {}",
            self.day,
            self.month,
            self.year,
            self.hour,
            self.minute,
            input
        );

        let ret_text = process_text_command(id, &result_command, db).unwrap();
        let command = SendMessageCommand { text: ret_text };
        return ProcessResult {
            frontend_command: vec![FrontendCommand::send(command)],
            next_state: Some(Box::new(ReadyToProcess {})),
        };
    }

    fn process_keyboard(
        &self,
        _id: i64,
        _callback_data: &str,
        _msg_text: &str,
        _db: &mut DataBase,
    ) -> ProcessResult {
        // This mean user just push some old button. Ignore.
        ProcessResult {
            frontend_command: vec![],
            next_state: None,
        }
    }
}

impl UserState for AfterInput {
    fn process(&self, id: i64, input: &str, db: &mut DataBase) -> ProcessResult {
        let message = input.to_string() + " " + &self.ev_text;
        let ret_text = process_text_command(id, &message, db).unwrap();

        let command = SendMessageCommand {
            text: "Resulting command:\n".to_string() + &message + "\n" + &ret_text,
        };
        ProcessResult {
            frontend_command: vec![FrontendCommand::send(command)],
            next_state: Some(Box::new(ReadyToProcess {})),
        }
    }

    fn process_keyboard(
        &self,
        _id: i64,
        _input: &str,
        _msg_text: &str,
        _db: &mut DataBase,
    ) -> ProcessResult {
        panic!("Default UserState::process")
    }
}

impl UserState for RepDeleteChoose {
    fn process(&self, _id: i64, input: &str, db: &mut DataBase) -> ProcessResult {
        let ev_to_del: i32;
        match input.parse::<i32>() {
            // FIXME: ev - 1 ?
            Ok(ev) => {
                ev_to_del = ev;
            }
            Err(_) => {
                let command = SendMessageCommand {
                    text: "You should write number. Operation aborted.".to_string(),
                };
                return ProcessResult {
                    frontend_command: vec![FrontendCommand::send(command)],
                    next_state: Some(Box::new(ReadyToProcess {})),
                };
            }
        }
        if ev_to_del < 0 || ev_to_del >= self.list_id.len() as i32 {
            let command = SendMessageCommand {
                text: "Number is out of limit. Operation aborted.".to_string(),
            };
            return ProcessResult {
                frontend_command: vec![FrontendCommand::send(command)],
                next_state: Some(Box::new(ReadyToProcess {})),
            };
        }
        db.delete_rep_event(ev_to_del as i64);
        let command = SendMessageCommand {
            text: "Done.".to_string(),
        };
        ProcessResult {
            frontend_command: vec![FrontendCommand::send(command)],
            next_state: Some(Box::new(ReadyToProcess {})),
        }
    }
    fn process_keyboard(
        &self,
        _id: i64,
        _input: &str,
        _msg_text: &str,
        _db: &mut DataBase,
    ) -> ProcessResult {
        error!("State RepDeleteChoose: process_keyboard function called");
        let command = SendMessageCommand {
            text: "Internal logic failed".to_string(),
        };
        return ProcessResult {
            frontend_command: vec![FrontendCommand::send(command)],
            next_state: Some(Box::new(ReadyToProcess {})),
        };
    }
}

// Some keyboard command should be processed regardless current state
// This fucntion handle this kind of commands
pub fn common_process_keyboard(
    _id: i64,
    callback_data: &str,
    _msg_text: &str,
    _db: &mut DataBase,
) -> Option<ProcessResult> {

    // TODO: may come normal command here (from main keyboard)

    if callback_data == "ignore" {
        Some(ProcessResult {
            frontend_command: vec![FrontendCommand::delete_message {}],
            next_state: None,
        })
    } else if callback_data == "Ok" {
        Some(ProcessResult {
            frontend_command: vec![FrontendCommand::delete_keyboard {}],
            next_state: None,
        })
    } else {
        None
    }
}
