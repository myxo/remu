use command::{Command, OneTimeEventImpl, RepetitiveEventImpl};
use chrono;
use chrono::prelude::*;
use rusqlite::Connection;
use sql_query as sql_q;

pub struct DataBase {
    conn: Connection,
}

impl DataBase {
    pub fn new() -> DataBase {
        let conn = Connection::open("database.db").expect("Cannot connect to sqlite");
        conn.execute(sql_q::CREATE_USER_TABLE, &[]).expect("Cannot create user table");
        conn.execute(sql_q::CREATE_ACTIVE_EVENT_TABLE, &[]).expect("Cannot create active_event table");
        conn.execute(sql_q::CREATE_REP_EVENT_TABLE, &[]).expect("Cannot create scheduled_event table");
        conn.execute(sql_q::CREATE_GROUP_TABLE, &[]).expect("Cannot create group table");
        conn.execute(sql_q::CREATE_GROUP_ITEM_TABLE, &[]).expect("Cannot create group_item table");
        conn.execute(sql_q::PRAGMA_FOREING_KEY, &[]).expect("Cannot apply pragma foreing key");
        DataBase { conn: conn }
    }

    pub fn add_user(&mut self, uid: i64, username: &str, chat_id: i64, tz: i32){
        let res = self.conn.execute(sql_q::INSERT_USER, &[&uid, 
                                                          &username, 
                                                          &chat_id,
                                                          &tz
                                                        ]);
        if res.is_err() {
            error!("Can't insert user in db. Reasone: {}", res.unwrap_err());
        }
    }

    pub fn put(&mut self, uid: i64, value: Command) {
        match value {
            Command::BadCommand => warn!("Can't put BadCommand in database"),
            Command::OneTimeEvent(ev) => self.put_one_time_event(uid, &ev),
            Command::RepetitiveEvent(ev) => self.put_repetitive_event(uid, &ev),
        }
    }

    pub fn pop(&mut self, key: DateTime<Utc>) -> Option<(Command, i64)> {
        let event_timestamp = key.timestamp();
        let mut result = None;
        let mut parent_id: i32 = -1;
        let mut uid: i64 = -1;

        {
            let mut stmt = self.conn.prepare(sql_q::SELECT_ACTIVE_EVENT_BY_TIMESTAMP).expect("error in sql connection prepare");
            let mut rows = (stmt.query(&[&event_timestamp])).unwrap();

            while let Some(result_row) = rows.next() {
                let row = result_row.unwrap();
                let id: i64 = row.get(0);
                parent_id = row.get(3);
                let c = Command::OneTimeEvent( OneTimeEventImpl {
                    event_text: row.get(1), 
                    event_time: Utc.timestamp(row.get(2), 0),
                });
                uid = row.get(4);
                self.conn.execute(sql_q::DELETE_FROM_ACTIVE_EVENT_BY_ID, &[&id]).expect("Cannot remove from one_time_event table");
                result = Some((c, uid));
                break;
            }
        }

        if parent_id != -1 {
            let event = self.conn.query_row(sql_q::SELECT_REP_BY_ID, &[&parent_id], |row| { 
                get_nearest_active_event_from_repetitive_params(row.get(2), row.get(3), row.get(1))
            });
            let event = event.unwrap();
            let res = self.conn.execute(sql_q::INSERT_ACTIVE_EVENT, &[&event.event_text, &event.event_time.timestamp(), &uid, &parent_id]);
            if res.is_err() {
                error!("Can't insert one time event in db. Reasone: {}", res.unwrap_err());
            }
        }

        result
    }


    pub fn get_nearest_wakeup(&self) -> Option<DateTime<Utc>> {
        self.conn.query_row(sql_q::MIN_TIMESTAMP_FROM_ACTIVE_EVENT, &[], |row| {
            let result = row.get_checked(0);
            match result {
                Ok(expr) => Some(Utc.timestamp(expr, 0)),
                Err(_) => None,
            }
        }).unwrap()
    }


    pub fn get_all_active_events(&self, uid: i64) -> Vec<Command> {
        let mut result = Vec::new();

        let mut stmt = self.conn.prepare(sql_q::SELECT_ALL_ACTIVE_EVENT_BY_UID_LIMIT).expect("error in sql connection prepare");
        let command_iter = stmt.query_map(&[&uid], |row| {
            Command::OneTimeEvent( OneTimeEventImpl {
                event_text: row.get(1), 
                event_time: Utc.timestamp(row.get(2), 0),
            })
        }).expect("error in query map");

        for command in command_iter {
            result.push(command.unwrap());
        }

        result
    }


    pub fn get_all_rep_events(&self, uid: i64) -> Vec<(Command, i64)> {
        let mut result = Vec::new();

        let mut stmt = self.conn.prepare(sql_q::SELECT_ALL_REP_BY_UID_LIMIT).expect("error in sql connection prepare");
        let command_iter = stmt.query_map(&[&uid], |row| {
            (Command::RepetitiveEvent( RepetitiveEventImpl {
                event_text: row.get(1), 
                event_start_time: Utc.timestamp(row.get(2), 0),
                event_wait_time: chrono::Duration::seconds(row.get(3)),
                
            }), row.get(0) ) 
        }).expect("error in query map");

        for command in command_iter {
            result.push(command.unwrap());
        }

        result
    }


    pub fn delete_rep_event(&mut self, event_id: i64){
        self.conn.execute(sql_q::DELETE_FROM_REP_BY_ID, &[&event_id]).expect("Cannot del rep event");
        self.conn.execute(sql_q::DELETE_FROM_ACTIVE_EVENT_BY_PARENT_ID, &[&event_id]).expect("Cannot del rep event");
    }


    fn put_one_time_event(&mut self, uid: i64, command: &OneTimeEventImpl){
        let event_time = command.event_time.timestamp();
        let parent_id = -1;
        let res = self.conn.execute(sql_q::INSERT_ACTIVE_EVENT, &[&command.event_text, &event_time, &uid, &parent_id]);
        if res.is_err() {
            error!("Can't insert one time event in db. Reasone: {}", res.unwrap_err());
        }
    }

    fn put_repetitive_event(&mut self, uid: i64, command: &RepetitiveEventImpl){
        let event_time: i64 = command.event_start_time.timestamp();
        let event_wait: i64 = command.event_wait_time.num_seconds();
        let res = self.conn.execute(sql_q::INSERT_REP_EVENT, &[&command.event_text, &event_time, &event_wait, &uid]);
        if res.is_err() {
            error!("Can't insert repetitive event in db. Reasone: {}", res.unwrap_err());
        }

        let id = self.conn.last_insert_rowid();
        let active_event = get_nearest_active_event_from_repetitive_params(
                command.event_start_time.timestamp(), 
                command.event_wait_time.num_seconds(), 
                command.event_text.clone());

        let res = self.conn.execute(sql_q::INSERT_ACTIVE_EVENT, &[&active_event.event_text, &active_event.event_time.timestamp(), &uid, &id]);
        if res.is_err() {
            error!("Can't insert one time event in db. Reasone: {}", res.unwrap_err());
        }
    }

} // impl DataBase 


fn get_nearest_active_event_from_repetitive_params(start_time: i64, wait_time: i64, text: String) -> OneTimeEventImpl{
        let now = Utc::now();
        let wait_time = if wait_time < 0 {1} else {wait_time}; // TODO: make propper error handling
        let dt = chrono::Duration::seconds(wait_time);
        let mut event_time = Utc.timestamp(start_time, 0);

        while event_time < now {
            event_time =  event_time + dt;
        }
        OneTimeEventImpl{
            event_text: text,
            event_time: event_time,
        }
    }
