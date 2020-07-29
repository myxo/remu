#![allow(non_upper_case_globals)]
extern crate remu_backend;
extern crate chrono;

use remu_backend::time::mock_time::set_mock_time;
use chrono::prelude::*;

mod test_case;
use crate::test_case::TestCase;

#[test]
fn simple_message() {
    set_mock_time(Some(Utc.timestamp(61, 0)));

    let mut case = TestCase::create();
    case.send(r#"{"TextMessage":{"uid":1,"msg_id":2774,"message":"1s test"}}"#);
    case.expect(r#"{"uid":1,"to_msg":2774,"cmd_vec":[
        {"send":{"text":"I'll remind you today at 03:01\ntest"}}
    ]}"#);
    case.advance_time(1);
    case.expect(r#"{"uid":1,"to_msg":null,"cmd_vec":[{"keyboard":{"action_type":"main","text":"test"}}]}"#);

    case.run();    
}
