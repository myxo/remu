use chrono::prelude::*;
use std::thread;

use command::*;
use database::DataBase;



pub struct Engine{
    next_wakeup : Option<DateTime<Local>>, 
    data_base : DataBase,
    // callback : fn(),
    stop_loop : bool,
}

impl Engine {
    pub fn run(&mut self){
        self.stop_loop = false;
        self.loop_thread();
        // thread::spawn( move || { self.loop_thread() } );
    }

    // temporal solution for get message from python
    // TODO: delete this after figuring out how to deal with callback from python
    // DO NOT DO RUN WITH THIS FUNCTION!
    pub fn check_for_message(&mut self) -> String{
        self.tick()
    }

    fn loop_thread(&mut self){
        while !self.stop_loop {
            self.tick();
            thread::sleep_ms(1000);
        }
    }

    // TODO: this matches look awful. Rewrite
    fn tick(&mut self) -> String {
        if self.next_wakeup.is_none() {
            self.next_wakeup = self.data_base.get_nearest_wakeup();
            return String::from("");
        }
        let next_wakeup = self.next_wakeup.unwrap();

        if Local::now() > next_wakeup {
            let command = self.data_base.pop(next_wakeup);
            match command {
                None => return String::from(""),
                Some (c) => {
                    match c {
                        Command::BadCommand => return String::from(""),
                        Command::OneTimeEvent(e) => {
                            let command_text = e.event_text;
                            println!("EVENT! Command text: {}", &command_text);
                            // self.next_wakeup = Local::now() + chrono::Duration::seconds(3)
                            self.next_wakeup = self.data_base.get_nearest_wakeup();
                            return command_text;
                        },
                    }
                },
            }
        }
        String::from("")
    }

    pub fn handle_text_message(&mut self, text_message : &str) -> String{
        let com = parse_command(String::from(text_message));
        match com {
            Command::BadCommand => self.process_bad_command(),
            Command::OneTimeEvent(e) => self.process_one_time_event_command(e),
        }
    }

    fn process_bad_command(&self) -> String{
        String::from("Can't parse input string")
    }

    fn process_one_time_event_command(&mut self, c : OneTimeEventImpl) -> String{
        let mut return_string = self.format_return_message_header(&c.event_time);
        return_string.push('\n');
        return_string.push_str(&c.event_text);
        self.data_base.put(c.event_time, Command::OneTimeEvent(c) );
        self.next_wakeup = self.data_base.get_nearest_wakeup();
        return_string
    }

    fn format_return_message_header(&self, event_time : &DateTime<Local>) -> String{
        event_time.format("I'll remind you at %H:%M").to_string()
    }

    pub fn stop(&mut self){
        self.stop_loop = true;
    }

    pub fn new() -> Engine{
        Engine {stop_loop : false
            , next_wakeup : None
            , data_base : DataBase::new()
            }
    }
}
