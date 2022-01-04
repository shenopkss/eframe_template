#![forbid(unsafe_code)]
#![cfg_attr(not(debug_assertions), deny(warnings))] // Forbid warnings in release builds
#![warn(clippy::all, rust_2018_idioms)]

mod app;

use std::collections::HashMap;
use std::iter::Map;
pub use app::TemplateApp;

// ----------------------------------------------------------------------------
// When compiling for web:

#[cfg(target_arch = "wasm32")]
use eframe::wasm_bindgen::{self, prelude::*};

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

#[derive(Default, Clone)]
pub struct DataSource {
    database: String,
    driver: String,
    host: String,
    port: u32,
    username: String,
    password: String,
    table: Vec<String>,
    namespace: String,
}

#[derive(Default, Clone)]
pub struct Table {
    name: String,
    comment: String,
    primary_key: String,
    columns: Vec<Column>,
    foreign_keys: Vec<Column>,
    no_foreign_keys: Vec<Column>,
    host: String,
    port: u32,
    username: String,
    password: String,
    table: Vec<String>,
    namespace: String,
}

#[derive(Default, Clone)]
pub struct Column {
    name: String,
    comment: String,
    table: Table,
    reference_table: Table,
    is_nullable: bool,
    is_unique: bool,
    db_type: String,
    java_type: String,
    size: String,
    export: bool,
    set: HashMap<String, String>,
}