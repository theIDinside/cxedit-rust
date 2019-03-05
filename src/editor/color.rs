use std::fmt::{Display, Formatter, Error as FmtError};
// use std::fmt::Error as FmtError;

#[derive(Clone)]
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
                write!(f, "\x1b[38;5;{}m", color.clone() as u8)
            },
            SetColor::Background(color) => {
                write!(f, "\x1b[48;5;{}m", color.clone() as u8 + 10)
            }
        }
    }
}