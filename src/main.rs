extern crate chrono;
extern crate regex;


mod command;
pub mod engine;
mod database;
use command::Command;
use engine::Engine;

fn main() {
    // let st : time::SteadyTime = time::SteadyTime::now();
    Command::parse_command(String::from("5s test"));
    Command::parse_command(String::from("5d test"));
    Command::parse_command(String::from("3m5s test"));

    let mut engine = Engine::new_engine();
    engine.handle_text_message("5s test");
    engine.run();
}
