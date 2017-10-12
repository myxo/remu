// #![feature(drop_types_in_const)]
#![feature(string_retain)]
extern crate chrono;
extern crate regex;
#[macro_use]
extern crate cpython;
#[macro_use]
extern crate log;
extern crate fern;
extern crate rusqlite;

mod command;
pub mod engine;
mod database;
mod sql_query;

use engine::Engine;

use cpython::{Python, PyResult, PyObject, ObjectProtocol, PyTuple, ToPyObject, PythonObject};
use std::thread;
use std::io;

static mut ENG : Option<Engine> = None;
static mut CALLBACK : Option<PyObject> = None;

py_module_initializer!(libremu_backend, 
    initlibremu_backend, 
    PyInit_libremu_backend, 
    |py, m| 
    {
        m.add(py, "initialize", py_fn!(py, initialize(verbose: bool)))?;
        m.add(py, "run", py_fn!(py, run()))?;
        m.add(py, "stop", py_fn!(py, stop()))?;
        m.add(py, "register_callback", py_fn!(py, register_callback(obj: PyObject)))?;
        m.add(py, "add_user", py_fn!(py, add_user(uid: i64, username: &str, chat_id: i64, tz: i32)))?;
        m.add(py, "handle_text_message", py_fn!(py, handle_text_message(uid: i64, message: &str)))?;
        m.add(py, "get_active_events", py_fn!(py, get_active_events(uid: i64)))?;
        m.add(py, "get_rep_events", py_fn!(py, get_rep_events(uid: i64)))?;
        m.add(py, "del_rep_event", py_fn!(py, del_rep_event(event_id: i64)))?;

        Ok(())
    });

fn initialize(_py : Python, verbose: bool) -> PyResult<(bool)>{
    setup_logging(3, verbose).expect("ERROR in logging initialization.");
    unsafe {
        ENG = Some(Engine::new());
    }
    Ok((true))
}

fn run(_py : Python) -> PyResult<(u64)>{
    thread::spawn( ||{
        unsafe{
            ENG.as_mut().expect("initialize engine!").run();
        }
    });
    Ok((64))
}

fn stop(_py : Python) -> PyResult<(u64)>{
    unsafe {
        ENG.as_mut().expect("initialize engine!").stop();
    }
    Ok((64))
}

fn add_user(_py : Python, uid: i64, username: &str, chat_id: i64, tz: i32) -> PyResult<(u64)>{
    unsafe {
        ENG.as_mut().expect("initialize engine!").add_user(uid, username, chat_id, tz);
    }
    Ok((64))
}

fn handle_text_message(_py : Python, uid: i64, message : &str) -> PyResult<String>{
    let out;
    unsafe{
        out = ENG.as_mut().expect("initialize engine!").handle_text_message(uid, message);
    }
    Ok(out)
}


fn get_active_events(_py : Python, uid: i64) -> PyResult<Vec<String>>{
    let out;
    unsafe{
        out = ENG.as_mut().expect("initialize engine!").get_active_event_list(uid);
    }
    Ok(out)
}

fn get_rep_events(_py : Python, uid: i64) -> PyResult<Vec<(String, i64)>>{
    let out;
    unsafe{
        out = ENG.as_mut().expect("initialize engine!").get_rep_event_list(uid);
    }
    Ok(out)
}

fn del_rep_event(_py: Python, event_id: i64) -> PyResult<(i64)> {
    unsafe{
        ENG.as_mut().expect("initialize engine!").delete_rep_event(event_id);
    }
    Ok(64)
}

fn engine_callback(text: String, uid: i64){
    unsafe{
        if CALLBACK.is_some() {
            let gil = Python::acquire_gil();
            let py = gil.python();
            let py_turple = PyTuple::new(py, &[text.to_py_object(py).into_object(), uid.to_py_object(py).into_object(),]);
            let _res = CALLBACK.as_mut().unwrap().call(py, py_turple, None);
        }
    }
}

fn register_callback(_py : Python, obj : PyObject) -> PyResult<bool>{
    if obj.is_callable(_py) {
        unsafe{
            CALLBACK = Some(obj);
            ENG.as_mut().expect("initialize engine!").register_callback(engine_callback);
        }
        return Ok(true);
    }
    Ok(false)
}




fn setup_logging(verbosity: u64, console_output_enabled: bool) -> Result<(), fern::InitError> {
    let mut base_config = fern::Dispatch::new();

    base_config = match verbosity {
        0 => {
            // Let's say we depend on something which whose "info" level messages are too verbose
            // to include in end-user output. If we don't need them, let's not include them.
            base_config
                .level(log::LogLevelFilter::Info)
                .level_for("overly-verbose-target", log::LogLevelFilter::Warn)
        }
        1 => base_config
            .level(log::LogLevelFilter::Debug)
            .level_for("overly-verbose-target", log::LogLevelFilter::Info),
        2 => base_config.level(log::LogLevelFilter::Debug),
        _3_or_more => base_config.level(log::LogLevelFilter::Trace),
    };

    // Separate file config so we can include year, month and day in file logs
    let file_config = fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{}[{}][{}] {}",
                chrono::Local::now().format("[%Y-%m-%d][%H:%M:%S]"),
                record.target(),
                record.level(),
                message
            ))
        })
        .chain(fern::log_file("log-engine.txt")?);

    let stdout_config = fern::Dispatch::new()
        .format(|out, message, record| {
            // special format for debug messages coming from our own crate.
            if record.level() > log::LogLevelFilter::Info && record.target() == "cmd_program" {
                out.finish(format_args!("---\nDEBUG: {}: {}\n---",
                                        chrono::Local::now().format("%H:%M:%S"),
                                        message))
            } else {
                out.finish(format_args!("[{}][{}][{}] {}",
                                        chrono::Local::now().format("%H:%M:%S"),
                                        record.target(),
                                        record.level(),
                                        message))
            }
        })
        .chain(io::stdout());

    if console_output_enabled {
        base_config.chain(file_config).chain(stdout_config).apply()?;
    } else {
        base_config.chain(file_config).apply()?;
    }

    Ok(())
}
