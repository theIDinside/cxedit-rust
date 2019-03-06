// TODO: implement module for writing and reading configuration files, perhaps use serde?
use std::path::Path;
use std::collections::HashMap;
use crate::cmd::Command;
use crate::cmd::Command::{CommandInput, Save, Open, Quit, Move, Jump, Find};
use crate::editor::key::KeyCode;


pub struct Config {
    key_bindings: HashMap<KeyCode, Command>
}

impl Default for Config {
    fn default() -> Self {
        let key_bindings =
            [   (KeyCode::CtrlO, Command::Open),
                (KeyCode::CtrlS, Command::Save),
                (KeyCode::CtrlQ, Command::Quit),
                (KeyCode::CtrlC, Command::CommandInput)
            ].iter().cloned().collect();
        Config {
            key_bindings
        }
    }
}

impl Config {
    pub fn read_cfg(file: &Path) {

    }

    #[inline]
    pub fn get_binding(&self, kc: KeyCode) -> Option<&Command> {
        self.key_bindings.get(&kc)
    }
}