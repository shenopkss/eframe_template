#![forbid(unsafe_code)]
#![cfg_attr(not(debug_assertions), deny(warnings))] // Forbid warnings in release builds
#![warn(clippy::all, rust_2018_idioms)]

mod app;

use std::borrow::BorrowMut;
use std::cell::RefCell;
use std::collections::{HashMap};
use std::convert::TryInto;
use std::fs::File;
use std::io::Write;
use std::iter::Map;
use std::ops::Deref;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use eframe::egui::Key::T;
pub use app::TemplateApp;

extern crate glob;
extern crate tera;

use serde;

// ----------------------------------------------------------------------------
// When compiling for web:

#[cfg(target_arch = "wasm32")]
use eframe::wasm_bindgen::{self, prelude::*};
use tera::{Context, Error, from_value, Function, Tera, to_value, try_get_value, Value};

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


struct MyRender {
    tera: Tera,
}

impl MyRender {
    fn new(path: &str) -> Self {
        let mut tera = MyRender::register_tera(path);
        let mut tera2 = MyRender::register_tera(path);

        tera.register_function("render", render(tera2));
        let mut render = MyRender {
            tera
        };

        render.setup();
        render
    }

    fn setup(&mut self) {}

    pub fn generate(&mut self, template_name: &str, context: &Context) -> tera::Result<String> {
        self.tera.render(template_name, context)
    }

    fn register_tera(path: &str) -> Tera {
        let mut tera = Tera::new(&path).unwrap();
        tera.register_filter("camel", camel);
        tera.register_filter("pascal", pascal);
        tera
    }
}

fn render(tera: Tera) -> impl Function {
    Box::new(move |args: &HashMap<String, Value>| -> Result<Value, Error> {
        let template = match args.get("template") {
            Some(template) => match from_value::<String>(template.clone()) {
                Ok(v) => v,
                Err(_) => {
                    return Err(Error::msg(format!(
                        "Function `render` received template={} but `template` can only be a String",
                        template
                    )));
                }
            },
            None => return Err(Error::msg(format!(
                "Function `render` received template=None but `template` can only be a String"
            ))),
        };
        let data = match args.get("data") {
            Some(data) => match from_value::<Value>(data.clone()) {
                Ok(v) => v,
                Err(_) => {
                    return Err(Error::msg(format!(
                        "Function `render` received data={} but `data` can only be a Value",
                        data
                    )));
                }
            },
            None => return Err(Error::msg(format!(
                "Function `render` received data=None but `data` can only be a Value",
            )))
        };

        let output = match args.get("output") {
            Some(output) => match from_value::<String>(output.clone()) {
                Ok(v) => v,
                Err(_) => {
                    return Err(Error::msg(format!(
                        "Function `render` received output={} but `output` can only be a String",
                        output
                    )));
                }
            },
            None => return Err(Error::msg(format!(
                "Function `render` received output=None but `output` can only be a String",
            ))),
        };

        println!("render template: {:?}", template);
        println!("render data: {:?}", data);
        println!("render output: {:?}", output);

        let mut context = Context::new();
        context.insert("data", &data);

        let content = tera.render(&template, &context).unwrap();
        println!("render content: {}", content);
        let mut file = File::create(output.clone())?;
        file.write_all(&content.as_bytes());

        Ok(to_value(format!("render file: {}", output)).unwrap())
    })
}

pub fn camel(value: &Value, _: &HashMap<String, Value>) -> Result<Value, Error> {
    let s = try_get_value!("camel", "value", String, value);
    let mut result = String::new();
    for item in s.split("_") {
        if result.len() == 0 {
            for c in item.chars().enumerate().map(|(i, v)| { if i == 0 { v.to_ascii_lowercase() } else { v } }) {
                result.push(c);
            }
        } else {
            for c in item.chars().enumerate().map(|(i, v)| { if i == 0 { v.to_ascii_uppercase() } else { v } }) {
                result.push(c);
            }
        }
    }

    Ok(to_value(result).unwrap())
}

pub fn pascal(value: &Value, _: &HashMap<String, Value>) -> Result<Value, Error> {
    let s = try_get_value!("pascal", "value", String, value);
    let mut result = String::new();
    for item in s.split("_") {
        for c in item.chars().enumerate().map(|(i, v)| { if i == 0 { v.to_ascii_uppercase() } else { v } }) {
            result.push(c);
        }
    }

    Ok(to_value(result).unwrap())
}