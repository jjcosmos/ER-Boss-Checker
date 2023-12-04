#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use eframe::egui;
use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::io::Read;
use std::path::Path;
use std::{collections::HashSet, io::Write};

static CONFIG_PATH: &str = "config.json";

fn main() -> Result<(), eframe::Error> {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([600.0, 500.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Boss Checker",
        options,
        Box::new(|_cc| {
            Box::<MyApp>::default()
        }),
    )
}

fn filter_entries(entries: &mut Vec<TableEntry>, search_region: &String, search_boss: &String) {
    let term = search_boss.to_lowercase();
    for entry in entries.iter_mut() {
        // TODO: replace with fuzzy search
        if search_region == "All" || entry.region == search_region.as_str() {
            entry.visible = true;
        }
        else {
            entry.visible = false;
        }

        if entry.name.to_lowercase().contains(&term) || entry.region.to_lowercase().contains(&term)
        {
            entry.visible &= true;
        } else {
            entry.visible &= false;
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct RegionEntry {
    region: String,
    bosses: Vec<String>,
}

fn load_tables_from_file(file_path: String, state: &State) -> Vec<TableEntry> {
    let mut file =
        File::open(&file_path).expect(format!("Failed to load table from {}", &file_path).as_str());
    let mut contents = String::new();
    file.read_to_string(&mut contents).unwrap();
    let boss_data: Vec<RegionEntry> = serde_json::from_str(&contents).unwrap();

    let mut tables: Vec<TableEntry> = Vec::new();
    for data in boss_data {
        for boss in data.bosses {
            let table_entry = TableEntry {
                region: data.region.clone(),
                name: boss.clone(),
                visible: true,
                checked: state
                    .completed
                    .contains(&(data.region.clone(), boss.clone())),
            };
            tables.push(table_entry)
        }
    }

    tables
}

fn extract_regions(entries: &Vec<TableEntry>) -> Vec<String> {
    let mut set: HashSet<String> = HashSet::new();
    set.insert("All".to_owned());
    for entry in entries.iter() {
        if !set.contains(&entry.region) {
            set.insert(entry.region.clone());
        }
    }

    let mut vec: Vec<String> = set.into_iter().collect();
    vec.sort();
    vec
}

struct TableEntry {
    region: String,
    name: String,
    checked: bool,
    visible: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Config {
    checklist_path: String,
    default_save: String,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            checklist_path: "boss_data.json".to_string(),
            default_save: "default_save.json".to_string(),
        }
    }
}

impl Config {
    fn make_or_load_from_file() -> Self {
        let file_path = Path::new(CONFIG_PATH);
        let exists = file_path.exists();
        let mut file: std::fs::File;

        if !exists {
            file = File::create(CONFIG_PATH).unwrap();

            let created = Config::default();
            let buf = serde_json::to_string(&created).unwrap();
            match file.write_all(buf.as_bytes()) {
                Ok(_) => {}
                Err(e) => {
                    eprint!("{}", e)
                }
            }
        }

        file = OpenOptions::new().read(true).open(CONFIG_PATH).unwrap();
        let mut data = String::new();
        file.read_to_string(&mut data).unwrap();
        let config: Config = serde_json::from_str(&data).unwrap();

        config
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct State {
    completed: HashSet<(String, String)>,
}

impl State {
    fn make_or_load_from_file() -> Self {
        let config = Config::make_or_load_from_file();
        let save_file_name = config.default_save;

        let file_path = Path::new(&save_file_name);
        let exists = file_path.exists();
        let mut file: std::fs::File;

        if !exists {
            file = File::create(&save_file_name).unwrap();

            let created = State::default();
            let buf = serde_json::to_string(&created).unwrap();
            match file.write_all(buf.as_bytes()) {
                Ok(_) => {}
                Err(e) => {
                    eprint!("{}", e)
                }
            }
        }

        file = OpenOptions::new().read(true).open(save_file_name).unwrap();
        let mut data = String::new();
        file.read_to_string(&mut data).unwrap();
        let state: State = serde_json::from_str(&data).unwrap();

        state
    }
}

impl Default for State {
    fn default() -> Self {
        State {
            completed: HashSet::new(),
        }
    }
}

struct MyApp {
    region_filter: String,
    boss_filter: String,
    config: Config,
    entries: Vec<TableEntry>,
    filter_regions: Vec<String>
}

impl MyApp {
    fn save_to_disk(&mut self) {
        let mut hash_set: HashSet<(String,String)> = HashSet::new();
        for entry in self.entries.iter() {
            if entry.checked {
                hash_set.insert((entry.region.clone(), entry.name.clone()));
            }
        }

        let path = Path::new(&self.config.default_save);
        let mut file = File::options().write(true).truncate(true).open(path).unwrap();
        let save_state = State{completed: hash_set};
        let serialized = serde_json::to_string(&save_state).unwrap();

        file.write_all(serialized.as_bytes()).expect("Failed to write save file");
    }
}

impl Default for MyApp {
    fn default() -> Self {
        let loaded_config = Config::make_or_load_from_file();
        let loaded_state = State::make_or_load_from_file();
        let loaded_data =
            load_tables_from_file(loaded_config.checklist_path.clone(), &loaded_state);

        Self {
            boss_filter: "".to_owned(),
            region_filter: "All".to_owned(),
            config: loaded_config,
            filter_regions: extract_regions(&loaded_data),
            entries: loaded_data,
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        filter_entries(&mut self.entries, &self.region_filter,&self.boss_filter);
        let mut dirty: bool = false;

        egui::CentralPanel::default().show(ctx, |ui| {
            //ui.heading("Boss Picker");
            ui.horizontal(|ui| {
                egui::ComboBox::new("Combo", "").width(200.0).selected_text(self.region_filter.to_string()).show_ui(ui, |ui|{
                    for region in self.filter_regions.iter_mut() {
                        ui.selectable_value(&mut self.region_filter, region.to_string(), region.to_string());
                    }
                });

                let name_label = ui.label("Boss:");
                ui.text_edit_singleline(&mut self.boss_filter)
                    .labelled_by(name_label.id);
            });
            

            egui::ScrollArea::vertical().show(ui, |ui| {
                egui::Grid::new("Grid").min_col_width(150.0).striped(true).show(ui, |ui| {
                    for entry in self.entries.iter_mut() {
                        if !entry.visible {
                            continue;
                        }
    
                        ui.label(&entry.region);
                        ui.label(&entry.name);
    
                        if ui.checkbox(&mut entry.checked, "Completed").changed() {
                            dirty = true;
                        }
    
                        ui.end_row();
                    }
                });
            });
            
        });

        if dirty {
            self.save_to_disk();
        }
    }
}
