
// TODO: implement module for writing and reading configuration files, perhaps use serde?
use std::path::Path;
use std::collections::HashMap;
use crate::cmd::Command;
use crate::editor::key::KeyCode;
use crate::editor::color::SetColor;
use crate::editor::color::Color;
use crate::cmd::command_engine::Operation;
use std::path::PathBuf;
use crate::{Serialize, Deserialize};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum CfgSizeOptions {
    Infinite,
    Bounded(usize),
    None
}

use std::collections::hash_set;

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Config {
    file_name: PathBuf,
    key_bindings: HashMap<KeyCode, Command>,
    command_combo_key_bindings: HashMap<KeyCode, HashMap<KeyCode, Command>>,
    history_size: CfgSizeOptions,
    bg_color: SetColor,
    fg_color: SetColor,
    stat_line_color: (SetColor, SetColor)
}

use std::fs::read_to_string;


impl Default for Config {
    fn default() -> Self {

        let default_cfg = Path::new("config.rc");
        if default_cfg.exists() {
            match read_to_string(default_cfg) {
                Ok(contents) => {
                    let cfg = serde_json::from_str(&contents);
                    match cfg {
                        Ok(config) => {
                            return config;
                        },
                        Err(e) => {

                        }
                    }
                },
                Err(e) => {

                }
            }
        }

        let file_name = PathBuf::from("config.rc");
        let key_bindings =
            [   (KeyCode::CtrlO, Command::Open),
                (KeyCode::CtrlS, Command::Save),
                (KeyCode::CtrlQ, Command::Quit),
                (KeyCode::CtrlC, Command::CommandInput),
                (KeyCode::CtrlG, Command::Jump),
                (KeyCode::CtrlZ, Command::Action(Operation::Undo)),
            ].iter().cloned().collect();

        let command_combo_key_bindings =
            [
                (KeyCode::CtrlW, [ (KeyCode::CtrlW, Command::Action(Operation::Copy(ObjectKind::Line))) ].iter().cloned().collect::<HashMap<KeyCode, Command>>())
            ].iter().cloned().collect();
        let history_size = CfgSizeOptions::Infinite;
        let bg_color = SetColor::Background(Color::Blue);
        let fg_color = SetColor::Foreground(Color::White);
        let stat_line_color = (SetColor::Background(Color::BrightCyan), SetColor::Background(Color::Black));

        Config {
            file_name,
            key_bindings,
            command_combo_key_bindings,
            history_size,
            bg_color,
            fg_color,
            stat_line_color
        }
    }
}
use std::fs;
use crate::data::text_buffer::ObjectKind;
use std::hash::Hash;

impl Config {
    pub fn read_config(_file: &Path) -> Config {
        let c = Config::default();
        match fs::read_to_string(_file) {
            Ok(contents) => {
                let deserialized = serde_json::from_str(&contents);
                match deserialized {
                    Ok(config) => {
                        config
                    },
                    Err(e) => {
                        Config::default()
                    }
                }
            },
            Err(e) => {
                Config::default()
            }
        }
    }

    pub fn save_config(&self, file_path: &Path) {
        let config_contents = serde_json::to_string_pretty(self);
        if file_path.exists() {
            match config_contents {
                Ok(data) => {

                },
                Err(e) => {

                }
            }
        }
    }

    #[inline]
    pub fn get_binding(&self, kc: KeyCode) -> Option<&Command> {
        self.key_bindings.get(&kc)
    }

    #[inline]
    pub fn get_combo_bindings(&self, kc: &KeyCode) -> Option<&HashMap<KeyCode, Command>> {
        self.command_combo_key_bindings.get(kc)
    }
}