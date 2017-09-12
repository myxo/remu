extern crate chrono;

use chrono::prelude::*;
use std::thread;

use command::Command;
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

    fn loop_thread(&mut self){
        let mut tick_index = 0;

        while !self.stop_loop {
            tick_index += 1;
            self.tick();
            thread::sleep_ms(1000);
            println!("{} ", tick_index);
        }
    }

    fn tick(&mut self){
        if self.next_wakeup.is_none() {
            self.next_wakeup = self.data_base.get_nearest_wakeup();
            return;
        }
        let next_wakeup = self.next_wakeup.unwrap();

        if Local::now() > next_wakeup {
            let command = self.data_base.pop(next_wakeup);
            println!("command text: {}", command.unwrap().event_text);
            // self.next_wakeup = Local::now() + chrono::Duration::seconds(3)
            self.next_wakeup = self.data_base.get_nearest_wakeup();
        }
    }

    pub fn handle_text_message(&mut self, text_message : &str){
        let command = Command::parse_command(String::from(text_message));
        println!("{:?}", command);
        self.data_base.put(command.event_time, command);
    }

    fn format_return_message_header(&self){
        
    }

    pub fn stop(&mut self){
        self.stop_loop = true;
    }

    pub fn new_engine() -> Engine{
        Engine {stop_loop : false
            , next_wakeup : None
            , data_base : DataBase::new()
            }
    }
}
