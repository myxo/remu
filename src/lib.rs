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
        m.add(py, "add_user", py_fn!(py, add_user(uid: i64, username: &str, chat_id: i64, first_name: &str, last_name: &str, tz: i32)))?;
        m.add(py, "handle_text_message", py_fn!(py, handle_text_message(uid: i64, message: &str)))?;
        m.add(py, "handle_keyboard_responce", py_fn!(py, handle_keyboard_responce(uid: i64, call_data: &str, msg_text: &str)))?;

        m.add(py, "get_user_chat_id_all", py_fn!(py, get_user_chat_id_all()))?;
        
        m.add(py, "register_callback", py_fn!(py, register_callback(obj: PyObject)))?;

        m.add(py, "log_debug", py_fn!(py, log_debug(s: &str)))?;
        m.add(py, "log_info", py_fn!(py, log_info(s: &str)))?;
        m.add(py, "log_error", py_fn!(py, log_error(s: &str)))?;

        Ok(())
    });

fn initialize(_py : Python, verbose: bool) -> PyResult<bool>{
    setup_logging(3, verbose).unwrap_or_else(
        |err| { 
            error!("Logging init failed, reasone: {}", err); 
        });
    unsafe {
        ENG = Some(Engine::new(false));
    }
    Ok(true)
}

fn run(_py : Python) -> PyResult<bool>{
    thread::spawn( ||{
        unsafe{
            ENG.as_mut().expect("initialize engine!").run();
        }
    });
    Ok(true)
}

fn stop(_py : Python) -> PyResult<bool>{
    unsafe {
        ENG.as_mut().expect("initialize engine!").stop();
    }
    Ok(true)
}

fn add_user(_py : Python, uid: i64, username: &str, chat_id: i64, first_name: &str, last_name: &str, tz: i32) -> PyResult<bool>{
    unsafe {
        Ok(ENG.as_mut().expect("initialize engine!").add_user(uid, username, chat_id, first_name, last_name, tz))
    }
}

fn handle_text_message(_py : Python, uid: i64, message : &str) -> PyResult<String>{
    unsafe{
        Ok(ENG.as_mut().expect("initialize engine!").handle_text_message(uid, message))
    }
}

fn handle_keyboard_responce(_py : Python, uid: i64, call_data : &str, msg_text : &str) -> PyResult<String>{
    unsafe{
        Ok(ENG.as_mut().expect("initialize engine!").handle_keyboard_responce(uid, call_data, msg_text))
    }
}

pub fn get_user_chat_id_all(_py: Python) -> PyResult<Vec<i32>> {
    unsafe{
        Ok(ENG.as_mut().expect("initialize engine!").get_user_chat_id_all())
    }
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

fn log_debug(_py : Python, s: &str) -> PyResult<bool> {
    debug!("[Frontend] {}", s);
    Ok(true)
}

fn log_info(_py : Python, s: &str) -> PyResult<bool> {
    info!("[Frontend] {}", s);
    Ok(true)
}

fn log_error(_py : Python, s: &str) -> PyResult<bool> {
    error!("[Frontend] {}", s);
    Ok(true)
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
        .chain(fern::log_file("log.txt")?);

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
