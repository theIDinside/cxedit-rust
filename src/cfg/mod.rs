// TODO: implement module for writing and reading configuration files, perhaps use serde?
use std::path::Path;
use std::collections::HashMap;
use crate::cmd::Command;
use crate::editor::key::KeyCode;
use crate::editor::color::SetColor;
use crate::editor::color::Color;
use crate::cmd::command_engine::Action;


pub enum CfgSizeOptions {
    Infinite,
    Bounded(usize),
    None
}

pub struct Config {
    key_bindings: HashMap<KeyCode, Command>,
    history_size: CfgSizeOptions,
    bg_color: SetColor,
    fg_color: SetColor,
    stat_line_color: (SetColor, SetColor)
}

impl Default for Config {
    fn default() -> Self {
        let key_bindings =
            [   (KeyCode::CtrlO, Command::Open),
                (KeyCode::CtrlS, Command::Save),
                (KeyCode::CtrlQ, Command::Quit),
                (KeyCode::CtrlC, Command::CommandInput),
                (KeyCode::CtrlG, Command::Jump),
                (KeyCode::CtrlZ, Command::Action(Action::Undo)),
            ].iter().cloned().collect();
        let history_size = CfgSizeOptions::Infinite;
        let bg_color = SetColor::Background(Color::Blue);
        let fg_color = SetColor::Foreground(Color::White);
        let stat_line_color = (SetColor::Background(Color::BrightCyan), SetColor::Background(Color::Black));

        Config {
            key_bindings,
            history_size,
            bg_color,
            fg_color,
            stat_line_color
        }
    }
}

impl Config {
    pub fn read_cfg(_file: &Path) {

    }

    #[inline]
    pub fn get_binding(&self, kc: KeyCode) -> Option<&Command> {
        self.key_bindings.get(&kc)
    }
}