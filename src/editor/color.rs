use std::fmt::{Display, Formatter, Error as FmtError};
// use std::fmt::Error as FmtError;
use crate::{Deserialize, Serialize};

#[derive(Clone, Deserialize, Serialize, Debug)]
pub enum Color {
    Black = 30,
    Red = 31,
    Green = 32,
    Yellow = 33,
    Blue = 34,
    Magenta = 35,
    Cyan = 36,
    White = 37,
    BrightCyan = 96
}

#[derive(Clone, Deserialize, Serialize, Debug)]
pub enum SetColor {
    Foreground(Color),
    Background(Color)
}

impl SetColor {
    pub fn colorize(&self, data: &str) -> String {
        format!("{}{}\x1b[m", self, data)
    }
}

impl Display for SetColor {
    fn fmt(&self, f: &mut Formatter) -> Result<(), FmtError> {
        match self {
            SetColor::Foreground(color) => {
                // write!(f, "\x1b[38;5;{}m", color.clone() as u8)
                write!(f, "\x1b[{}m", color.clone() as u8)
            },
            SetColor::Background(color) => {
                // TODO: build on the enum so that more colors can be used. Uncomment when that's done
                // write!(f, "\x1b[48;5;{}m", color.clone() as u8 + 10)
                // the simpler color ansi seq, only has colors between 30-40 and 40-50 for bg
                write!(f, "\x1b[{}m", color.clone() as u8 + 10)
            }
        }
    }
}