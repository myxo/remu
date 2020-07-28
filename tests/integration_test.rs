#![allow(non_upper_case_globals)]
extern crate remu_backend;
extern crate chrono;

use remu_backend::engine::{CmdToEngine, CmdFromEngine, engine_run};
use remu_backend::database::DbMode;
// use std::time;
use chrono::prelude::*;
use std::sync::mpsc;

// fn time_moment_eq(t1: DateTime<FixedOffset>, t2: DateTime<FixedOffset>) -> bool{
//     t1.signed_duration_since(t2).num_milliseconds().abs() < 1500
// }

// fn str_to_event(mut s: String, tz: i32) -> (String, chrono::DateTime<FixedOffset>){
//     let offset = s.rfind(":").unwrap_or_else( || { panic!("Wrong str format - {}", s); } );
//     let text = s.split_off(offset-1)[3..].to_string();
//     let tz = FixedOffset::west(tz * 60 * 60);
//     let t = tz.datetime_from_str(&s[..], "%e %b %l.%M").unwrap_or_else( |err| { panic!("Cant format str to datetime: {} , {}", s, err) });
//     (text, t)
// }

enum CmdType {
    Send,
    Expect
}

struct Cmd {
    json : &'static str,
    cmd_type : CmdType,
}

struct TestCase {
    cmd_list : Vec<Cmd>,
    tx_to_engine: mpsc::Sender<CmdToEngine>,
    rx_out_engine: mpsc::Receiver<CmdFromEngine>,
}

impl TestCase {
    fn create() -> TestCase {
        let (tx_to_engine, rx_out_engine) = engine_run(DbMode::InMemory);
        let mut res = TestCase { 
            cmd_list : Vec::new(), 
            tx_to_engine,
            rx_out_engine,
        };
        res.add(CmdType::Send, r#"{"AddUser":{"uid":1,"username":"test_username","chat_id":123,"first_name":"First","last_name":"Last","tz":-3}}"#);
        res
    }

    fn send(&mut self, json: &'static str) {
        self.add(CmdType::Send, json);
    }

    fn expect(&mut self, json: &'static str) {
        self.add(CmdType::Expect, json);
    }

    fn add(&mut self, cmd_type: CmdType, json: &'static str) {
        self.cmd_list.push( Cmd{ json, cmd_type, } );
    }

    fn run(&self) {
        for cmd in &self.cmd_list {
            match cmd.cmd_type {
                CmdType::Send => self.handle_send(serde_json::from_str(cmd.json).unwrap()),
                CmdType::Expect => self.handle_expect(serde_json::from_str(cmd.json).unwrap()),
            }
        }
    }

    fn handle_send(&self, cmd: CmdToEngine) {
        assert!(self.tx_to_engine.send(cmd).is_ok());
    }

    fn handle_expect(&self, expect: CmdFromEngine) {
        let response = self.rx_out_engine.recv().expect("Response from engine is Error");
        assert_eq!(response.uid, expect.uid);
        assert_eq!(response.to_msg, expect.to_msg);
        assert_eq!(response.cmd_vec.len(), expect.cmd_vec.len());
        // assert!(tx_to_engine.send(cmd).is_ok());
    }
}

impl Drop for TestCase {
    fn drop(&mut self) {
        self.tx_to_engine.send(CmdToEngine::Terminate).expect("Terminate cmd failed");
    }
}

#[test]
fn simple_message() {
    let mut case = TestCase::create();
    case.send(r#"{"TextMessage":{"uid":1,"msg_id":2774,"message":"1s test"}}"#);
    case.expect(r#"{"uid":1,"to_msg":2774,"cmd_vec":[
        {"send":{"text":"I'll remind you today at 04:57\ntest"}}
    ]}"#);

    case.run();    
}
