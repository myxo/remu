use anyhow::{Context, Result};
use frankenstein::{
    TelegramApi,
    client_ureq::Bot,
    methods::{EditMessageReplyMarkupParams, GetUpdatesParams, SendMessageParams},
    types::{InlineKeyboardButton, InlineKeyboardMarkup, ReplyMarkup},
    updates::UpdateContent,
};
use log::{debug, info, warn};

use crate::state::FrontendCommand;

mod command;
pub mod database;
pub mod engine;
mod helpers;
mod sql_query;
pub mod state;
pub mod time;

/*
async fn keyboard(
    provider: Arc<Mutex<engine::Engine>>,
    bot: Bot,
    q: CallbackQuery,
) -> ResponseResult<()> {
    let msg = q.message.unwrap();
    let user = q.from;
    let mut engine = provider.lock().await;
    let cmds = engine.handle_keyboard_responce(
        user.id.0 as i64,
        msg.id().0,
        &q.data.unwrap(),
        &msg.regular_message().unwrap().text().unwrap(),
    );
    let mut front = TelegramFrontend { bot: bot };
    if let Err(e) = handle_command_to_frontend(&mut front, user.id.0 as i64, cmds) {
        warn!("cannot handle frontend command: {e}");
    }
    Ok(())
}
*/

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
            state::FrontendCommand::calendar(_at_calendar_command) => todo!(),
            state::FrontendCommand::keyboard(keyboard_command) => {
                match keyboard_command.action_type {
                    state::KeyboardCommandType::Main => {
                        front.send_message(
                            uid,
                            &keyboard_command.text,
                            Some(make_main_keyboard()),
                        )?;
                    }
                    state::KeyboardCommandType::Hour => todo!(),
                    state::KeyboardCommandType::Minute => todo!(),
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

fn make_main_keyboard() -> InlineKeyboardMarkup {
    let mut keyboard: Vec<Vec<InlineKeyboardButton>> = vec![];

    keyboard.push(vec![
        InlineKeyboardButton::builder()
            .text("at")
            .callback_data("at")
            .build(),
        InlineKeyboardButton::builder()
            .text("after")
            .callback_data("after")
            .build(),
    ]);

    keyboard.push(vec![
        InlineKeyboardButton::builder()
            .text("5m")
            .callback_data("5m")
            .build(),
        InlineKeyboardButton::builder()
            .text("30m")
            .callback_data("30m")
            .build(),
        InlineKeyboardButton::builder()
            .text("1h")
            .callback_data("1h")
            .build(),
    ]);

    keyboard.push(vec![
        InlineKeyboardButton::builder()
            .text("3h")
            .callback_data("3h")
            .build(),
        InlineKeyboardButton::builder()
            .text("1d")
            .callback_data("1d")
            .build(),
        InlineKeyboardButton::builder()
            .text("Ok")
            .callback_data("Ok")
            .build(),
    ]);

    InlineKeyboardMarkup {
        inline_keyboard: keyboard,
    }
}

#[cfg(test)]
mod tests {

    use crate::state::EXPECT_DURATION_MSG;

    use super::*;

    struct Message {
        msg: String,
        keyboard: Option<InlineKeyboardMarkup>,
    }

    struct MockFront {
        chat: Vec<Message>,
    }

    impl MockFront {
        fn new() -> Self {
            Self { chat: vec![] }
        }
    }

    impl FrontendHandler for MockFront {
        fn send_message(
            &mut self,
            _uid: i64,
            msg: &str,
            keyboard: Option<InlineKeyboardMarkup>,
        ) -> Result<()> {
            self.chat.push(Message {
                msg: msg.to_owned(),
                keyboard: keyboard,
            });
            Ok(())
        }

        fn delete_keyboard(&mut self, _uid: i64, msg_id: i32) -> Result<()> {
            self.chat[msg_id as usize].keyboard = None;
            // TODO: error if no keyboard?
            Ok(())
        }
    }

    #[test]
    fn property_test() {
        // model of messager (messages + keyboard attached to them)
        // model of expected events

        let make_duration_spec = || -> String {
            "5m".to_owned() // TODO: accept duration
        };
        chaos_theory::check(|src| {
            if src.should_log() {
                env_logger::Builder::from_env(
                    env_logger::Env::default().default_filter_or("debug"),
                )
                .init();
            }
            let clock = Box::new(crate::time::MockClock::new(chrono::Utc::now()));
            let uid = 69;
            let mut count = 0;
            let mut new_msg = || {
                count += 1;
                format!("msg# {count}")
            };

            let mut front = MockFront::new();
            let mut engine = engine::Engine::new(database::DbMode::InMemory, clock);
            engine.add_user(uid, "name", uid, "", "", -3); // TODO: chaos tz

            let labels = &["user_write_msg", "user_push_button", "tick"];

            let n = src.any_of("iter", chaos_theory::make::int_in_range(0..50));
            for _ in 0..n {
                src.select("select", labels, |src, l, _| {
                    match l {
                        "user_write_msg" => {
                            let need_answer_duration = front
                                .chat
                                .last()
                                .and_then(|last| Some(last.msg == EXPECT_DURATION_MSG))
                                .unwrap_or(false);

                            let msg = if need_answer_duration {
                                if src.any("error_instead_of_duration") {
                                    "non spec string".to_owned()
                                } else {
                                    make_duration_spec()
                                }
                            } else {
                                if src.any("spec message") {
                                    make_duration_spec() + " " + &new_msg()
                                } else {
                                    new_msg()
                                }
                            };

                            src.log_value("msg", &msg);
                            let cmds = engine
                                .handle_text_message(uid, &msg)
                                .expect("no error in test");
                            handle_command_to_frontend(&mut front, uid, cmds)
                                .expect("no error in test");
                        }
                        "user_push_button" => {
                            let mut buttons = Vec::<(InlineKeyboardButton, i32, &str)>::new();
                            for (i, msg) in front.chat.iter().enumerate() {
                                if let Some(k) = &msg.keyboard {
                                    for row in &k.inline_keyboard {
                                        for b in row.iter() {
                                            buttons.push((b.clone(), i as i32, &msg.msg));
                                        }
                                    }
                                }
                            }

                            if let Some((b, _)) = src.choose("button", &buttons) {
                                if let Some(callback) = &b.0.callback_data {
                                    let cmds = engine
                                        .handle_keyboard_responce(uid, b.1, callback, b.2)
                                        .expect("no error in test");
                                    handle_command_to_frontend(&mut front, uid, cmds)
                                        .expect("no error in test");
                                } else {
                                    panic!("no callback data in button: {:?}", b.0);
                                }
                            }
                        }
                        "tick" => {}
                        _ => panic!("meh"),
                    };
                });
            }
        });
    }
}
