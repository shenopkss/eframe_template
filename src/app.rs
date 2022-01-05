use eframe::{egui, epi};
use toml::Value;
use std::fs;
use std::sync::Arc;
use crate::{Column, DataSource, Render, Table};
use glob::glob;
use tera::{Function, Tera};
use tera::Context;
use tera;

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
    tera: Arc<Tera>,
    datasources: Vec<DataSource>,
}

impl Default for TemplateApp {
    fn default() -> Self {
        let tera = match Tera::new("template/**/*.twig") {
            Err(e) => {
                println!("Parsing error(s): {}", e);
                panic!("Parsing error")
            }
            Ok(mut t) => {
                let mut t1 = Arc::new(t);
                let mut render = Render::new();
                t1.clone().register_function("render", render);
                t1
            }
        };
        Self {
            // Example stuff:
            label: "Hello World!".to_owned(),
            value: 0.0,
            datasources: Vec::new(),
            tera,
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

        let Self { label, value, datasources, tera } = self;


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
                            let t = config["table"].as_table().unwrap();
                            let columns = config["column"].as_array().unwrap();
                            let mut table = Table::default();
                            table.name = t["name"].as_str().unwrap().to_string();
                            table.comment = t["comment"].as_str().unwrap().to_string();
                            for c in columns {
                                let mut column = Column::default();
                                column.name = c["name"].as_str().unwrap().to_string();
                                column.db_type = c["type"].as_str().unwrap().to_string();
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
                                if let Some(comment) = c.get("comment") {
                                    column.comment = comment.as_str().unwrap().to_string();
                                }
                                if let Some(java_type) = c.get("java_type") {
                                    column.java_type = java_type.as_str().unwrap().to_string();
                                }
                                table.columns.push(column);
                            }
                            ds.tables.push(table)
                        }
                        Err(e) => println!("{:?}", e),
                    }
                }
                datasources.push(ds);
            }
        }
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
        let Self { label, value, datasources, tera } = self;

        // Examples of how to create different panels and windows.
        // Pick whichever suits you.
        // Tip: a good default choice is to just keep the `CentralPanel`.
        // For inspiration and more examples, go to https://emilk.github.io/egui

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            // The top panel is often a good place for a menu bar:
            egui::menu::bar(ui, |ui| {
                // egui::menu::menu(ui, "File", |ui| {
                //     if ui.button("Quit").clicked() {
                //         frame.quit();
                //     }
                // });

                if ui.button("Run").clicked() {
                    let mut context = Context::new();
                    context.insert("db", &datasources.get(0).unwrap());
                    let content = tera.render("springboot/test.twig", &context);
                    match content {
                        Ok(c) => println!("content {}", c),
                        Err(e) => println!("{:?}", e),
                    }
                }
            });
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

        egui::CentralPanel::default().show(ctx, |ui| {
            // The central panel the region left after adding TopPanel's and SidePanel's

            ui.heading("eframe projects");
            ui.hyperlink("https://github.com/emilk/eframe_template");
            ui.add(egui::github_link_file!(
                "https://github.com/emilk/eframe_template/blob/master/",
                "Source code."
            ));
            egui::warn_if_debug_build(ui);
        });
    }
}
