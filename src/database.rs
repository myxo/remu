use command::{Command, OneTimeEventImpl, RepetitiveEventImpl};
use chrono::prelude::*;
use rusqlite::Connection;

pub struct DataBase {
    conn: Connection,
}

#[derive(Debug)]
struct Res {
    id: i32,
    com: Command,
}

impl DataBase {
    pub fn new() -> DataBase {
        let conn = Connection::open("database.db").expect("Cannot connect to sqlite");
        conn.execute(SQL_CREATE_ONE_TIME_EVENT_TABLE, &[]).expect("Cannot create one_time_event table");
        DataBase { conn }
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

        let mut stmt = self.conn.prepare(SQL_SELECT_BY_TIMESTAMP).expect("error in sql connection prepare");
        let command_iter = stmt.query_map(&[&event_timestamp], |row| {
            let id = row.get(0);
            let c = Command::OneTimeEvent( OneTimeEventImpl {
                event_text: row.get(1), 
                event_time: Utc.timestamp(row.get(2), 0),
            });

            Res {id: id, com: c}
        }).expect("error in query map");

        for command in command_iter {
            let c = command.unwrap();
            let id = c.id;
            self.conn.execute(SQL_DELETE_ONE_TIME_EVENT, &[&id]).expect("Cannot remove from one_time_event table");
            return Some(c.com);
        }

        None
    }

    pub fn get_nearest_wakeup(&self) -> Option<DateTime<Utc>> {
        self.conn.query_row(SQL_MIN_TIMESTAMP_ONE_TIME_EVENT, &[], |row| {
            let result = row.get_checked(0);
            match result {
                Ok(expr) => Some(Utc.timestamp(expr, 0)),
                Err(_) => None,
            }
        }).unwrap()
    }


    pub fn get_all_active_events(&self) -> Vec<Command> {
        let mut result = Vec::new();

        let mut stmt = self.conn.prepare(SQL_SELECT_ALL_LIMIT).expect("error in sql connection prepare");
        let command_iter = stmt.query_map(&[], |row| {
            let c = Command::OneTimeEvent( OneTimeEventImpl {
                event_text: row.get(1), 
                event_time: Utc.timestamp(row.get(2), 0),
            });

            c
        }).expect("error in query map");

        for command in command_iter {
            result.push(command.unwrap());
        }

        result
    }


    fn put_one_time_event(&mut self, command: &OneTimeEventImpl){
        let event_time = command.event_time.timestamp();
        let res = self.conn.execute(SQL_INSERT_ONE_TIME_EVENT, &[&command.event_text, &event_time]);
        if res.is_err() {
            error!("Can't insert one time event in db. Reasone: {}", res.unwrap_err());
        }
        // self.conn.execute(INSERT_ONE_TIME_EVENT, &[&command.event_text, &command.event_time]);
    }

    fn put_repetitive_event(&mut self, command: &RepetitiveEventImpl){

    }
}


const SQL_CREATE_ONE_TIME_EVENT_TABLE : &str = 
    "CREATE TABLE IF NOT EXISTS one_time_event(
        id                  INTEGER PRIMARY KEY,
        message_text        TEXT NOT NULL,
        event_time          INTEGER
    )";

const SQL_INSERT_ONE_TIME_EVENT : &str = 
    "INSERT INTO one_time_event(message_text, event_time) VALUES (?1, ?2);";

const SQL_SELECT_BY_TIMESTAMP : &str = 
    "SELECT id, message_text, event_time FROM one_time_event WHERE event_time = ?1;";

const SQL_DELETE_ONE_TIME_EVENT : &str =
    "DELETE FROM one_time_event WHERE id = ?1;";

const SQL_MIN_TIMESTAMP_ONE_TIME_EVENT : &str =
    "SELECT min(event_time) FROM one_time_event;";

const SQL_SELECT_ALL_LIMIT : &str = 
    "SELECT id, message_text, event_time FROM one_time_event ORDER BY event_time LIMIT 20;";