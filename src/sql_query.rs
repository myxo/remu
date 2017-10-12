pub const PRAGMA_FOREING_KEY: &str = "PRAGMA foreign_keys = ON;";

pub const CREATE_USER_TABLE: &str = 
    "CREATE TABLE IF NOT EXISTS user(
        uid                 INTEGER PRIMARY KEY NOT NULL,
        username            TEXT NOT NULL,
        first_name          TEXT,
        last_name           TEXT,
        timezone            INTEGER,
        chat_id             INTEGER NOT NULL
    )";

pub const CREATE_ACTIVE_EVENT_TABLE: &str = 
    "CREATE TABLE IF NOT EXISTS active_event(
        id                  INTEGER PRIMARY KEY AUTOINCREMENT,
        uid                 INTEGER NOT NULL,
        parent_id           INTEGER,
        event_text          TEXT NOT NULL,
        event_time          INTEGER,
        FOREIGN KEY(uid)    REFERENCES user(uid)
    )";

pub const CREATE_REP_EVENT_TABLE: &str = 
    "CREATE TABLE IF NOT EXISTS scheduled_event(
        id                  INTEGER PRIMARY KEY AUTOINCREMENT,
        uid                 INTEGER NOT NULL,
        event_text          TEXT NOT NULL,
        event_time          INTEGER,
        event_wait          INTEGER,
        FOREIGN KEY(uid)    REFERENCES user(uid)
    )";

pub const CREATE_GROUP_TABLE: &str = 
    "CREATE TABLE IF NOT EXISTS group_list(
        gid                 INTEGER PRIMARY KEY AUTOINCREMENT,
        uid                 INTEGER NOT NULL,
        group_name          TEXT NOT NULL,
        FOREIGN KEY(uid)    REFERENCES user(uid)
    )";

pub const CREATE_GROUP_ITEM_TABLE: &str = 
    "CREATE TABLE IF NOT EXISTS group_item(
        id                  INTEGER PRIMARY KEY AUTOINCREMENT,
        gid                 INTEGER NOT NULL,
        group_item          TEXT,
        FOREIGN KEY(gid)    REFERENCES group_list(gid)
    )";


// SQL user --------------------------------------------

pub const INSERT_USER: &str =
    "INSERT INTO user(uid, username, chat_id, timezone) VALUES (?1, ?2, ?3, ?4);";

pub const GET_USER_TIMEZONE: &str = 
    "SELECT timezone FROM user WHERE uid = ?1;";

// SQL one time events --------------------------------------------


pub const INSERT_ACTIVE_EVENT: &str = 
    "INSERT INTO active_event(event_text, event_time, uid, parent_id) VALUES (?1, ?2, ?3, ?4);";

pub const SELECT_ACTIVE_EVENT_BY_TIMESTAMP: &str = 
    "SELECT id, event_text, event_time, parent_id, uid FROM active_event WHERE event_time = ?1;";

pub const DELETE_FROM_ACTIVE_EVENT_BY_ID: &str =
    "DELETE FROM active_event WHERE id = ?1;";

pub const DELETE_FROM_ACTIVE_EVENT_BY_PARENT_ID: &str =
    "DELETE FROM active_event WHERE parent_id = ?1;";

pub const MIN_TIMESTAMP_FROM_ACTIVE_EVENT: &str =
    "SELECT min(event_time) FROM active_event;";

pub const SELECT_ALL_ACTIVE_EVENT_BY_UID_LIMIT: &str = 
    "SELECT id, event_text, event_time FROM active_event WHERE uid = ?1 ORDER BY event_time LIMIT 20;";


// SQL rep events ------------------------------------------------


pub const INSERT_REP_EVENT: &str = 
    "INSERT INTO scheduled_event(event_text, event_time, event_wait, uid) VALUES (?1, ?2, ?3, ?4);";

pub const SELECT_REP_BY_ID: &str = 
    "SELECT id, event_text, event_time, event_wait FROM scheduled_event WHERE id = ?1;";

pub const DELETE_FROM_REP_BY_ID: &str =
    "DELETE FROM scheduled_event WHERE id = ?1;";

pub const SELECT_ALL_REP_BY_UID_LIMIT: &str = 
    "SELECT id, event_text, event_time, event_wait FROM scheduled_event WHERE uid = ?1 ORDER BY event_time LIMIT 20;";