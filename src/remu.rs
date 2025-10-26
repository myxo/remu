use anyhow::{Context, Result};
use log::{debug, info, warn};
use std::sync::Arc;
use teloxide::{
    Bot,
    dispatching::{HandlerExt, UpdateFilterExt},
    dptree,
    payloads::SendMessageSetters,
    prelude::{Dispatcher, Requester, ResponseResult},
    types::{
        CallbackQuery, InlineKeyboardButton, InlineKeyboardMarkup, Message, Recipient, Update,
    },
    utils::command::BotCommands,
};
use tokio::sync::Mutex;

use crate::state::FrontendCommand;

mod command;
pub mod database;
pub mod engine;
mod helpers;
mod sql_query;
pub mod state;
pub mod time;

#[derive(BotCommands, Clone)]
#[command(
    rename_rule = "lowercase",
    description = "These commands are supported:"
)]
enum Command {
    Help,
    Start,
}

async fn answer_command(
    //engine: &mut engine::Engine,
    provider: Arc<Mutex<engine::Engine>>,
    bot: Bot,
    msg: Message,
    cmd: Command,
) -> ResponseResult<()> {
    let mut engine = provider.lock().await;
    match cmd {
        Command::Help => {
            bot.send_message(msg.chat.id, Command::descriptions().to_string())
                .await?;
        }
        Command::Start => {
            let user = msg.from.expect("TODO");
            engine.add_user(
                user.id.0 as i64,
                user.username.as_ref().unwrap(),
                msg.chat.id.0,
                &user.first_name,
                &user.last_name.as_ref().unwrap(),
                -3,
            );
            // bot.send_message(msg.chat.id, "well, hello...").await?;
        }
    };
    Ok(())
}

async fn answer(
    provider: Arc<Mutex<engine::Engine>>,
    bot: Bot,
    msg: Message,
) -> ResponseResult<()> {
    let user = msg.from.as_ref().expect("message has user");
    debug!("In a text handler, user: {}", user.first_name);
    let mut engine = provider.lock().await;
    let cmds = engine.handle_text_message(user.id.0 as i64, &msg.text().unwrap());
    match cmds {
        Ok(cmds) => {
            let mut front = TelegramFrontend { bot: bot };
            if let Err(e) = handle_command_to_frontend(&mut front, user.id.0 as i64, cmds) {
                warn!("cannot handle frontend command: {e}");
            }
        }
        Err(e) => {
            bot.send_message(
                user.id,
                format!("Error while state machine processing:\n\n{e:#}"),
            )
            .await?;
        }
    };
    Ok(())
}

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
        msg.id().0 as i64,
        &q.data.unwrap(),
        &msg.regular_message().unwrap().text().unwrap(),
    );
    let mut front = TelegramFrontend { bot: bot };
    if let Err(e) = handle_command_to_frontend(&mut front, user.id.0 as i64, cmds) {
        warn!("cannot handle frontend command: {e}");
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    info!("start");

    let clock = Box::new(crate::time::OsClock {});
    let provider = Arc::new(Mutex::new(engine::Engine::new(
        database::DbMode::Filesystem,
        clock,
    )));
    let api_key = std::fs::read_to_string("token.id")?;
    let bot = Bot::new(api_key);
    let mut front = TelegramFrontend { bot: bot.clone() };
    let provider_copy = provider.clone();

    tokio::spawn(async move {
        loop {
            let mut engine = provider_copy.lock().await;
            if engine.is_stop() {
                break;
            }
            let events = engine.tick();
            if !events.is_empty() {
                warn!("TMP: tick events: {events:?}");
            }
            for ev in events {
                if let Err(e) = handle_command_to_frontend(&mut front, ev.uid, ev.cmd_vec) {
                    warn!("cannot handle frontend command: {e}");
                }
            }

            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }
    });

    let handler = dptree::entry()
        .branch(
            Update::filter_message()
                .filter_command::<Command>()
                .endpoint(answer_command),
        )
        .branch(Update::filter_message().endpoint(answer))
        .branch(Update::filter_callback_query().endpoint(keyboard));

    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![provider])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
    Ok(())
}

trait FrontendHandler {
    fn send_message(
        &mut self,
        uid: i64,
        msg: &str,
        keyboard: Option<InlineKeyboardMarkup>,
    ) -> Result<()>;
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
        futures::executor::block_on(async {
            let send = self
                .bot
                .send_message(Recipient::Id(teloxide::types::ChatId(uid)), msg);
            let send = if let Some(keyboard) = keyboard {
                send.reply_markup(keyboard)
            } else {
                send
            };
            send.await.context("cannot send telegram message")?;
            Ok(())
        })
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
            state::FrontendCommand::delete_keyboard {} => todo!(),
        }
    }
    Ok(())
}

fn make_main_keyboard() -> InlineKeyboardMarkup {
    let mut keyboard: Vec<Vec<InlineKeyboardButton>> = vec![];

    keyboard.push(vec![
        InlineKeyboardButton::callback("at".to_owned(), "at".to_owned()),
        InlineKeyboardButton::callback("after".to_owned(), "after".to_owned()),
    ]);

    keyboard.push(vec![
        InlineKeyboardButton::callback("5m".to_owned(), "5m".to_owned()),
        InlineKeyboardButton::callback("30m".to_owned(), "30m".to_owned()),
        InlineKeyboardButton::callback("1h".to_owned(), "1h".to_owned()),
    ]);

    keyboard.push(vec![
        InlineKeyboardButton::callback("3h".to_owned(), "3h".to_owned()),
        InlineKeyboardButton::callback("1d".to_owned(), "1d".to_owned()),
        InlineKeyboardButton::callback("Ok".to_owned(), "Ok".to_owned()),
    ]);

    InlineKeyboardMarkup::new(keyboard)
}

#[cfg(test)]
mod tests {
    use teloxide::types::InlineKeyboardButtonKind;

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
                            let mut buttons = Vec::<(InlineKeyboardButton, i64, &str)>::new();
                            for (i, msg) in front.chat.iter().enumerate() {
                                if let Some(k) = &msg.keyboard {
                                    for row in &k.inline_keyboard {
                                        for b in row.iter() {
                                            buttons.push((b.clone(), i as i64, &msg.msg));
                                        }
                                    }
                                }
                            }

                            if let Some((b, _)) = src.choose("button", &buttons) {
                                if let InlineKeyboardButtonKind::CallbackData(callback) = &b.0.kind
                                {
                                    let cmds =
                                        engine.handle_keyboard_responce(uid, b.1, callback, b.2);
                                    handle_command_to_frontend(&mut front, uid, cmds)
                                        .expect("no error in test");
                                } else {
                                    panic!("unexpected inline button kind: {:?}", b.0.kind);
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
