use std::sync::Arc;
use teloxide::{prelude::*, utils::command::BotCommands};
use tokio::sync::Mutex;

#[macro_use]
extern crate log;

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
    engine: &mut engine::Engine,
    bot: Bot,
    msg: Message,
    cmd: Command,
) -> ResponseResult<()> {
    let user = msg.from().expect("message has user");
    match cmd {
        Command::Help => {
            bot.send_message(msg.chat.id, Command::descriptions().to_string())
                .await?;
        }
        Command::Start => {
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

async fn answer(engine: &mut engine::Engine, bot: Bot, msg: Message) -> ResponseResult<()> {
    let user = msg.from().expect("message has user");
    info!("In a text handler");
    let cmds = engine.handle_text_message(user.id.0 as i64, &msg.text().unwrap());
    Ok(())
}
async fn keyboard(engine: &mut engine::Engine, bot: Bot, q: CallbackQuery) -> ResponseResult<()> {
    let msg = q.message.unwrap();
    let user = msg.from().expect("message has user");
    engine.handle_keyboard_responce(user.id.0 as i64, msg.id.0 as i64, "", &msg.text().unwrap());
    Ok(())
}

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    let clock = Box::new(crate::time::OsClock {});
    let provider = Arc::new(Mutex::new(engine::Engine::new(
        database::DbMode::Filesystem,
        clock,
    )));

    /*
    let handler = Update::filter_message().filter_command::<Command>().endpoint(
        |bot: Bot, provider: Arc<Mutex<engine::Engine>>, msg: Message, cmd: Command| async move {
            let mut prov = provider.lock().await;
            answer(&mut prov, bot, msg, cmd).await?;
            respond(())
        },
    );
    */

    let handler =
        dptree::entry()
            .branch(Update::filter_message().endpoint(
                |bot: Bot, provider: Arc<Mutex<engine::Engine>>, msg: Message| async move {
                    let mut prov = provider.lock().await;
                    answer(&mut prov, bot, msg).await?;
                    respond(())
                },
            ))
            .branch(
                Update::filter_message()
                    .filter_command::<Command>()
                    .endpoint(
                        |bot: Bot,
                         provider: Arc<Mutex<engine::Engine>>,
                         msg: Message,
                         cmd: Command| async move {
                            let mut prov = provider.lock().await;
                            answer_command(&mut prov, bot, msg, cmd).await?;
                            respond(())
                        },
                    ),
            )
            .branch(Update::filter_callback_query().endpoint(
                |bot: Bot, provider: Arc<Mutex<engine::Engine>>, q: CallbackQuery| async move {
                    let mut prov = provider.lock().await;
                    keyboard(&mut prov, bot, q).await?;
                    respond(())
                },
            ));

    let bot = Bot::from_env();
    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![provider])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
}
