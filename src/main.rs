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
    key_to_keysym: HashMap<String, String>,
    keysym_to_key: HashMap<String, String>,
    num_of_workspaces: String,
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            modifier_vec: get_vec(),
            workspace_keybinding_map: BTreeMap::new(),
            key_to_keysym: HashMap::new(),
            keysym_to_key: HashMap::new(),
            num_of_workspaces: "4".into(),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
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

const EMPTY_KEYBINDING: &str = "[\"\"]";

struct GSettings;

impl GSettings {
    // id is 1-9

    fn disable_switch_to_application_shortcuts() -> Result<()> {
        for i in 1..10 {
            Self::set_switch_to_application_keybinding(i, EMPTY_KEYBINDING)?;
        }
        Ok(())
    }

    fn set_switch_to_application_keybinding(id: u32, gsettings_value: &str) -> Result<()> {
        let _ = Command::new("gsettings")
            .arg("set")
            .arg("org.gnome.shell.keybindings")
            .arg(format!("switch-to-application-{id}"))
            .arg(gsettings_value)
            .output()?
            .stdout;
        Ok(())
    }

    fn set_number_of_workspaces(num: usize) -> Result<()> {
        let _ = Command::new("gsettings")
            .arg("set")
            .arg("org.gnome.desktop.wm.preferences")
            .arg("num-workspaces")
            .arg(num.to_string())
            .output()?
            .stdout;
        Ok(())
    }
    fn get_number_of_workspaces() -> Result<usize> {
        Ok(String::from_utf8(
            Command::new("gsettings")
                .arg("get")
                .arg("org.gnome.desktop.wm.preferences")
                .arg("num-workspaces")
                .output()?
                .stdout,
        )?
        .trim()
        .parse()?)
    }
    fn get_wm_keybinding(gsettings_key: &str) -> Result<String> {
        Ok(String::from_utf8(
            Command::new("gsettings")
                .arg("get")
                .arg("org.gnome.desktop.wm.keybindings")
                .arg(gsettings_key)
                .output()?
                .stdout,
        )?)
    }

    fn set_wm_keybinding(gsettings_key: &str, gsettings_value: &str) -> Result<()> {
        let s = String::from_utf8(
            Command::new("gsettings")
                .arg("set")
                .arg("org.gnome.desktop.wm.keybindings")
                .arg(gsettings_key)
                .arg(gsettings_value)
                .output()?
                .stdout,
        )?;
        println!("{}", s);
        Ok(())
    }
}

impl MyApp {
    fn new() -> Self {
        let mut app = Self::default();
        app.init_keysyms();
        app.gen_workspace_keybinding_map();
        app.get_gsettings_values_from_config();
        app.num_of_workspaces = GSettings::get_number_of_workspaces().unwrap().to_string();
        app
    }

    fn init_keysyms(&mut self) {
        let keys: &str = include_str!("../gnome-keysyms.txt");

        let lines: Vec<&str> = keys.split('\n').collect();

        for line in lines {
            let s: Vec<&str> = line.split_whitespace().collect();
            if s.len() >= 3 {
                self.key_to_keysym.insert(s[2].into(), s[0].into());
                self.keysym_to_key.insert(s[0].into(), s[2].into());
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

    fn get_gsettings_value_from_config(&mut self, i: usize) -> Result<()> {
        let v = self.workspace_keybinding_map.get_mut(&i).unwrap();
        v.gsettings_value = GSettings::get_wm_keybinding(&v.gsettings_key)?;

        // save the original index of modifier vec
        let mut m_vals: Vec<(usize, Modifier)> = vec![];
        for i in 0..self.modifier_vec.len() {
            let v = (i, self.modifier_vec[i].clone());
            m_vals.push(v);
        }

        // reverse sort array by string length to get the longest common string first
        m_vals.sort_by(|a, b| b.1.gsettings_value.len().cmp(&a.1.gsettings_value.len()));

        for (i, m) in m_vals {
            if !m.gsettings_value.is_empty() && v.gsettings_value.contains(&m.gsettings_value) {
                v.modifier_index = i;
                break;
            }
        }
        let m = self.modifier_vec[v.modifier_index]
            .gsettings_value
            .to_string();

        let keysym = v
            .gsettings_value
            .replace(&m, "")
            .replace(['\'', '[', ']'], "")
            .replace("@as", "")
            .trim()
            .to_string();

        v.keybinding = match self.keysym_to_key.get(&keysym) {
            Some(key) => key.to_string(),
            None => keysym.to_string(),
        };
        Ok(())
    }

    fn get_gsettings_values_from_config(&mut self) -> Result<()> {
        for k in self.workspace_keybinding_map.clone().keys() {
            self.get_gsettings_value_from_config(*k)?;
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

            let keybind = match self.key_to_keysym.get(&selection.keybinding) {
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

            if ui.button("Overwrite").clicked() {
                let res = GSettings::set_wm_keybinding(
                    &selection.gsettings_key,
                    &selection.converted_keybinding,
                );

                match res {
                    Ok(()) => {
                        self.get_gsettings_value_from_config(k).unwrap();
                    }
                    Err(e) => {
                        println!("{}", e);
                    }
                }
            }
        });
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Number of Workspaces");
                let te = TextEdit::singleline(&mut self.num_of_workspaces);
                ui.add_sized(Vec2::new(40.0, 20.0), te);
                if ui.button("Overwrite").clicked() {
                    GSettings::set_number_of_workspaces(self.num_of_workspaces.parse().unwrap())
                        .unwrap();
                    self.num_of_workspaces =
                        GSettings::get_number_of_workspaces().unwrap().to_string();
                }
            });

            ui.horizontal(|ui| {
                if ui
                    .button("Disable switch-to-application shortcuts")
                    .clicked()
                {
                    GSettings::disable_switch_to_application_shortcuts().unwrap();
                }
            });

            ui.heading("Shortcuts");
            for (k, _) in self.workspace_keybinding_map.clone() {
                self.workspace_keybinding_input(ui, k);
            }
        });
    }
}
