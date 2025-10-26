#[cfg(test)]
mod tests {
    use anyhow::Result;
    use frankenstein::types::{InlineKeyboardButton, InlineKeyboardMarkup};

    use super::*;

    use crate::{
        FrontendHandler, database, engine, handle_command_to_frontend, state::EXPECT_DURATION_MSG,
    };

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
