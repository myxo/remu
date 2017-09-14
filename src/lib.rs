#![feature(drop_types_in_const)]
extern crate chrono;
extern crate regex;
#[macro_use]
extern crate cpython;

mod command;
pub mod engine;
mod database;
// use command::Command;
use engine::Engine;

use cpython::{Python, PyResult};

use std::thread;

py_module_initializer!(libtelegram_rust_backend, 
        initlibtelegram_rust_backend, 
        PyInit_libtelegram_rust_backend, 
        |py, m| {
    m.add(py, "initialize", py_fn!(py, initialize()))?;
    m.add(py, "run", py_fn!(py, run()))?;
    m.add(py, "stop", py_fn!(py, stop()))?;
    m.add(py, "handle_text_message", py_fn!(py, handle_text_message(message : &str)))?;
    m.add(py, "check_for_message", py_fn!(py, check_for_message()))?;

    Ok(())
});

static mut ENG : Option<Engine> = None;

fn initialize(_py : Python) -> PyResult<(u64)>{
    unsafe {
        ENG = Some(Engine::new());
    }
    Ok((64))
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
