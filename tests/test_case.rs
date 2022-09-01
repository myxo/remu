use remu_backend::database::DbMode;
use remu_backend::engine::{engine_run, CmdFromEngine, CmdToEngine};
use std::sync::mpsc;

#[derive(Copy, Clone)]
enum Cmd {
    Send(&'static str),
    Expect(&'static str),
    TimeAdvance(i64),
    SkipExpect,
}

pub struct TestCase {
    cmd_list: Vec<Cmd>,
    tx_to_engine: mpsc::Sender<CmdToEngine>,
    rx_out_engine: mpsc::Receiver<CmdFromEngine>,
}

impl TestCase {
    pub fn create() -> TestCase {
        let (tx_to_engine, rx_out_engine) = engine_run(DbMode::InMemory);
        let mut res = TestCase {
            cmd_list: Vec::new(),
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

    pub fn advance_time(&mut self, seconds: i64) {
        self.add(Cmd::TimeAdvance(seconds));
    }

    pub fn skip_one(&mut self) {
        self.add(Cmd::SkipExpect);
    }

    pub fn run(&self) {
        for cmd in &self.cmd_list {
            match cmd {
                Cmd::Send(json) => {
                    self.handle_send(serde_json::from_str(json).expect("wrong json"))
                }
                Cmd::Expect(json) => {
                    self.handle_expect(serde_json::from_str(json).expect("wrong json"))
                }
                Cmd::TimeAdvance(seconds) => self.handle_advance_time(*seconds),
                Cmd::SkipExpect => self.handle_skip(),
            }
        }
    }

    fn add(&mut self, cmd: Cmd) {
        self.cmd_list.push(cmd);
    }

    fn handle_send(&self, cmd: CmdToEngine) {
        assert!(self.tx_to_engine.send(cmd).is_ok());
    }

    fn handle_expect(&self, expect: CmdFromEngine) {
        let response = self
            .rx_out_engine
            .recv()
            .expect("Response from engine is Error");
        assert_eq!(response, expect);
    }

    fn handle_advance_time(&self, seconds: i64) {
        let cmd_json = format!("{{\"AdvanceTime\": {}}}", seconds);
        assert!(self
            .tx_to_engine
            .send(serde_json::from_str(&cmd_json).expect("wrong json"))
            .is_ok());
    }

    fn handle_skip(&self) {
        self.rx_out_engine.recv().expect("recv return error");
    }
}

impl Drop for TestCase {
    fn drop(&mut self) {
        self.tx_to_engine
            .send(CmdToEngine::Terminate)
            .expect("Terminate cmd failed");
    }
}
