use remu_backend::database::DbMode;
use remu_backend::engine::{CmdToEngine, CmdFromEngine, engine_run};
use remu_backend::time::mock_time::{set_mock_time, now};
use std::sync::mpsc;

#[derive(Copy, Clone)]
enum Cmd {
    Send(&'static str),
    Expect(&'static str),
    TimeAdvance(i64),
}

pub struct TestCase {
    cmd_list : Vec<Cmd>,
    tx_to_engine: mpsc::Sender<CmdToEngine>,
    rx_out_engine: mpsc::Receiver<CmdFromEngine>,
}


impl TestCase {
    pub fn create() -> TestCase {
        let (tx_to_engine, rx_out_engine) = engine_run(DbMode::InMemory);
        let mut res = TestCase { 
            cmd_list : Vec::new(), 
            tx_to_engine,
            rx_out_engine,
        };
        res.add(Cmd::Send(r#"{"AddUser":{"uid":1,"username":"test_username","chat_id":123,"first_name":"First","last_name":"Last","tz":-3}}"#));
        res
    }

    pub fn send(&mut self, json: &'static str) {
        self.add(Cmd::Send(json));
    }

    pub fn expect(&mut self, json: &'static str) {
        self.add(Cmd::Expect(json));
    }

    pub fn advance_time(&mut self, seconds : i64) {
        self.add(Cmd::TimeAdvance(seconds));
    }

    pub fn run(&self) {
        for cmd in &self.cmd_list {
            match cmd {
                Cmd::Send(json) => self.handle_send(serde_json::from_str(json).unwrap()),
                Cmd::Expect(json) => self.handle_expect(serde_json::from_str(json).unwrap()),
                Cmd::TimeAdvance(seconds) => self.handle_advance_time(*seconds),
            }
        }
    }
    
    fn add(&mut self, cmd: Cmd) {
        self.cmd_list.push( cmd );
    }

    fn handle_send(&self, cmd: CmdToEngine) {
        assert!(self.tx_to_engine.send(cmd).is_ok());
    }

    fn handle_expect(&self, expect: CmdFromEngine) {
        let response = self.rx_out_engine.recv().expect("Response from engine is Error");
        assert_eq!(response, expect);
    }

    fn handle_advance_time(&self, seconds: i64) {
        set_mock_time(Some(now() + chrono::Duration::seconds(seconds)));
    }
}

impl Drop for TestCase {
    fn drop(&mut self) {
        self.tx_to_engine.send(CmdToEngine::Terminate).expect("Terminate cmd failed");
    }
}