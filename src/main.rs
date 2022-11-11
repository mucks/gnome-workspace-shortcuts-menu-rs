use anyhow::Result;
use std::{
    collections::{BTreeMap, HashMap},
    process::Command,
};

use eframe::{
    egui::{self, TextEdit, Ui},
    epaint::Vec2,
};

fn main() {
    // Log to stdout (if you run with `RUST_LOG=debug`).
    tracing_subscriber::fmt::init();

    let options = eframe::NativeOptions {
        initial_window_size: Some(Vec2::new(1280.0, 720.0)),
        ..Default::default()
    };

    eframe::run_native(
        "Gnome Workspace Shortcuts Menu",
        options,
        Box::new(|_cc| Box::new(MyApp::new())),
    );
}

#[derive(Debug, Clone)]
struct WorkspaceKeybinding {
    pub modifier: String,
    pub modifier_index: usize,
    pub gsettings_key: String,
    pub gsettings_value: String,
    pub label: String,
    pub keybinding: String,
    pub converted_keybinding: String,
}

struct MyApp {
    modifier_vec: Vec<Modifier>,
    workspace_keybinding_map: BTreeMap<usize, WorkspaceKeybinding>,
    keysyms: HashMap<String, String>,
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            modifier_vec: get_vec(),
            workspace_keybinding_map: BTreeMap::new(),
            keysyms: HashMap::new(),
        }
    }
}

#[derive(Debug, PartialEq)]
struct Modifier {
    pub name: String,
    pub gsettings_value: String,
}

impl Modifier {
    pub fn new(name: &str, gsettings_value: &str) -> Self {
        Self {
            name: name.into(),
            gsettings_value: gsettings_value.into(),
        }
    }
}

fn get_vec() -> Vec<Modifier> {
    vec![
        Modifier::new("NONE", ""),
        Modifier::new("ALT", "<Alt>"),
        Modifier::new("CTRL", "<Ctrl>"),
        Modifier::new("SUPER", "<Super>"),
        Modifier::new("SHIFT", "<Shift>"),
        Modifier::new("SHIFT+SUPER", "<Shift><Super>"),
    ]
}

fn get_gsettings_value_from_key(gsettings_key: &str) -> Result<String> {
    Ok(String::from_utf8(
        Command::new("gsettings")
            .arg("get")
            .arg("org.gnome.desktop.wm.keybindings")
            .arg(gsettings_key)
            .output()?
            .stdout,
    )?)
}

impl MyApp {
    fn new() -> Self {
        let mut app = Self::default();
        app.gen_workspace_keybinding_map();
        app.get_gsettings_values_from_config();
        app.init_keysyms();
        println!("{:#?}", app.keysyms);
        app
    }

    fn init_keysyms(&mut self) {
        let keys: &str = include_str!("../test.txt");

        let lines: Vec<&str> = keys.split('\n').collect();

        for line in lines {
            let s: Vec<&str> = line.split_whitespace().collect();
            if s.len() >= 3 {
                self.keysyms.insert(s[2].into(), s[0].into());
            }
        }
    }

    fn gen_workspace_keybinding_map(&mut self) {
        let workspace_count = 10;
        for i in 0..workspace_count {
            self.workspace_keybinding_map.insert(
                i,
                WorkspaceKeybinding {
                    modifier: "NONE".into(),
                    modifier_index: 0,
                    gsettings_key: format!("switch-to-workspace-{}", i + 1),
                    gsettings_value: "".into(),
                    label: format!("Switch to workspace {}", i + 1),
                    keybinding: "".into(),
                    converted_keybinding: "".into(),
                },
            );
        }
        for i in 0..workspace_count {
            self.workspace_keybinding_map.insert(
                i + workspace_count,
                WorkspaceKeybinding {
                    modifier: "NONE".into(),
                    modifier_index: 0,
                    gsettings_key: format!("move-to-workspace-{}", i + 1),
                    gsettings_value: "".into(),
                    label: format!("Move window to workspace {}", i + 1),
                    keybinding: "".into(),
                    converted_keybinding: "".into(),
                },
            );
        }
    }

    fn get_gsettings_values_from_config(&mut self) -> Result<()> {
        for (k, v) in &mut self.workspace_keybinding_map {
            v.gsettings_value = get_gsettings_value_from_key(&v.gsettings_key)?;
        }
        Ok(())
    }
    fn workspace_keybinding_input(&mut self, ui: &mut Ui, k: usize) {
        ui.horizontal(|ui| {
            let selection = &mut self.workspace_keybinding_map.get_mut(&k).unwrap();

            ui.label(&selection.label);

            egui::ComboBox::from_id_source(k)
                .selected_text(self.modifier_vec[selection.modifier_index].name.to_string())
                .show_ui(ui, |ui| {
                    for i in 0..self.modifier_vec.len() {
                        let value = ui.selectable_value(
                            &mut &self.modifier_vec[i],
                            &self.modifier_vec[selection.modifier_index],
                            &self.modifier_vec[i].name,
                        );
                        if value.clicked() {
                            selection.modifier = self.modifier_vec[i].name.to_owned();
                            selection.modifier_index = i;
                        }
                    }
                });

            let te = TextEdit::singleline(&mut selection.keybinding);
            ui.add_sized(Vec2::new(40.0, 20.0), te);

            // make sure it's only 1 key
            if selection.keybinding.len() > 1 {
                selection.keybinding =
                    selection.keybinding.chars().collect::<Vec<char>>()[0].into();
            }

            let keybind = match self.keysyms.get(&selection.keybinding) {
                Some(keysym) => keysym.to_string(),
                None => selection.keybinding.to_string(),
            };

            selection.converted_keybinding = format!(
                "['{}{}']",
                self.modifier_vec[selection.modifier_index].gsettings_value, keybind
            );

            let converted_te =
                TextEdit::singleline(&mut selection.converted_keybinding).interactive(false);
            ui.add_sized(Vec2::new(300.0, 20.0), converted_te);

            let te3 = TextEdit::singleline(&mut selection.gsettings_value).interactive(false);
            ui.add_sized(Vec2::new(300.0, 20.0), te3);
        });
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            // let events = ui.input().events.clone();
            // for event in events {
            //     // println!("{:?}", event);
            // }

            ui.heading("Shortcuts");
            for (k, _) in self.workspace_keybinding_map.clone() {
                self.workspace_keybinding_input(ui, k);
            }
        });
    }
}
