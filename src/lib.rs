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

use engine::Engine;

use cpython::{Python, PyResult};
use std::thread;
use std::io;

static mut ENG : Option<Engine> = None;

py_module_initializer!(libtelegram_rust_backend, 
    initlibtelegram_rust_backend, 
    PyInit_libtelegram_rust_backend, 
    |py, m| 
    {
        m.add(py, "initialize", py_fn!(py, initialize(verbose: bool)))?;
        m.add(py, "run", py_fn!(py, run()))?;
        m.add(py, "stop", py_fn!(py, stop()))?;
        m.add(py, "handle_text_message", py_fn!(py, handle_text_message(message: &str)))?;
        m.add(py, "check_for_message", py_fn!(py, check_for_message()))?;

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

fn handle_text_message(_py : Python, message : &str) -> PyResult<String>{
    let out;
    unsafe{
        out = ENG.as_mut().expect("initialize engine!").handle_text_message(message);
    }
    Ok(out)
}

fn check_for_message(_py : Python) -> PyResult<String>{
    let out;
    unsafe{
        out = ENG.as_mut().expect("initialize engine!").check_for_message();
    }
    Ok(out)
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
                                        chrono::Local::now().format("%H:%M"),
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
