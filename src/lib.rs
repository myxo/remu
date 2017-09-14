#![feature(drop_types_in_const)]
// link function to python

extern crate chrono;
extern crate regex;

mod command;
pub mod engine;
mod database;
use command::Command;
use engine::Engine;

#[macro_use]
extern crate cpython;
#[macro_use]
extern crate lazy_static;

use cpython::{Python, PyResult};
use database::DataBase;

use std::thread;




py_module_initializer!(libtelegram_rust_backend, 
        initlibtelegram_rust_backend, 
        PyInit_libtelegram_rust_backend, 
        |py, m| {
    m.add(py, "engine_run", py_fn!(py, engine_run()))?;
    m.add(py, "engine_stop", py_fn!(py, engine_stop()))?;

    Ok(())
});

// static mut engine : Option<Engine> =  None;
// static mut engine_ref: &'static Engine;
// lazy_static! {
//     static ref ENG : Engine = Engine::new();
// }

// static mut ENG : *mut Engine = 0 as *mut Engine;

static mut ENG : Option<Engine> = None;

// fn get_engine() -> &'static Engine {
//     unsafe {
//         match ENG {
//             Some(ref e) => e,
//             None => panic!("Initialize engine first!"),
//         }
//     }
// }

fn engine_run(_py : Python) -> PyResult<(u64)>{
    unsafe {
        // ENG = &mut Engine::new();
        // engine_ref = &engine.unwrap()
        ENG = Some(Engine::new());
    }
    let handle = thread::spawn( ||{
        unsafe{
            // let mut en = unsafe{ &*ENG };
            // en.run();
            
            ENG.as_mut().unwrap().run();
        }
    });
    // engine.run();

    Ok((64))
}

fn engine_stop(_py : Python) -> PyResult<(u64)>{
    unsafe {
        ENG.as_mut().unwrap().stop();
    }

    Ok((64))
}

// fn engine_stop