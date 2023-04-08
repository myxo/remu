#![allow(non_upper_case_globals)]
extern crate chrono;
extern crate remu_backend;

mod test_case;
use crate::test_case::TestCase;

#[test]
fn simple_message() {
    let mut case = TestCase::create();
    case.send(r#"{"TextMessage":{"uid":1,"msg_id":2774,"message":"1s test"}}"#);
    case.expect(
        r#"{"uid":1,"to_msg":2774,"cmd_vec":[
        {"send":{"text":"I'll remind you today at 03:01\ntest"}}
    ]}"#,
    );
    case.advance_time(1);
    case.expect(
        r#"{"uid":1,"to_msg":null,"cmd_vec":[{"keyboard":{"action_type":"main","text":"test"}}]}"#,
    );
    case.send(r#"{"KeyboardMessage":{"uid":1,"msg_id":2782,"call_data":"Ok","msg_text":"test"}}"#);
    case.expect(r#"{"uid":1,"to_msg":2782,"cmd_vec":[{"delete_keyboard":{}}]}"#);

    case.run();
}

#[test]
fn simple_message_day_command() {
    let mut case = TestCase::create();
    case.send(r#"{"TextMessage":{"uid":1,"msg_id":2774,"message":"1d1h1s test"}}"#);
    case.expect(
        r#"{"uid":1,"to_msg":2774,"cmd_vec":[
        {"send":{"text":"I'll remind you tomorrow at 04:01\ntest"}}
    ]}"#,
    );
    case.advance_time(25 * 60 * 60 + 1);
    case.expect(
        r#"{"uid":1,"to_msg":null,"cmd_vec":[{"keyboard":{"action_type":"main","text":"test"}}]}"#,
    );
    case.send(r#"{"KeyboardMessage":{"uid":1,"msg_id":2782,"call_data":"Ok","msg_text":"test"}}"#);
    case.expect(r#"{"uid":1,"to_msg":2782,"cmd_vec":[{"delete_keyboard":{}}]}"#);

    case.run();
}

#[test]
fn simple_message_and_after_button() {
    let mut case = TestCase::create();
    case.send(r#"{"TextMessage":{"uid":1,"msg_id":2774,"message":"1s test"}}"#);
    case.expect(
        r#"{"uid":1,"to_msg":2774,"cmd_vec":[
        {"send":{"text":"I'll remind you today at 03:01\ntest"}}
    ]}"#,
    );
    case.advance_time(1);
    case.expect(
        r#"{"uid":1,"to_msg":null,"cmd_vec":[{"keyboard":{"action_type":"main","text":"test"}}]}"#,
    );
    case.send(
        r#"{"KeyboardMessage":{"uid":1,"msg_id":2785,"call_data":"after","msg_text":"test"}}"#,
    );
    case.expect(
        r#"{"uid":1,"to_msg":2785,"cmd_vec":[{"send":{"text":"Ok, now write time duration."}}]}"#,
    );
    case.send(r#"{"TextMessage":{"uid":1,"msg_id":2787,"message":"1m"}}"#);

    // TODO: delete current keyboard

    case.expect(r#"{"uid":1,"to_msg":2787,"cmd_vec":[{"send":{"text":"Resulting command:\n1m test\nI'll remind you today at 03:02\ntest"}}]}"#);
    case.advance_time(60);
    case.expect(
        r#"{"uid":1,"to_msg":null,"cmd_vec":[{"keyboard":{"action_type":"main","text":"test"}}]}"#,
    );
    case.send(r#"{"KeyboardMessage":{"uid":1,"msg_id":2782,"call_data":"Ok","msg_text":"test"}}"#);
    case.expect(r#"{"uid":1,"to_msg":2782,"cmd_vec":[{"delete_keyboard":{}}]}"#);

    case.run();
}

#[test]
fn simple_message_and_after_5m_button() {
    let mut case = TestCase::create();
    case.send(r#"{"TextMessage":{"uid":1,"msg_id":2774,"message":"1s test"}}"#);
    case.expect(
        r#"{"uid":1,"to_msg":2774,"cmd_vec":[{"send":{"text":"I'll remind you today at 03:01\ntest"}}]}"#,
    );
    case.advance_time(1);
    case.expect(
        r#"{"uid":1,"to_msg":null,"cmd_vec":[{"keyboard":{"action_type":"main","text":"test"}}]}"#,
    );
    case.send(r#"{"KeyboardMessage":{"uid":1,"msg_id":2792,"call_data":"5m","msg_text":"test"}}"#);
    case.expect(r#"{"uid":1,"to_msg":2792,"cmd_vec":[{"send":{"text":"Resulting command:\n5m test\nI'll remind you today at 03:06\ntest"}}]}"#);
    case.advance_time(5 * 60);
    case.expect(
        r#"{"uid":1,"to_msg":null,"cmd_vec":[{"keyboard":{"action_type":"main","text":"test"}}]}"#,
    );
    case.send(r#"{"KeyboardMessage":{"uid":1,"msg_id":2794,"call_data":"Ok","msg_text":"test"}}"#);
    case.expect(r#"{"uid":1,"to_msg":2794,"cmd_vec":[{"delete_keyboard":{}}]}"#);

    case.run();
}

#[test]
fn simple_message_and_at_button() {
    let mut case = TestCase::create();
    case.send(r#"{"TextMessage":{"uid":1,"msg_id":2774,"message":"1s test"}}"#);
    case.expect(
        r#"{"uid":1,"to_msg":2774,"cmd_vec":[
        {"send":{"text":"I'll remind you today at 03:01\ntest"}}
    ]}"#,
    );
    case.advance_time(1);
    case.expect(
        r#"{"uid":1,"to_msg":null,"cmd_vec":[{"keyboard":{"action_type":"main","text":"test"}}]}"#,
    );
    case.send(r#"{"KeyboardMessage":{"uid":1,"msg_id":2796,"call_data":"at","msg_text":"test"}}"#);
    case.expect(r#"{"uid":1,"to_msg":2796,"cmd_vec":[{"calendar":{"action_type":"calendar","year":1970,"month":1,"tz":-3,"edit_cur_msg":true}}]}"#);

    case.send(r#"{"KeyboardMessage":{"uid":1,"msg_id":2796,"call_data":"calendar-day-2","msg_text":"Please, choose a date"}}"#);

    case.expect(r#"{"uid":1,"to_msg":2796,"cmd_vec":[{"delete_message":2796},{"keyboard":{"action_type":"hour","text":"Ok, now write the time of event"}}]}"#);
    case.send(r#"{"KeyboardMessage":{"uid":1,"msg_id":2797,"call_data":"time_hour:10","msg_text":"Please, choose hour"}}"#);
    case.expect(r#"{"uid":1,"to_msg":2797,"cmd_vec":[{"delete_message":2797},{"keyboard":{"action_type":"minute","text":"Ok, 10. Now choose minute"}}]}"#);
    case.send(r#"{"KeyboardMessage":{"uid":1,"msg_id":2798,"call_data":"time_minute:00","msg_text":"Ok, 10. Now choose minute"}}"#);
    case.expect(r#"{"uid":1,"to_msg":2798,"cmd_vec":[{"send":{"text":"I'll remind you tomorrow at 10:00\ntest"}}]}"#);

    case.advance_time(35 * 60 * 60);
    case.expect(
        r#"{"uid":1,"to_msg":null,"cmd_vec":[{"keyboard":{"action_type":"main","text":"test"}}]}"#,
    );
    case.send(r#"{"KeyboardMessage":{"uid":1,"msg_id":2894,"call_data":"Ok","msg_text":"test"}}"#);
    case.expect(r#"{"uid":1,"to_msg":2894,"cmd_vec":[{"delete_keyboard":{}}]}"#);

    case.run();
}

#[test]
fn active_event_list() {
    let mut case = TestCase::create();
    case.send(r#"{"TextMessage":{"uid":1,"msg_id":2774,"message":"5s test"}}"#);
    case.expect(
        r#"{"uid":1,"to_msg":2774,"cmd_vec":[
        {"send":{"text":"I'll remind you today at 03:01\ntest"}}
    ]}"#,
    );
    case.advance_time(1);

    case.send(r#"{"TextMessage":{"uid":1,"msg_id":2776,"message":"5s test2"}}"#);
    case.expect(
        r#"{"uid":1,"to_msg":2776,"cmd_vec":[{"send":{"text":"I'll remind you today at 03:01\ntest2"}}
    ]}"#,
    );
    case.advance_time(1);

    case.send(r#"{"TextMessage":{"uid":1,"msg_id":2800,"message":"/list"}}"#);
    case.expect(
        r#"{"uid":1,"to_msg":2800,"cmd_vec":[
        {"send":{"text":"1) test : _ 1 Jan  3.01_\n2) test2 : _ 1 Jan  3.01_\n"}}
    ]}"#,
    );

    case.run();
}

#[test]
fn at_some_day() {
    let mut case = TestCase::create();

    case.send(r#"{"TextMessage":{"uid":1,"msg_id":2802,"message":"/at"}}"#);
    case.expect(r#"{"uid":1,"to_msg":2802,"cmd_vec":[{"calendar":{"action_type":"calendar","year":1970,"month":1,"tz":-3,"edit_cur_msg":false}}]}"#);

    case.send(r#"{"KeyboardMessage":{"uid":1,"msg_id":2803,"call_data":"calendar-day-7","msg_text":"Please, choose a date"}}"#);
    case.expect(r#"{"uid":1,"to_msg":2803,"cmd_vec":[{"delete_message":2803},{"keyboard":{"action_type":"hour","text":"Ok, now write the time of event"}}]}"#);

    case.send(r#"{"KeyboardMessage":{"uid":1,"msg_id":2804,"call_data":"time_hour:10","msg_text":"Please, choose hour"}}"#);
    case.expect(r#"{"uid":1,"to_msg":2804,"cmd_vec":[{"delete_message":2804},{"keyboard":{"action_type":"minute","text":"Ok, 10. Now choose minute"}}]}"#);

    case.send(r#"{"KeyboardMessage":{"uid":1,"msg_id":2805,"call_data":"time_minute:00","msg_text":"Ok, 10. Now choose minute"}}"#);
    case.expect(r#"{"uid":1,"to_msg":2805,"cmd_vec":[{"delete_message":2805},{"send":{"text":"Now write event message"}}]}"#);
    case.send(r#"{"TextMessage":{"uid":1,"msg_id":2832,"message":"test message"}}"#);
    case.expect(r#"{"uid":1,"to_msg":2832,"cmd_vec":[{"send":{"text":"I'll remind you January  7 at 10:00\ntest message"}}]}"#);

    case.run();
}

#[test]
fn at_next_mounth() {
    let mut case = TestCase::create();

    case.send(r#"{"TextMessage":{"uid":1,"msg_id":2802,"message":"/at"}}"#);
    case.expect(r#"{"uid":1,"to_msg":2802,"cmd_vec":[{"calendar":{"action_type":"calendar","year":1970,"month":1,"tz":-3,"edit_cur_msg":false}}]}"#);

    case.send(r#"{"KeyboardMessage":{"uid":1,"msg_id":2803,"call_data":"next-month","msg_text":"Please, choose a date"}}"#);
    case.expect(r#"{"uid":1,"to_msg":2803,"cmd_vec":[{"calendar":{"action_type":"calendar","year":1970,"month":2,"tz":-3,"edit_cur_msg":true}}]}"#);

    case.send(r#"{"KeyboardMessage":{"uid":1,"msg_id":2803,"call_data":"next-month","msg_text":"Please, choose a date"}}"#);
    case.expect(r#"{"uid":1,"to_msg":2803,"cmd_vec":[{"calendar":{"action_type":"calendar","year":1970,"month":3,"tz":-3,"edit_cur_msg":true}}]}"#);

    case.send(r#"{"KeyboardMessage":{"uid":1,"msg_id":2803,"call_data":"previous-month","msg_text":"Please, choose a date"}}"#);
    case.expect(r#"{"uid":1,"to_msg":2803,"cmd_vec":[{"calendar":{"action_type":"calendar","year":1970,"month":2,"tz":-3,"edit_cur_msg":true}}]}"#);

    case.send(r#"{"KeyboardMessage":{"uid":1,"msg_id":2803,"call_data":"calendar-day-1","msg_text":"Please, choose a date"}}"#);
    case.expect(r#"{"uid":1,"to_msg":2803,"cmd_vec":[{"delete_message":2803},{"keyboard":{"action_type":"hour","text":"Ok, now write the time of event"}}]}"#);

    case.send(r#"{"KeyboardMessage":{"uid":1,"msg_id":2804,"call_data":"time_hour:10","msg_text":"Please, choose hour"}}"#);
    case.expect(r#"{"uid":1,"to_msg":2804,"cmd_vec":[{"delete_message":2804},{"keyboard":{"action_type":"minute","text":"Ok, 10. Now choose minute"}}]}"#);

    case.send(r#"{"KeyboardMessage":{"uid":1,"msg_id":2805,"call_data":"time_minute:00","msg_text":"Ok, 10. Now choose minute"}}"#);
    case.expect(r#"{"uid":1,"to_msg":2805,"cmd_vec":[{"delete_message":2805},{"send":{"text":"Now write event message"}}]}"#);
    case.send(r#"{"TextMessage":{"uid":1,"msg_id":2832,"message":"test message"}}"#);
    case.expect(r#"{"uid":1,"to_msg":2832,"cmd_vec":[{"send":{"text":"I'll remind you February  1 at 10:00\ntest message"}}]}"#);

    case.run();
}

#[test]
fn at_today() {
    let mut case = TestCase::create();

    case.send(r#"{"TextMessage":{"uid":1,"msg_id":2802,"message":"/at"}}"#);
    case.expect(r#"{"uid":1,"to_msg":2802,"cmd_vec":[{"calendar":{"action_type":"calendar","year":1970,"month":1,"tz":-3,"edit_cur_msg":false}}]}"#);

    case.send(r#"{"KeyboardMessage":{"uid":1,"msg_id":2803,"call_data":"today","msg_text":"Please, choose a date"}}"#);
    case.expect(r#"{"uid":1,"to_msg":2803,"cmd_vec":[{"delete_message":2803},{"keyboard":{"action_type":"hour","text":"Ok, now write the time of event"}}]}"#);

    case.send(r#"{"KeyboardMessage":{"uid":1,"msg_id":2804,"call_data":"time_hour:10","msg_text":"Please, choose hour"}}"#);
    case.expect(r#"{"uid":1,"to_msg":2804,"cmd_vec":[{"delete_message":2804},{"keyboard":{"action_type":"minute","text":"Ok, 10. Now choose minute"}}]}"#);

    case.send(r#"{"KeyboardMessage":{"uid":1,"msg_id":2805,"call_data":"time_minute:00","msg_text":"Ok, 10. Now choose minute"}}"#);
    case.expect(r#"{"uid":1,"to_msg":2805,"cmd_vec":[{"delete_message":2805},{"send":{"text":"Now write event message"}}]}"#);
    case.send(r#"{"TextMessage":{"uid":1,"msg_id":2832,"message":"test message"}}"#);
    case.expect(r#"{"uid":1,"to_msg":2832,"cmd_vec":[{"send":{"text":"I'll remind you today at 10:00\ntest message"}}]}"#);

    case.run();
}

#[test]
fn at_tomorrow() {
    let mut case = TestCase::create();

    case.send(r#"{"TextMessage":{"uid":1,"msg_id":2802,"message":"/at"}}"#);
    case.expect(r#"{"uid":1,"to_msg":2802,"cmd_vec":[{"calendar":{"action_type":"calendar","year":1970,"month":1,"tz":-3,"edit_cur_msg":false}}]}"#);

    case.send(r#"{"KeyboardMessage":{"uid":1,"msg_id":2803,"call_data":"tomorrow","msg_text":"Please, choose a date"}}"#);
    case.expect(r#"{"uid":1,"to_msg":2803,"cmd_vec":[{"delete_message":2803},{"keyboard":{"action_type":"hour","text":"Ok, now write the time of event"}}]}"#);

    case.send(r#"{"KeyboardMessage":{"uid":1,"msg_id":2804,"call_data":"time_hour:10","msg_text":"Please, choose hour"}}"#);
    case.expect(r#"{"uid":1,"to_msg":2804,"cmd_vec":[{"delete_message":2804},{"keyboard":{"action_type":"minute","text":"Ok, 10. Now choose minute"}}]}"#);

    case.send(r#"{"KeyboardMessage":{"uid":1,"msg_id":2805,"call_data":"time_minute:00","msg_text":"Ok, 10. Now choose minute"}}"#);
    case.expect(r#"{"uid":1,"to_msg":2805,"cmd_vec":[{"delete_message":2805},{"send":{"text":"Now write event message"}}]}"#);
    case.send(r#"{"TextMessage":{"uid":1,"msg_id":2832,"message":"test message"}}"#);
    case.expect(r#"{"uid":1,"to_msg":2832,"cmd_vec":[{"send":{"text":"I'll remind you tomorrow at 10:00\ntest message"}}]}"#);

    case.run();
}

#[test]
fn at_write_time_by_hand() {
    // + negative
    let mut case = TestCase::create();

    case.send(r#"{"TextMessage":{"uid":1,"msg_id":2802,"message":"/at"}}"#);
    case.expect(r#"{"uid":1,"to_msg":2934,"cmd_vec":[{"calendar":{"action_type":"calendar","year":2020,"month":8,"tz":-3,"edit_cur_msg":false}}]}"#);
    case.send(r#"{"KeyboardMessage":{"uid":1,"msg_id":2935,"call_data":"tomorrow","msg_text":"Please, choose a date"}}"#);
    case.expect(r#"{"uid":1,"to_msg":2935,"cmd_vec":[{"delete_message":2935},{"keyboard":{"action_type":"hour","text":"Ok, now write the time of event"}}]}"#);
    case.send(r#"{"KeyboardMessage":{"uid":1,"msg_id":2936,"call_data":"time_hour:10","msg_text":"Please, choose hour"}}"#);
    case.expect(r#"{"uid":1,"to_msg":2936,"cmd_vec":[{"delete_message":2936},{"keyboard":{"action_type":"minute","text":"Ok, 10. Now choose minute"}}]}"#);
    case.send(r#"{"TextMessage":{"uid":1,"msg_id":2938,"message":"33"}}"#);
    case.expect(r#"{"uid":1,"to_msg":2938,"cmd_vec":[{"delete_message":0},{"send":{"text":"Now write event message"}}]}"#);
    case.send(r#"{"TextMessage":{"uid":1,"msg_id":2939,"message":"test"}}"#);
    case.expect(r#"{"uid":1,"to_msg":2939,"cmd_vec":[{"send":{"text":"I'll remind you tomorrow at 10:33\ntest"}}]}"#);
}

#[test]
fn after_wrong_format() {
    let mut case = TestCase::create();

    case.send(r#"{"TextMessage":{"uid":1,"msg_id":2970,"message":"test"}}"#);
    case.expect(
        r#"{"uid":1,"to_msg":2970,"cmd_vec":[{"keyboard":{"action_type":"main","text":"test"}}]}"#,
    );
    case.send(
        r#"{"KeyboardMessage":{"uid":1,"msg_id":2971,"call_data":"after","msg_text":"test"}}"#,
    );
    case.expect(
        r#"{"uid":1,"to_msg":2971,"cmd_vec":[{"send":{"text":"Ok, now write time duration."}}]}"#,
    );
    case.send(r#"{"TextMessage":{"uid":1,"msg_id":2973,"message":"0"}}"#);
    case.expect(
        r#"{"uid":1,"to_msg":2973,"cmd_vec":[{"send":{"text":"Wrong `after` command format"}}]}"#,
    );

    case.run()
}

// add test:
// - insert more earlier event
