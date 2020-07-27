use crate::command::{Command, OneTimeEventImpl, RepetitiveEventImpl};
use crate::sql_query as sql_q;
use chrono::{Utc};
use chrono::prelude::*;
use rusqlite::{params, Connection};

pub struct DataBase {
    conn: Connection,
}

pub enum DbMode {
    InMemory,
    Filesystem,
}

pub struct UserInfo <'a>{
    pub uid: i64,
    pub name: &'a str,
    pub chat_id: i64,
    pub first_name: &'a str,
    pub last_name: &'a str,
    pub tz: i32,
}

impl DataBase {
    pub fn new(mode: DbMode) -> DataBase {
        let conn = match mode {
            DbMode::Filesystem => Connection::open("database.db").expect("Cannot connect to sqlite"),
            DbMode::InMemory => Connection::open_in_memory().expect("Cannot open db in memory"),
        };
        conn.execute(sql_q::CREATE_USER_TABLE, params![])
            .expect("Cannot create user table");
        conn.execute(sql_q::CREATE_ACTIVE_EVENT_TABLE, params![])
            .expect("Cannot create active_event table");
        conn.execute(sql_q::CREATE_REP_EVENT_TABLE, params![])
            .expect("Cannot create scheduled_event table");
        conn.execute(sql_q::PRAGMA_FOREING_KEY, params![])
            .expect("Cannot apply pragma foreing key");
        DataBase { conn }
    }

    pub fn add_user(
        &mut self,
        info : UserInfo
    ) -> Result<(), String> {
        let res = self.conn.execute(
            sql_q::INSERT_USER,
            params![&info.uid, &info.name, &info.first_name, &info.last_name, &info.chat_id, &info.tz],
        );
        match res {
            Ok(_) => Ok(()),
            Err(e) => Err(e.to_string()),
        }
    }

    pub fn put(&mut self, uid: i64, value: Command) -> bool {
        match value {
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
            let mut stmt = self
                .conn
                .prepare(sql_q::SELECT_ACTIVE_EVENT_BY_TIMESTAMP)
                .expect("error in sql connection prepare");
            let mut rows = (stmt.query(&[&event_timestamp])).unwrap();

            while let Ok(Some(row)) = rows.next() {
                let id: i64 = row.get(0).unwrap();
                parent_id = row.get(3).unwrap();
                let comm = Command::OneTimeEvent(OneTimeEventImpl {
                    event_text: row.get(1).unwrap(),
                    event_time: Utc.timestamp(row.get(2).unwrap(), 0),
                });
                uid = row.get(4).unwrap();
                self.conn
                    .execute(sql_q::DELETE_FROM_ACTIVE_EVENT_BY_ID, &[&id])
                    .expect("Cannot remove from one_time_event table");
                result = Some((comm, uid));
                break;
            }
        }

        if parent_id != -1 {
            let event = self
                .conn
                .query_row(sql_q::SELECT_REP_BY_ID, &[&parent_id], |row| {
                    Ok(get_nearest_active_event_from_repetitive_params(
                        row.get(2).unwrap(),
                        row.get(3).unwrap(),
                        row.get(1).unwrap(),
                    ))
                });
            let event = event.unwrap();
            let res = self.conn.execute(
                sql_q::INSERT_ACTIVE_EVENT,
                params![
                    &event.event_text,
                    &event.event_time.timestamp(),
                    &uid,
                    &parent_id,
                ],
            );
            if res.is_err() {
                error!(
                    "Can't insert one time event in db. Reasone: {}",
                    res.unwrap_err()
                );
            }
        }

        result
    }

    pub fn get_nearest_wakeup(&self) -> Option<DateTime<Utc>> {
        self.conn
            .query_row(sql_q::MIN_TIMESTAMP_FROM_ACTIVE_EVENT, params![], |row| {
                row.get(0).map(|expr| Utc.timestamp(expr, 0))
            }).ok()
    }

    pub fn get_all_active_events(&self, uid: i64) -> Vec<Command> {
        let mut result = Vec::new();

        let mut stmt = self
            .conn
            .prepare(sql_q::SELECT_ALL_ACTIVE_EVENT_BY_UID_LIMIT)
            .expect("error in sql connection prepare");
        let command_iter = stmt
            .query_map(&[&uid], |row| {
                Ok(Command::OneTimeEvent(OneTimeEventImpl {
                    event_text: row.get(1).unwrap(),
                    event_time: Utc.timestamp(row.get(2).unwrap(), 0),
                }))
            })
            .expect("error in query map");

        for command in command_iter {
            result.push(command.unwrap());
        }

        result
    }

    pub fn get_all_rep_events(&self, uid: i64) -> Vec<(Command, i64)> {
        let mut result = Vec::new();

        let mut stmt = self
            .conn
            .prepare(sql_q::SELECT_ALL_REP_BY_UID_LIMIT)
            .expect("error in sql connection prepare");
        let command_iter = stmt
            .query_map(&[&uid], |row| {
                Ok((
                    Command::RepetitiveEvent(RepetitiveEventImpl {
                        event_text: row.get(1).unwrap(),
                        event_start_time: Utc.timestamp(row.get(2).unwrap(), 0),
                        event_wait_time: chrono::Duration::seconds(row.get(3).unwrap()),
                    }),
                    row.get(0).unwrap(),
                ))
            })
            .expect("error in query map");

        for command in command_iter {
            result.push(command.unwrap());
        }

        result
    }

    pub fn delete_rep_event(&mut self, event_id: i64) -> bool {
        if self
            .conn
            .execute(sql_q::DELETE_FROM_REP_BY_ID, &[&event_id])
            .is_err()
        {
            return false;
        }
        if self
            .conn
            .execute(sql_q::DELETE_FROM_ACTIVE_EVENT_BY_PARENT_ID, &[&event_id])
            .is_err()
        {
            return false;
        }
        true
    }

    pub fn get_user_timezone(&self, uid: i64) -> i32 {
        let row = self
            .conn
            .query_row(sql_q::GET_USER_TIMEZONE, &[&uid], |row| row.get(0));
        row.unwrap()
    }

    pub fn get_user_chat_id_all(&self) -> Vec<i32> {
        let mut result = Vec::new();

        let mut stmt = self
            .conn
            .prepare(sql_q::GET_ALL_USER_CHAT_ID)
            .expect("error in sql connection prepare");
        stmt.query_map(params![], |row| row.get(0))
            .expect("error in query map")
            .for_each(|id| {
                result.push(id.unwrap());
            });

        result
    }

    fn put_one_time_event(&mut self, uid: i64, command: &OneTimeEventImpl) -> bool {
        let event_time = command.event_time.timestamp();
        let parent_id = -1;
        let res = self.conn.execute(
            sql_q::INSERT_ACTIVE_EVENT,
            params![&command.event_text, &event_time, &uid, &parent_id],
        );
        if res.is_err() {
            error!(
                "Can't insert one time event in db. Reasone: {}",
                res.unwrap_err()
            );
            return false;
        }
        true
    }

    fn put_repetitive_event(&mut self, uid: i64, command: &RepetitiveEventImpl) -> bool {
        let event_time: i64 = command.event_start_time.timestamp();
        let event_wait: i64 = command.event_wait_time.num_seconds();
        let res = self.conn.execute(
            sql_q::INSERT_REP_EVENT,
            params![&command.event_text, &event_time, &event_wait, &uid],
        );
        if res.is_err() {
            error!(
                "Can't insert repetitive event in db. Reasone: {}",
                res.unwrap_err()
            );
            return false;
        }

        let id = self.conn.last_insert_rowid();
        let active_event = get_nearest_active_event_from_repetitive_params(
            command.event_start_time.timestamp(),
            command.event_wait_time.num_seconds(),
            command.event_text.clone(),
        );

        let res = self.conn.execute(
            sql_q::INSERT_ACTIVE_EVENT,
            params![
                &active_event.event_text,
                &active_event.event_time.timestamp(),
                &uid,
                &id,
            ],
        );
        if res.is_err() {
            error!(
                "Can't insert one time event in db. Reasone: {}",
                res.unwrap_err()
            );
            return false;
        }
        true
    }
} // impl DataBase

fn get_nearest_active_event_from_repetitive_params(
    start_time: i64,
    wait_time: i64,
    text: String,
) -> OneTimeEventImpl {
    let now = Utc::now();
    let wait_time = if wait_time < 0 { 1 } else { wait_time }; // TODO: make propper error handling
    let dt = chrono::Duration::seconds(wait_time);
    let mut event_time = Utc.timestamp(start_time, 0);

    while event_time < now {
        event_time = event_time + dt;
    }
    OneTimeEventImpl {
        event_text: text,
        event_time,
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    // use crate::command::Command::*;

    #[test]
    fn add_user() {
        let mut db = DataBase::new(DbMode::InMemory);
        let info = UserInfo {
            uid: 1,
            name: "name",
            chat_id: 123,
            first_name: "first",
            last_name: "last",
            tz: -1,
        };
        assert!(db.add_user(info).is_ok());
        assert_eq!(db.get_user_timezone(1), -1);
        assert_eq!(db.get_user_chat_id_all(), vec!(123));

        let info = UserInfo {
            uid: 2,
            name: "name",
            chat_id: 1234,
            first_name: "first",
            last_name: "last",
            tz: -2,
        };
        assert!(db.add_user(info).is_ok());
        assert_eq!(db.get_user_timezone(2), -2);
        assert_eq!(db.get_user_chat_id_all(), vec!(123, 1234));
    }


    // #[test]
    // fn put_one_time_event_negative() {
    //     //
    // }

    #[test]
    fn put_one_time_event() {
        let mut db = DataBase::new(DbMode::InMemory);
        let info = UserInfo {
            uid: 1,
            name: "name",
            chat_id: 123,
            first_name: "first",
            last_name: "last",
            tz: -1,
        };
        db.add_user(info).unwrap();
        assert!(db.get_nearest_wakeup().is_none());

        // add event
        let event = Command::OneTimeEvent(OneTimeEventImpl {
            event_text: String::from("test"),
            event_time: Utc.timestamp(61, 0),
        });
        db.put(1, event);
        let wake = db.get_nearest_wakeup();
        assert!(wake.is_some());
        assert_eq!(wake.unwrap().timestamp(), 61);

        // pop event
        let event = db.pop(Utc.timestamp(61, 0));
        assert!(db.get_nearest_wakeup().is_none());
        assert!(event.is_some());
    }
}