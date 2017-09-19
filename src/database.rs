use command::{Command, OneTimeEventImpl};
use std::collections::HashMap;
use chrono::prelude::*;
use rusqlite::Connection;

pub struct DataBase {
    // db_link: HashMap<DateTime<Local>, Command>,
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

    pub fn put(&mut self, key: DateTime<Local>, value: Command) {
        // self.db_link.insert(key, value.clone());
        match value {
            Command::BadCommand => warn!("Can't put BadCommand in database"),
            Command::OneTimeEvent(e) => self.put_one_time_event(&e),
        }
    }

    pub fn pop(&mut self, key: DateTime<Local>) -> Option<Command> {
        let event_timestamp = key.timestamp();

        let mut id : i32 = 5;


        let mut stmt = self.conn.prepare(SQL_SELECT_BY_TIMESTAMP).expect("error in prepare");
        let person_iter = stmt.query_map(&[&event_timestamp], |row| {
            let id = row.get(0);
            let c = Command::OneTimeEvent( OneTimeEventImpl {
                event_text: row.get(1), 
                event_time: Local.timestamp(row.get(2), 0),
            });

            Res {id: id, com: c}
        }).expect("error in query map");

        for person in person_iter {
            let p = person.unwrap();
            println!("Found person {:?}", p);
            id = p.id;
            self.conn.execute(SQL_DELETE_ONE_TIME_EVENT, &[&id]).expect("Cannot remove from one_time_event table");
            return Some(p.com);
        }

        None
    }

    pub fn get_nearest_wakeup(&self) -> Option<DateTime<Local>> {
        // self.db_link.keys().min().cloned()
        self.conn.query_row(SQL_MIN_TIMESTAMP_ONE_TIME_EVENT, &[], |row| {
            let result = row.get_checked(0);
            match result {
                Ok(expr) => Some(Local.timestamp(expr, 0)),
                Err(_) => None,
            }
        }).unwrap()
    }


    fn put_one_time_event(&mut self, command: &OneTimeEventImpl){
        let event_time = command.event_time.timestamp();
        let res = self.conn.execute(SQL_INSERT_ONE_TIME_EVENT, &[&command.event_text, &event_time]);
        if res.is_err() {
            error!("Can't insert one time event in db. Reasone: {}", res.unwrap_err());
        }
        // self.conn.execute(INSERT_ONE_TIME_EVENT, &[&command.event_text, &command.event_time]);
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