use anyhow::{Context, Result};
use frankenstein::{
    TelegramApi,
    client_ureq::Bot,
    methods::{
        DeleteMessageParams, EditMessageReplyMarkupParams, EditMessageTextParams, GetUpdatesParams,
        SendMessageParams,
    },
    types::{InlineKeyboardMarkup, ReplyMarkup},
    updates::UpdateContent,
};
use log::{debug, error, info, warn};

use crate::{
    engine::Engine,
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

fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("remu=debug"))
        .init();
    info!("start");

    let mut engine = engine::Engine::new(database::DbMode::Filesystem);
    let api_key = std::fs::read_to_string("token.id")?;
    let bot = Bot::new(api_key.trim());
    let mut front = TelegramFrontend { bot: bot.clone() };

    let mut update_params = GetUpdatesParams::builder().build();
    loop {
        update_params.timeout = engine
            .get_time_until_next_wakeup(chrono::Utc::now())
            .map(|dur| dur.as_secs() as u32);
        let result = bot.get_updates(&update_params);

        match result {
            Ok(response) => {
                for update in response.result {
                    let update_id = update.update_id;
                    process_event(update, &mut engine, &mut front, chrono::Utc::now());
                    update_params.offset = Some(i64::from(update_id) + 1);
                }
            }
            Err(error) => {
                println!("Failed to get updates: {error:?}");
            }
        }
        let events = engine.tick(chrono::Utc::now());
        for ev in events {
            if let Err(e) = handle_command_to_frontend(&mut front, ev.uid, ev.cmd_vec) {
                warn!("cannot handle frontend command: {e}");
            }
        }
    }
}

fn process_event(
    update: frankenstein::updates::Update,
    engine: &mut Engine,
    front: &mut impl FrontendHandler,
    now: chrono::DateTime<chrono::Utc>,
) {
    match update.content {
        UpdateContent::Message(message) => {
            let user = message.from.as_ref().expect("message has user");
            let msg_text = message.text.as_ref().unwrap();
            match msg_text.as_str() {
                "/start" => {
                    // very special case
                    let res = engine.add_user(
                        user.id as i64,
                        user.username.as_ref().unwrap(),
                        message.chat.id,
                        &user.first_name,
                        user.last_name.as_ref().unwrap(),
                        -3,
                    );
                    if let Err(e) = res {
                        error!(
                            "cannot add user, UID - <{}>, username - <{:?}>, chat_id - <{}>. Reason: {e:#}",
                            user.id, user.username, message.chat.id
                        );
                        let _ = front.send_message(
                            user.id as i64,
                            &format!("cannot process message: {e}"),
                            None,
                        );
                    }
                }
                _ => {
                    match engine.handle_text_message(user.id as i64, msg_text, now) {
                        Ok(cmds) => {
                            if let Err(e) = handle_command_to_frontend(front, user.id as i64, cmds)
                            {
                                warn!("cannot handle frontend command: {e}");
                            }
                        }
                        Err(e) => {
                            let _ = front.send_message(
                                user.id as i64,
                                &format!("Error while state machine processing:\n\n{e:#}"),
                                None,
                            );
                        }
                    };
                }
            }
        }
        UpdateContent::CallbackQuery(callback_query) => {
            let user = &callback_query.from;
            let msg = callback_query.message.as_ref().unwrap();
            let msg = match msg {
                frankenstein::types::MaybeInaccessibleMessage::Message(message) => message,
                frankenstein::types::MaybeInaccessibleMessage::InaccessibleMessage(_) => {
                    warn!(
                        "getting InaccessibleMessage in callback query: {:?}",
                        callback_query
                    );
                    return;
                }
            };

            debug!("In a keyboard handler, user: {}", user.first_name);
            let cmds = engine.handle_keyboard_responce(
                user.id as i64,
                msg.message_id,
                &callback_query.data.unwrap(),
                msg.text.as_ref().unwrap(),
                now,
            );
            match cmds {
                Ok(cmds) => {
                    if let Err(e) = handle_command_to_frontend(front, user.id as i64, cmds) {
                        warn!("cannot handle frontend command: {e}");
                    }
                }
                Err(e) => {
                    let _ = front.send_message(
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
}

trait FrontendHandler {
    fn send_message(
        &mut self,
        uid: i64,
        msg: &str,
        keyboard: Option<InlineKeyboardMarkup>,
    ) -> Result<()>;

    fn edit_message(
        &mut self,
        uid: i64,
        mid: i32,
        msg: &str,
        keyboard: Option<InlineKeyboardMarkup>,
    ) -> Result<()>;

    fn delete_keyboard(&mut self, uid: i64, msg_id: i32) -> Result<()>;
    fn delete_message(&mut self, uid: i64, msg_id: i32) -> Result<()>;
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

    fn edit_message(
        &mut self,
        uid: i64,
        msg_id: i32,
        msg: &str,
        keyboard: Option<InlineKeyboardMarkup>,
    ) -> Result<()> {
        debug!("TelegramFrontend: edit_message {msg_id}");

        let send_message_params = EditMessageTextParams::builder()
            .chat_id(uid)
            .message_id(msg_id)
            .text(msg);

        let params = if let Some(keyboard) = keyboard {
            send_message_params.reply_markup(keyboard).build()
        } else {
            send_message_params.build()
        };
        self.bot.edit_message_text(&params)?;
        Ok(())
    }

    fn delete_keyboard(&mut self, uid: i64, msg_id: i32) -> Result<()> {
        debug!("TelegramFrontend: delete_keyboard at msg {msg_id}");
        let params = EditMessageReplyMarkupParams::builder()
            .chat_id(uid)
            .message_id(msg_id)
            .build();
        self.bot.edit_message_reply_markup(&params)?;
        Ok(())
    }

    fn delete_message(&mut self, uid: i64, msg_id: i32) -> Result<()> {
        debug!("TelegramFrontend: delete_message {msg_id}");
        let params = DeleteMessageParams::builder()
            .chat_id(uid)
            .message_id(msg_id)
            .build();
        self.bot.delete_message(&params)?;
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
                let msg = &at_calendar_command.message;
                let keyboard = Some(make_calendar_keyboard(
                    at_calendar_command.year,
                    at_calendar_command.month as u32,
                ));
                if let Some(msg_id) = at_calendar_command.msg_id {
                    front.edit_message(uid, msg_id, msg, keyboard)?;
                } else {
                    front.send_message(uid, msg, keyboard)?;
                }
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
            state::FrontendCommand::delete_message(msg_id) => front.delete_message(uid, msg_id)?,
            state::FrontendCommand::delete_keyboard(msg_id) => {
                front.delete_keyboard(uid, msg_id)?
            }
        }
    }
    Ok(())
}
