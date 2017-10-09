use command::{Command, OneTimeEventImpl, RepetitiveEventImpl};
use chrono;
use chrono::prelude::*;
use rusqlite::Connection;

pub struct DataBase {
    conn: Connection,
}

#[derive(Debug)]
struct Res {
    id: i32,
    parent_id: i32,
    com: Command,
}

impl DataBase {
    pub fn new() -> DataBase {
        let conn = Connection::open("database.db").expect("Cannot connect to sqlite");
        conn.execute(SQL_CREATE_ACTIVE_EVENT_TABLE, &[]).expect("Cannot create one_time_event table");
        conn.execute(SQL_CREATE_REP_EVENT_TABLE, &[]).expect("Cannot create one_time_event table");
        DataBase { conn: conn }
    }

    pub fn put(&mut self, value: Command) {
        match value {
            Command::BadCommand => warn!("Can't put BadCommand in database"),
            Command::OneTimeEvent(ev) => self.put_one_time_event(&ev),
            Command::RepetitiveEvent(ev) => self.put_repetitive_event(&ev),
        }
    }

    pub fn pop(&mut self, key: DateTime<Utc>) -> Option<Command> {
        let event_timestamp = key.timestamp();
        let mut result = None;
        let mut parent_id: i32 = -1;

        {
            let mut stmt = self.conn.prepare(SQL_SELECT_ACTIVE_EVENT_BY_TIMESTAMP).expect("error in sql connection prepare");
            let command_iter = stmt.query_map(&[&event_timestamp], |row| {
                let id = row.get(0);
                let c = Command::OneTimeEvent( OneTimeEventImpl {
                    event_text: row.get(1), 
                    event_time: Utc.timestamp(row.get(2), 0),
                });
                let parent_id = row.get(3);

                Res {id: id, parent_id: parent_id, com: c}
            }).expect("error in query map");

            for command in command_iter {
                let c = command.unwrap();
                let id = c.id;
                parent_id = c.parent_id;
                self.conn.execute(SQL_DELETE_FROM_ACTIVE_EVENT_BY_ID, &[&id]).expect("Cannot remove from one_time_event table");
                result = Some(c.com);
                break;
            }
        }

        if parent_id != -1 {
            let event = self.conn.query_row(SQL_SELECT_REP_BY_ID, &[&parent_id], |row| { 
                get_nearest_active_event_from_repetitive_params(row.get(2), row.get(3), row.get(1))
            });
            let event = event.unwrap();
            let res = self.conn.execute(SQL_INSERT_ACTIVE_EVENT, &[&event.event_text, &event.event_time.timestamp(), &parent_id]);
            if res.is_err() {
                error!("Can't insert one time event in db. Reasone: {}", res.unwrap_err());
            }
        }

        result
    }


    pub fn get_nearest_wakeup(&self) -> Option<DateTime<Utc>> {
        self.conn.query_row(SQL_MIN_TIMESTAMP_FROM_ACTIVE_EVENT, &[], |row| {
            let result = row.get_checked(0);
            match result {
                Ok(expr) => Some(Utc.timestamp(expr, 0)),
                Err(_) => None,
            }
        }).unwrap()
    }


    pub fn get_all_active_events(&self) -> Vec<Command> {
        let mut result = Vec::new();

        let mut stmt = self.conn.prepare(SQL_SELECT_ALL_ACTIVE_EVENT_LIMIT).expect("error in sql connection prepare");
        let command_iter = stmt.query_map(&[], |row| {
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


    fn put_one_time_event(&mut self, command: &OneTimeEventImpl){
        let event_time = command.event_time.timestamp();
        let parent_id = -1;
        let res = self.conn.execute(SQL_INSERT_ACTIVE_EVENT, &[&command.event_text, &event_time, &parent_id]);
        if res.is_err() {
            error!("Can't insert one time event in db. Reasone: {}", res.unwrap_err());
        }
    }

    fn put_repetitive_event(&mut self, command: &RepetitiveEventImpl){
        let event_time: i64 = command.event_start_time.timestamp();
        let event_wait: i64 = command.event_wait_time.num_seconds();
        let res = self.conn.execute(SQL_INSERT_REP_EVENT, &[&command.event_text, &event_time, &event_wait]);
        if res.is_err() {
            error!("Can't insert repetitive event in db. Reasone: {}", res.unwrap_err());
        }

        let id = self.conn.last_insert_rowid();
        let active_event = get_nearest_active_event_from_repetitive_params(
                command.event_start_time.timestamp(), 
                command.event_wait_time.num_seconds(), 
                command.event_text.clone());

        let res = self.conn.execute(SQL_INSERT_ACTIVE_EVENT, &[&active_event.event_text, &active_event.event_time.timestamp(), &id]);
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


// SQL one time events --------------------------------------------

const SQL_CREATE_ACTIVE_EVENT_TABLE: &str = 
    "CREATE TABLE IF NOT EXISTS active_event(
        id                  INTEGER PRIMARY KEY,
        parent_id           INTEGER,
        message_text        TEXT NOT NULL,
        event_time          INTEGER
    )";

const SQL_INSERT_ACTIVE_EVENT: &str = 
    "INSERT INTO active_event(message_text, event_time, parent_id) VALUES (?1, ?2, ?3);";

const SQL_SELECT_ACTIVE_EVENT_BY_TIMESTAMP: &str = 
    "SELECT id, message_text, event_time, parent_id FROM active_event WHERE event_time = ?1;";

const SQL_DELETE_FROM_ACTIVE_EVENT_BY_ID: &str =
    "DELETE FROM active_event WHERE id = ?1;";

const SQL_MIN_TIMESTAMP_FROM_ACTIVE_EVENT: &str =
    "SELECT min(event_time) FROM active_event;";

const SQL_SELECT_ALL_ACTIVE_EVENT_LIMIT: &str = 
    "SELECT id, message_text, event_time FROM active_event ORDER BY event_time LIMIT 20;";


// SQL rep events ------------------------------------------------

const SQL_CREATE_REP_EVENT_TABLE: &str = 
    "CREATE TABLE IF NOT EXISTS scheduled_event(
        id                  INTEGER PRIMARY KEY,
        message_text        TEXT NOT NULL,
        event_time          INTEGER,
        event_wait          INTEGER
    )";

const SQL_INSERT_REP_EVENT: &str = 
    "INSERT INTO scheduled_event(message_text, event_time, event_wait) VALUES (?1, ?2, ?3);";

// const SQL_SELECT_REP_BY_TIMESTAMP: &str = 
//     "SELECT id, message_text, event_time, event_wait FROM scheduled_event WHERE event_time = ?1;";

const SQL_SELECT_REP_BY_ID: &str = 
    "SELECT id, message_text, event_time, event_wait FROM scheduled_event WHERE id = ?1;";


// const SQL_DELETE_FROM_REP_BY_ID: &str =
//     "DELETE FROM scheduled_event WHERE id = ?1;";

// const SQL_MIN_TIMESTAMP_FROM_REP: &str =
//     "SELECT min(event_time) FROM scheduled_event;";

// const SQL_SELECT_ALL_REP_LIMIT: &str = 
//     "SELECT id, message_text, event_time, event_wait FROM scheduled_event ORDER BY event_time LIMIT 20;";