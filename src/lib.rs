#![forbid(unsafe_code)]
#![cfg_attr(not(debug_assertions), deny(warnings))] // Forbid warnings in release builds
#![warn(clippy::all, rust_2018_idioms)]

mod app;

use std::collections::{HashMap};
use std::convert::TryInto;
use std::iter::Map;
use std::sync::Arc;
use eframe::egui::Key::T;
pub use app::TemplateApp;

extern crate glob;
extern crate tera;

use serde;

// ----------------------------------------------------------------------------
// When compiling for web:

#[cfg(target_arch = "wasm32")]
use eframe::wasm_bindgen::{self, prelude::*};
use tera::{Context, Error, from_value, Function, Tera, to_value, Value};

/// This is the entry-point for all the web-assembly.
/// This is called once from the HTML.
/// It loads the app, installs some callbacks, then returns.
/// You can add more callbacks like this if you want to call in to your code.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn start(canvas_id: &str) -> Result<(), eframe::wasm_bindgen::JsValue> {
    let app = TemplateApp::default();
    eframe::start_web(canvas_id, Box::new(app))
}

#[derive(Default, Clone, serde::Deserialize, serde::Serialize)]
pub struct DataSource {
    database: String,
    driver: String,
    host: String,
    port: u32,
    username: String,
    password: String,
    tables: Vec<Table>,
    namespace: String,
}

#[derive(Default, Clone, serde::Deserialize, serde::Serialize)]
pub struct Table {
    name: String,
    comment: String,
    primary_key: String,
    foreign_keys: Vec<Column>,
    no_foreign_keys: Vec<Column>,
    columns: Vec<Column>,
}

#[derive(Default, Clone, serde::Deserialize, serde::Serialize)]
pub struct Column {
    name: String,
    comment: String,
    table: Table,
    reference_table: Table,
    not_null: bool,
    unique: bool,
    auto_increment: bool,
    primary_key: bool,
    db_type: String,
    java_type: String,
    length: u32,
    export: bool,
    set: HashMap<String, String>,
}

struct Render {
    tera: Option<Tera>,
}

impl Render {
    fn new() -> Render {
        Render {
            tera: None
        }
    }

    fn init(&mut self, t: Tera) {
        self.tera = Some(t);
    }
}

impl Function for Render {
    fn call(&self, args: &HashMap<String, Value>) -> tera::Result<Value> {
        match self.tera {
            Some(t) => {
                match args.get("path") {
                    Some(path) => {
                        let mut context = Context::new();
                        let data = args.get("data").unwrap().as_object().unwrap();
                        context.insert("data", data);
                        println!("Render data {:?} {:?}", path, data);
                        t.render(&path.as_str().unwrap(), &context);
                        Ok(Value::Null)
                    }
                    _ => {
                        Err(Error::msg("oops"))
                    }
                }
            }
            _ => Err(Error::msg("oops"))
        }
    }
}

// struct MyRender {
//     base_path: String,
//     tera: Tera,
// }
//
// impl MyRender {
//     fn new(path: String) -> Self {
//         let mut render = MyRender {
//             base_path: path,
//             tera: Tera::new(&path.clone()).unwrap(),
//         };
//
//         render.register();
//         render
//     }
//
//     fn register(&mut self) {
//         self.tera.register_function("my_render", self);
//     }
//
//     fn register_func(&mut self, func: Box<dyn Function>) {
//         self.
//     }
//
//
// }