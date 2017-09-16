use command::Command;
use std::collections::HashMap;
use chrono::prelude::*;

pub struct DataBase {
    db_link: HashMap<DateTime<Local>, Command>,
}

impl DataBase {
    pub fn new() -> DataBase {
        DataBase { db_link: HashMap::new() }
    }

    pub fn put(&mut self, key: DateTime<Local>, value: Command) {
        self.db_link.insert(key, value);
    }

    pub fn pop(&mut self, key: DateTime<Local>) -> Option<Command> {
        self.db_link.remove(&key)
    }

    pub fn get_nearest_wakeup(&self) -> Option<DateTime<Local>> {
        self.db_link.keys().min().cloned()
    }
}