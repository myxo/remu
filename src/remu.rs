use anyhow::{Context, Result};
use frankenstein::{
    TelegramApi,
    client_ureq::Bot,
    methods::{EditMessageReplyMarkupParams, GetUpdatesParams, SendMessageParams},
    types::{InlineKeyboardMarkup, ReplyMarkup},
    updates::UpdateContent,
};
use log::{debug, info, warn};

use crate::{
    keyboards::{
        make_calendar_keyboard, make_hour_keyboard, make_main_action_keyboard, make_minute_keyboard,
    },
    state::FrontendCommand,
};

mod command;
mod database;
mod engine;
mod helpers;
mod keyboards;
mod prop_test;
mod sql_query;
mod state;
mod text_data;
mod time;

fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    info!("start");

    let clock = Box::new(crate::time::OsClock {});
    let mut engine = engine::Engine::new(database::DbMode::Filesystem, clock);
    let api_key = std::fs::read_to_string("token.id")?;
    let bot = Bot::new(&api_key);
    let mut front = TelegramFrontend { bot: bot.clone() };

    let mut update_params = GetUpdatesParams::builder().build();
    loop {
        let result = bot.get_updates(&update_params);

        match result {
            Ok(response) => {
                for update in response.result {
                    // dbg!(&update);
                    match update.content {
                        UpdateContent::Message(message) => {
                            /*
                            // TODO: /start
                            let user = msg.from.expect("TODO");
                            engine.add_user(
                                user.id.0 as i64,
                                user.username.as_ref().unwrap(),
                                msg.chat.id.0,
                                &user.first_name,
                                &user.last_name.as_ref().unwrap(),
                                -3,
                            );
                            */

                            let user = message.from.as_ref().expect("message has user");
                            debug!("In a text handler, user: {}", user.first_name);
                            let cmds =
                                engine.handle_text_message(user.id as i64, &message.text.unwrap());
                            match cmds {
                                Ok(cmds) => {
                                    if let Err(e) =
                                        handle_command_to_frontend(&mut front, user.id as i64, cmds)
                                    {
                                        warn!("cannot handle frontend command: {e}");
                                    }
                                }
                                Err(e) => {
                                    front.send_message(
                                        user.id as i64,
                                        &format!("Error while state machine processing:\n\n{e:#}"),
                                        None,
                                    );
                                }
                            };
                        }
                        UpdateContent::CallbackQuery(callback_query) => {
                            let user = &callback_query.from;
                            let msg = callback_query.message.as_ref().unwrap();
                            let msg = match msg {
                                frankenstein::types::MaybeInaccessibleMessage::Message(message) => message,
                                frankenstein::types::MaybeInaccessibleMessage::InaccessibleMessage(_) => {
                                    warn!("getting InaccessibleMessage in callback query: {:?}", callback_query);
                                    continue;
                                }
                            };

                            debug!("In a keyboard handler, user: {}", user.first_name);
                            let cmds = engine.handle_keyboard_responce(
                                user.id as i64,
                                msg.message_id,
                                &callback_query.data.unwrap(),
                                msg.text.as_ref().unwrap(),
                            );
                            match cmds {
                                Ok(cmds) => {
                                    if let Err(e) =
                                        handle_command_to_frontend(&mut front, user.id as i64, cmds)
                                    {
                                        warn!("cannot handle frontend command: {e}");
                                    }
                                }
                                Err(e) => {
                                    front.send_message(
                                        user.id as i64,
                                        &format!("Error while state machine processing:\n\n{e:#}"),
                                        None,
                                    );
                                }
                            };
                        }
                        _ => {
                            warn!("Unknown update type: {:?}", update)
                        }
                    }
                    update_params.offset = Some(i64::from(update.update_id) + 1);
                }
            }
            Err(error) => {
                println!("Failed to get updates: {error:?}");
            }
        }
        let events = engine.tick();
        for ev in events {
            if let Err(e) = handle_command_to_frontend(&mut front, ev.uid, ev.cmd_vec) {
                warn!("cannot handle frontend command: {e}");
            }
        }
    }
}

trait FrontendHandler {
    fn send_message(
        &mut self,
        uid: i64,
        msg: &str,
        keyboard: Option<InlineKeyboardMarkup>,
    ) -> Result<()>;

    fn delete_keyboard(&mut self, uid: i64, msg_id: i32) -> Result<()>;
}

struct TelegramFrontend {
    bot: Bot,
}

impl FrontendHandler for TelegramFrontend {
    fn send_message(
        &mut self,
        uid: i64,
        msg: &str,
        keyboard: Option<InlineKeyboardMarkup>,
    ) -> Result<()> {
        debug!("TelegramFrontend: send_message");

        let send_message_params = SendMessageParams::builder().chat_id(uid).text(msg);

        let send_message_params = if let Some(keyboard) = keyboard {
            send_message_params
                .reply_markup(ReplyMarkup::InlineKeyboardMarkup(keyboard))
                .build()
        } else {
            send_message_params.build()
        };

        self.bot
            .send_message(&send_message_params)
            .context("cannot send message")?;
        Ok(())
    }

    fn delete_keyboard(&mut self, uid: i64, msg_id: i32) -> Result<()> {
        debug!("delete keyboard for msg {msg_id}");
        let params = EditMessageReplyMarkupParams::builder()
            .chat_id(uid)
            .message_id(msg_id)
            .build();
        self.bot.edit_message_reply_markup(&params)?;
        Ok(())
    }
}

fn handle_command_to_frontend(
    front: &mut impl FrontendHandler,
    uid: i64,
    cmds: Vec<FrontendCommand>,
) -> Result<()> {
    for cmd in cmds {
        debug!("process frontend command {:?}", cmd);
        match cmd {
            state::FrontendCommand::send(send_message_command) => {
                front.send_message(uid, &send_message_command.text, None)?;
            }
            state::FrontendCommand::calendar(at_calendar_command) => {
                // TODO: edit_cur_msg
                front.send_message(
                    uid,
                    "Please, shoose a data",
                    Some(make_calendar_keyboard(
                        at_calendar_command.year,
                        at_calendar_command.month as u32,
                    )),
                )?;
            }
            state::FrontendCommand::keyboard(keyboard_command) => {
                match keyboard_command.action_type {
                    state::KeyboardCommandType::Main => {
                        front.send_message(
                            uid,
                            &keyboard_command.text,
                            Some(make_main_action_keyboard()),
                        )?;
                    }
                    state::KeyboardCommandType::Hour => {
                        front.send_message(
                            uid,
                            &keyboard_command.text,
                            Some(make_hour_keyboard()),
                        )?;
                    }
                    state::KeyboardCommandType::Minute => {
                        front.send_message(
                            uid,
                            &keyboard_command.text,
                            Some(make_minute_keyboard()),
                        )?;
                    }
                }
            }
            state::FrontendCommand::delete_message(_) => todo!(),
            state::FrontendCommand::delete_keyboard(msg_id) => {
                front.delete_keyboard(uid, msg_id)?
            }
        }
    }
    Ok(())
}
