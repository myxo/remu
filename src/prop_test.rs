#[cfg(test)]
mod tests {
    use anyhow::Result;
    use frankenstein::types::{InlineKeyboardButton, InlineKeyboardMarkup};

    use crate::{
        FrontendHandler,
        database::{self, UserInfo},
        engine, handle_command_to_frontend,
        keyboards::OK_BUTTON_TEXT,
        state::{EXPECT_BUTTON_PUSH, EXPECT_DURATION_MSG, FrontendCommand, is_state_message},
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

        fn last_msg_id(&self) -> i32 {
            self.chat.len() as i32 - 1
        }

        fn get_all_buttons(&self) -> Vec<(InlineKeyboardButton, i32, &str)> {
            let mut buttons = Vec::<(InlineKeyboardButton, i32, &str)>::new();
            for (i, msg) in self.chat.iter().enumerate() {
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
            buttons
        }

        fn get_last_msg_buttons(&self) -> Vec<(InlineKeyboardButton, i32, &str)> {
            let mut buttons = Vec::<(InlineKeyboardButton, i32, &str)>::new();

            for (i, msg) in self.chat.iter().rev().enumerate() {
                if !msg.deleted {
                    if let Some(k) = &msg.keyboard {
                        for row in &k.inline_keyboard {
                            for b in row.iter() {
                                buttons.push((b.clone(), i as i32, &msg.msg));
                            }
                        }
                    }
                    break;
                }
            }
            buttons
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
                keyboard,
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
    #[ignore]
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
            let uid = 69;
            let mut count = 0;
            let mut new_msg = || {
                count += 1;
                format!("msg# {count}")
            };
            let mut now =
                chrono::DateTime::from_timestamp(1_600_000_000, 0).expect("valid timestamp");

            let mut front = MockFront::new();
            let mut engine = engine::Engine::new(database::DbMode::InMemory);
            let user = UserInfo {
                uid,
                name: "name",
                chat_id: uid,
                first_name: "",
                last_name: "",
                tz: -3,
            };
            engine.add_user(user, 0).expect("cannot add user"); // TODO: chaos tz

            let labels = &["user_write_msg", "user_push_button", "tick"];

            src.repeat_n("iter", 0..50, |src| {
                src.select("select", labels, |src, l, _| {
                    match l {
                        "user_write_msg" => {
                            let expected_next = front
                                .chat
                                .last()
                                .map(|last| {
                                    src.log_value("last_msg", &last.msg);
                                    if last.msg.contains(EXPECT_BUTTON_PUSH) {
                                        ExpectedNextAction::PushButton
                                    } else if last.msg.contains(EXPECT_DURATION_MSG) {
                                        ExpectedNextAction::WriteDurationSpec
                                    } else {
                                        ExpectedNextAction::None
                                    }
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
                            front.send_message(uid, &msg, None).expect("always");
                            let cmds =
                                engine.handle_text_message(uid, front.last_msg_id(), &msg, now);

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
                                .expect("unexpected error");
                        }
                        "user_push_button" => {
                            let state_machine_middle_state = front
                                .chat
                                .last()
                                .map(|last| {
                                    if let Some(keyboard) = &last.keyboard {
                                        keyboard
                                            .inline_keyboard
                                            .iter()
                                            .flat_map(|k| k.iter())
                                            .find_map(|k| {
                                                k.callback_data
                                                    .as_ref()
                                                    .map(|f| f == OK_BUTTON_TEXT)
                                            })
                                            .is_none()
                                    } else {
                                        is_state_message(&last.msg)
                                    }
                                })
                                .unwrap_or(false);

                            // if we in a middle of state machine processing, we use only last message keyboard
                            // just because it whould be harder to make prop test
                            // (mainly because it's hard to make a good property with current chat model and
                            // it's not worth the efford)
                            let buttons = if state_machine_middle_state {
                                front.get_last_msg_buttons()
                            } else {
                                front.get_all_buttons()
                            };

                            let button = src.choose("button", &buttons).map(|b| b.0);

                            if let Some(b) = button {
                                let callback =
                                    b.0.callback_data
                                        .as_ref()
                                        .expect(&format!("no callback data in button: {:?}", b.0));
                                let cmds = engine
                                    .handle_keyboard_responce(uid, b.1, callback, b.2, now)
                                    .expect("unexpected error");
                                log_frontend_command(src, &cmds);
                                handle_command_to_frontend(&mut front, uid, cmds)
                                    .expect("unexpected error");
                            }
                        }
                        "tick" => {}
                        "advance_time" => now += std::time::Duration::from_secs(1),
                        _ => panic!("meh"),
                    };
                });
                chaos_theory::Effect::Success
            });
        });
    }
}
