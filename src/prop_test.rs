#[cfg(test)]
mod tests {
    use anyhow::Result;
    use frankenstein::types::{InlineKeyboardButton, InlineKeyboardMarkup};

    use crate::{
        FrontendHandler, database, engine, handle_command_to_frontend,
        state::{EXPECT_BUTTON_PUSH, EXPECT_DURATION_MSG, FrontendCommand},
    };

    struct Message {
        msg: String,
        keyboard: Option<InlineKeyboardMarkup>,
        deleted: bool,
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
                deleted: false,
            });
            Ok(())
        }

        fn edit_message(
            &mut self,
            _uid: i64,
            mid: i32,
            msg: &str,
            keyboard: Option<InlineKeyboardMarkup>,
        ) -> Result<()> {
            assert!(!self.chat[mid as usize].deleted);

            self.chat[mid as usize] = Message {
                msg: msg.to_owned(),
                keyboard,
                deleted: false,
            };
            Ok(())
        }

        fn delete_keyboard(&mut self, _uid: i64, msg_id: i32) -> Result<()> {
            assert!(self.chat[msg_id as usize].keyboard.is_some());
            self.chat[msg_id as usize].keyboard = None;
            Ok(())
        }

        fn delete_message(&mut self, _uid: i64, msg_id: i32) -> Result<()> {
            assert!(!self.chat[msg_id as usize].deleted);
            self.chat[msg_id as usize].deleted = true;
            Ok(())
        }
    }

    fn log_frontend_command(src: &chaos_theory::Source, cmds: &Vec<FrontendCommand>) {
        for cmd in cmds {
            src.log_value("front_command", cmd);
        }
    }

    #[test]
    fn property_test() {
        // model of messager (messages + keyboard attached to them)
        // model of expected events

        let make_duration_spec = || -> String {
            "5m".to_owned() // TODO: accept duration
        };

        enum ExpectedNextAction {
            None,
            PushButton,
            WriteDurationSpec,
        }

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

            src.repeat_n("iter", 0..50, |src| {
                src.select("select", labels, |src, l, _| {
                    match l {
                        "user_write_msg" => {
                            let expected_next = front
                                .chat
                                .last()
                                .and_then(|last| {
                                    src.log_value("last_msg", &last.msg);
                                    Some(if last.msg.starts_with(EXPECT_BUTTON_PUSH) {
                                        ExpectedNextAction::PushButton
                                    } else if last.msg == EXPECT_DURATION_MSG {
                                        ExpectedNextAction::WriteDurationSpec
                                    } else {
                                        ExpectedNextAction::None
                                    })
                                })
                                .unwrap_or(ExpectedNextAction::None);

                            let (msg, expect_error) = match expected_next {
                                ExpectedNextAction::None => (new_msg(), false),
                                ExpectedNextAction::PushButton => {
                                    if src.any("error_instead_of_button_push") {
                                        ("text instead of button".to_owned(), true)
                                    } else {
                                        return;
                                    }
                                }
                                ExpectedNextAction::WriteDurationSpec => {
                                    if src.any("error_instead_of_duration") {
                                        ("non spec string for test".to_owned(), true)
                                    } else {
                                        (make_duration_spec(), false)
                                    }
                                }
                            };

                            src.log_value("msg", &msg);
                            let cmds = engine.handle_text_message(uid, &msg);

                            let cmds = match cmds {
                                Ok(cmds) => {
                                    if expect_error {
                                        panic!("expect error, but get commands {:?}", cmds);
                                    }
                                    cmds
                                }
                                Err(e) => {
                                    if !expect_error {
                                        panic!("unexpected error: {:#}", e);
                                    }
                                    return;
                                }
                            };

                            log_frontend_command(src, &cmds);
                            handle_command_to_frontend(&mut front, uid, cmds)
                                .expect("no error in test");
                        }
                        "user_push_button" => {
                            let mut buttons = Vec::<(InlineKeyboardButton, i32, &str)>::new();
                            for (i, msg) in front.chat.iter().enumerate() {
                                if let Some(k) = &msg.keyboard
                                    && !msg.deleted
                                {
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
                                    log_frontend_command(src, &cmds);
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
                chaos_theory::Effect::Success
            });
        });
    }
}
