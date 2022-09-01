#[macro_use]
extern crate cpython;
#[macro_use]
extern crate log;

mod command;
pub mod database;
pub mod engine;
mod helpers;
mod sql_query;
pub mod state;
pub mod time;

use engine::{engine_run, CmdToEngine};

use cpython::{ObjectProtocol, PyObject, PyResult, PyTuple, Python, PythonObject, ToPyObject};
use std::io;
use std::sync::mpsc;
use std::thread;

static mut CALLBACK: Option<PyObject> = None;
static mut TX_TO_ENGINE: Option<mpsc::Sender<CmdToEngine>> = None;

#[rustfmt::skip::macros(py_module_initializer)]
#[rustfmt::skip::macros(py_fn)]
py_module_initializer!(libremu_backend, |py, m|
    {
        m.add(py, "initialize", py_fn!(py, initialize(verbose: bool, callback: PyObject)))?;
        m.add(py, "stop", py_fn!(py, stop()))?;
        m.add(py, "add_user", py_fn!(py, add_user(uid: i64, username: &str, chat_id: i64, first_name: &str, last_name: &str, tz: i32)))?;
        m.add(py, "handle_text_message", py_fn!(py, handle_text_message(uid: i64, msg_id: i64, message: &str)))?;
        m.add(py, "handle_keyboard_responce", py_fn!(py, handle_keyboard_responce(uid: i64, msg_id: i64, call_data: &str, msg_text: &str)))?;

        m.add(py, "log_debug", py_fn!(py, log_debug(s: &str)))?;
        m.add(py, "log_info", py_fn!(py, log_info(s: &str)))?;
        m.add(py, "log_error", py_fn!(py, log_error(s: &str)))?;

        Ok(())
    });

fn initialize(_py: Python, verbose: bool, callback: PyObject) -> PyResult<bool> {
    setup_logging(3, verbose).unwrap_or_else(|err| {
        panic!("Logging init failed, reasone: {}", err);
    });
    if !callback.is_callable(_py) {
        return Ok(false);
    }

    unsafe {
        CALLBACK = Some(callback);
    }

    let (tx_to_engine, rx_out_engine) = engine_run(database::DbMode::Filesystem);
    unsafe {
        TX_TO_ENGINE = Some(tx_to_engine);
    }

    thread::spawn(move || {
        for cmd in rx_out_engine {
            let cmd_str = serde_json::to_string(&cmd).unwrap();
            info!("EXPECT: {}", &cmd_str);
            engine_callback(cmd_str);
        }
    });
    Ok(true)
}

fn stop(_py: Python) -> PyResult<bool> {
    unsafe {
        if let Err(error) = TX_TO_ENGINE.as_mut().unwrap().send(CmdToEngine::Terminate) {
            error!("Can't send stop signal to engine: {}", error);
        }
    }
    Ok(true)
}

fn add_user(
    _py: Python,
    uid: i64,
    username: &str,
    chat_id: i64,
    first_name: &str,
    last_name: &str,
    tz: i32,
) -> PyResult<bool> {
    unsafe {
        TX_TO_ENGINE
            .as_mut()
            .unwrap()
            .send(CmdToEngine::AddUser {
                uid,
                username: username.to_string(),
                chat_id,
                first_name: first_name.to_string(),
                last_name: last_name.to_string(),
                tz,
            })
            .unwrap();
    }
    Ok(true)
}

fn handle_text_message(_py: Python, uid: i64, msg_id: i64, message: &str) -> PyResult<bool> {
    unsafe {
        TX_TO_ENGINE
            .as_mut()
            .unwrap()
            .send(CmdToEngine::TextMessage {
                uid,
                msg_id,
                message: message.to_string(),
            })
            .unwrap();
    }
    Ok(true)
}

fn handle_keyboard_responce(
    _py: Python,
    uid: i64,
    msg_id: i64,
    call_data: &str,
    msg_text: &str,
) -> PyResult<bool> {
    unsafe {
        TX_TO_ENGINE
            .as_mut()
            .unwrap()
            .send(CmdToEngine::KeyboardMessage {
                uid,
                msg_id,
                call_data: call_data.to_string(),
                msg_text: msg_text.to_string(),
            })
            .unwrap();
    }
    Ok(true)
}

fn engine_callback(text: String) {
    // SAFETY: engine is single threaded, so cannot call CALLBACK from different threads.
    // Since GIL is locked for callback, client side should be OK too.
    unsafe {
        if CALLBACK.is_some() {
            let gil = Python::acquire_gil();
            let py = gil.python();
            let py_turple = PyTuple::new(py, &[text.to_py_object(py).into_object()]);
            let _res = CALLBACK.as_mut().unwrap().call(py, py_turple, None);
        }
    }
}

fn log_debug(_py: Python, s: &str) -> PyResult<bool> {
    debug!("[Frontend] {}", s);
    Ok(true)
}

fn log_info(_py: Python, s: &str) -> PyResult<bool> {
    info!("[Frontend] {}", s);
    Ok(true)
}

fn log_error(_py: Python, s: &str) -> PyResult<bool> {
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
                .level(log::LevelFilter::Info)
                .level_for("overly-verbose-target", log::LevelFilter::Warn)
        }
        1 => base_config
            .level(log::LevelFilter::Debug)
            .level_for("overly-verbose-target", log::LevelFilter::Info),
        2 => base_config.level(log::LevelFilter::Debug),
        _3_or_more => base_config.level(log::LevelFilter::Trace),
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
            if record.level() > log::LevelFilter::Info && record.target() == "cmd_program" {
                out.finish(format_args!(
                    "---\nDEBUG: {}: {}\n---",
                    chrono::Local::now().format("%H:%M:%S"),
                    message
                ))
            } else {
                out.finish(format_args!(
                    "[{}][{}] {}",
                    chrono::Local::now().format("%H:%M:%S"),
                    record.level(),
                    message
                ))
            }
        })
        .chain(io::stdout());

    if console_output_enabled {
        base_config
            .chain(file_config)
            .chain(stdout_config)
            .apply()?;
    } else {
        base_config.chain(file_config).apply()?;
    }

    Ok(())
}
