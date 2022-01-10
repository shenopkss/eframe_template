use eframe::{egui, epi};
use toml::Value;
use std::fs;
use std::vec::Vec;
use crate::{Column, DataSource, MyRender, Table};
use glob::glob;
use tera::{Function, Tera};
use tera::Context;
use tera;
use std::sync::mpsc;
use crossbeam_channel::{Receiver, Sender, unbounded};
use mysql::*;
use mysql::prelude::*;
use chrono::prelude::*; // 用来处理日期

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "persistence", serde(default))] // if we add new fields, give them default values when deserializing old state
pub struct TemplateApp {
    // Example stuff:
    label: String,

    // this how you opt-out of serialization of a member
    #[cfg_attr(feature = "persistence", serde(skip))]
    value: f32,
    #[cfg_attr(feature = "persistence", serde(skip))]
    render: MyRender,
    datasources: Vec<DataSource>,
    #[cfg_attr(feature = "persistence", serde(skip))]
    output_event_receiver: Receiver<String>,
    #[cfg_attr(feature = "persistence", serde(skip))]
    output_event_sender: Sender<String>,
    output_events: Vec<String>,
}

impl Default for TemplateApp {
    fn default() -> Self {
        let (sender, receiver) = unbounded::<String>();
        Self {
            // Example stuff:
            label: "Hello World!".to_owned(),
            value: 0.0,
            datasources: Vec::new(),
            render: MyRender::new("/Users/shan/Projects/tools/eframe_template/template/**/*.twig", sender.clone()),
            output_event_receiver: receiver,
            output_event_sender: sender,
            output_events: Vec::new(),
        }
    }
}

impl epi::App for TemplateApp {
    fn name(&self) -> &str {
        "eframe projects"
    }

    /// Called once before the first frame.
    fn setup(
        &mut self,
        _ctx: &egui::CtxRef,
        _frame: &mut epi::Frame<'_>,
        _storage: Option<&dyn epi::Storage>,
    ) {
        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        // #[cfg(feature = "persistence")]
        // if let Some(storage) = _storage {
        //     *self = epi::get_value(storage, epi::APP_KEY).unwrap_or_default()
        // }

        let Self { label, value, datasources, render, output_event_receiver, output_event_sender, output_events } = self;

        let paths = fs::read_dir("/Users/shan/Projects/tools/eframe_template/schema").unwrap();
        for path in paths {
            if let Ok(path) = path {
                let filename = format!("{}/config.toml", path.path().display());
                let contents = fs::read_to_string(filename).expect("Something went wrong reading the file");
                let config = contents.parse::<Value>().unwrap();
                let datasource = config["datasource"].as_table().unwrap();
                let mut ds = DataSource::default();
                ds.database = datasource["database"].as_str().unwrap().to_string();
                ds.driver = datasource["driver"].as_str().unwrap().to_string();
                ds.host = datasource["host"].as_str().unwrap().to_string();
                ds.port = datasource["port"].as_integer().unwrap() as u32;
                ds.username = datasource["username"].as_str().unwrap().to_string();
                ds.password = datasource["password"].as_str().unwrap().to_string();

                let table_file = format!("{}/table/*.toml", path.path().display());
                for entry in glob(&table_file).expect("Failed to read glob pattern") {
                    match entry {
                        Ok(path) => {
                            let filename = format!("{}", path.display());
                            let contents = fs::read_to_string(filename).unwrap();
                            let config = contents.parse::<Value>().unwrap();
                            let table_cfg = config["table"].as_table().unwrap();
                            let columns = config["column"].as_array().unwrap();
                            let mut table = Table::default();
                            table.name = table_cfg["name"].as_str().unwrap().to_string();
                            table.comment = table_cfg["comment"].as_str().unwrap().to_string();

                            if let Some(ref_tables) = table_cfg.get("ref_tables") {
                                for item in ref_tables.as_array().unwrap() {
                                    let mut ref_table = Table::default();
                                    ref_table.name = item.as_str().unwrap().to_string();
                                    table.ref_tables.push(ref_table);
                                }
                            }

                            for c in columns {
                                let mut column = Column::default();
                                column.name = c["name"].as_str().unwrap().to_string();
                                column.db_type = c["db_type"].as_str().unwrap().to_string();
                                match column.db_type.as_str() {
                                    "tinyint" | "smallint" | "mediumint" | "int" => {
                                        column.java_type = String::from("Integer");
                                    }
                                    "bigint" => {
                                        column.java_type = String::from("Long");
                                    }
                                    "float" => {
                                        column.java_type = String::from("Float");
                                    }
                                    "double" => {
                                        column.java_type = String::from("Double");
                                    }
                                    "decimal" => {
                                        column.java_type = String::from("java.math.BigDecimal");
                                    }
                                    "json" => {
                                        column.java_type = String::from("JSONArray");
                                    }
                                    "date" | "datetime" | "timestamp" => {
                                        column.java_type = String::from("LocalDateTime");
                                    }
                                    _ => {
                                        column.java_type = String::from("String");
                                    }
                                }
                                if let Some(primary_key) = c.get("primary_key") {
                                    column.primary_key = primary_key.as_bool().unwrap();
                                    if column.primary_key {
                                        table.primary_key = column.name.clone();
                                    }
                                }
                                if let Some(length) = c.get("length") {
                                    column.length = length.as_integer().unwrap() as u32;
                                }
                                if let Some(not_null) = c.get("not_null") {
                                    column.not_null = not_null.as_bool().unwrap();
                                }
                                if let Some(auto_increment) = c.get("auto_increment") {
                                    column.auto_increment = auto_increment.as_bool().unwrap();
                                }
                                if let Some(export) = c.get("export") {
                                    column.export = export.as_bool().unwrap();
                                }
                                if let Some(set) = c.get("set") {
                                    let set = set.as_array().unwrap();
                                    let keys = set.get(0).unwrap().as_array().unwrap();
                                    let values = set.get(1).unwrap().as_array().unwrap();
                                    for i in 0..keys.len() {
                                        column.set.insert(keys.get(i).unwrap().as_integer().unwrap(), values.get(i).unwrap().as_str().unwrap().to_string());
                                    }
                                }
                                if let Some(comment) = c.get("comment") {
                                    column.comment = comment.as_str().unwrap().to_string();
                                }
                                if let Some(java_type) = c.get("java_type") {
                                    column.java_type = java_type.as_str().unwrap().to_string();
                                }
                                if let Some(unique) = c.get("unique") {
                                    column.unique = unique.as_bool().unwrap();
                                }

                                if let Some(ref_table_name) = c.get("ref_table") {
                                    let mut ref_table = Table::default();
                                    ref_table.name = ref_table_name.as_str().unwrap().to_string();
                                    column.ref_table = ref_table;

                                    table.foreign_keys.push(column.clone())
                                }

                                table.columns.push(column);
                            }
                            ds.tables.push(table)
                        }
                        Err(e) => println!("{:?}", e),
                    }
                }

                let tables = ds.tables.clone();

                for table in ds.tables.iter_mut() {
                    for column in table.columns.iter_mut() {
                        if let Some(ref_table) = tables.iter().find(|&item| { item.name == column.ref_table.name }) {
                            column.ref_table = ref_table.clone()
                        }
                    }
                }

                for mut table in ds.tables.iter_mut() {
                    for mut ref_table in table.ref_tables.iter_mut() {
                        if let Some(t) = tables.iter().find(|&item| { item.name == ref_table.name }) {
                            ref_table = &mut t.clone();
                        }
                    }
                }

                output_event_sender.send(format!("Load schema {} success!", ds.database));
                datasources.push(ds);
            }
        }

        // //TODO 测试
        // let mut context = Context::new();
        // context.insert("db", &datasources.get(0).unwrap());
        // let content = render.generate("springboot/start.twig", &context);
        // match content {
        //     Ok(c) => {
        //         println!("code generate:{}", c);
        //         output_event_sender.send(String::from("generate success!"));
        //     }
        //     Err(e) => println!("{:?}", e),
        // }
        // //TODO 测试 end
    }

    /// Called by the frame work to save state before shutdown.
    /// Note that you must enable the `persistence` feature for this to work.
    #[cfg(feature = "persistence")]
    fn save(&mut self, storage: &mut dyn epi::Storage) {
        epi::set_value(storage, epi::APP_KEY, self);
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    /// Put your widgets into a `SidePanel`, `TopPanel`, `CentralPanel`, `Window` or `Area`.
    fn update(&mut self, ctx: &egui::CtxRef, frame: &mut epi::Frame<'_>) {
        let Self { label, value, datasources, render, output_event_receiver, output_event_sender, output_events } = self;

        // Examples of how to create different panels and windows.
        // Pick whichever suits you.
        // Tip: a good default choice is to just keep the `CentralPanel`.
        // For inspiration and more examples, go to https://emilk.github.io/egui

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.add_space(10.0);

            // The top panel is often a good place for a menu bar:
            egui::menu::bar(ui, |ui| {
                // egui::menu::menu(ui, "File", |ui| {
                //     if ui.button("Quit").clicked() {
                //         frame.quit();
                //     }
                // });


                if ui.button("Code Generate").clicked() {
                    let mut context = Context::new();
                    context.insert("db", &datasources.get(0).unwrap());
                    let content = render.generate("springboot/start.twig", &context);
                    match content {
                        Ok(c) => {
                            output_event_sender.send(String::from("generate success!"));
                        }
                        Err(e) => println!("{:?}", e),
                    }
                }

                if ui.button("Database Sync").clicked() {
                    let DataSource { username, password, host, port, database, tables, .. } = datasources.get(0).unwrap();
                    let url = format!("mysql://{username}:{password}@{host}:{port}/{database}", username = username, password = password, host = host, port = port, database = database);
                    let pool = Pool::new(Opts::from_url(&url[..]).unwrap()).unwrap(); // 获取连接池
                    let mut conn = pool.get_conn().unwrap();// 获取链接

                    // for table in tables.iter() {
                    //     let row = conn.query_iter("show create table bulletin");
                    //     println!("row {:?}", row.);
                    // }
                }
            });
            ui.add_space(5.0);
        });

        egui::SidePanel::left("side_panel").show(ctx, |ui| {
            ui.add_space(5.0);
            ui.heading("Schema Explorer");
            ui.add_space(5.0);

            for datasource in datasources {
                let datasource = datasource.clone();
                ui.collapsing(datasource.database.clone(), |ui| {
                    for table in datasource.tables {
                        ui.collapsing(table.name.clone(), |ui| {
                            for column in table.columns {
                                ui.label(column.name.clone());
                            }
                        });
                    }
                });
            }

            // ui.horizontal(|ui| {
            //     ui.label("Write something: ");
            //     ui.text_edit_singleline(label);
            // });
            //
            // ui.add(egui::Slider::new(value, 0.0..=10.0).text("value"));
            // if ui.button("Increment").clicked() {
            //     *value += 1.0;
            // }
            //
            // ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
            //     ui.horizontal(|ui| {
            //         ui.spacing_mut().item_spacing.x = 0.0;
            //         ui.label("powered by ");
            //         ui.hyperlink_to("egui", "https://github.com/emilk/egui");
            //         ui.label(" and ");
            //         ui.hyperlink_to("eframe", "https://github.com/emilk/egui/tree/master/eframe");
            //     });
            // });
        });
        egui::SidePanel::right("side_panel_template").show(ctx, |ui| {
            ui.add_space(5.0);
            ui.heading("Template Explorer");

            ui.add_space(5.0);

            for name in render.get_template_names() {
                ui.collapsing(name.clone(), |ui| {
                    // for table in datasource.tables {
                    //     ui.collapsing(table.name.clone(), |ui| {
                    //         for column in table.columns {
                    //             ui.label(column.name.clone());
                    //         }
                    //     });
                    // }
                });
            }
        });

        egui::CentralPanel::default().show(ctx, |ui| {

            // ui.horizontal(|ui| {
            //     ui.label("代码生成路径:");
            //     ui.add(egui::TextEdit::singleline(&mut code_path).desired_width(120.0));
            // });

            // The central panel the region left after adding TopPanel's and SidePanel's

            // ui.heading("eframe projects");
            // ui.hyperlink("https://github.com/emilk/eframe_template");
            // ui.add(egui::github_link_file!(
            //     "https://github.com/emilk/eframe_template/blob/master/",
            //     "Source code."
            // ));
            // egui::warn_if_debug_build(ui);
        });

        output_event_receiver.try_iter().for_each(|event| {
            output_events.push(event);
        });

        egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            ui.set_min_height(200.0);
            ui.add_space(5.0);
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 50.0;
                ui.heading("Event Log");
                if ui.button("clear all").clicked() {
                    output_events.clear();
                }
            });
            ui.add_space(5.0);
            // ui.separator();

            egui::ScrollArea::vertical()
                .stick_to_bottom()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    for event in output_events {
                        ui.label(event);
                    }
                });
        });
    }
}
