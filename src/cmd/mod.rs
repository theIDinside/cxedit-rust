pub mod command_engine;
type OffsetAbsolute = usize;

#[derive(Clone)]
pub enum MoveDir {
    Previous,
    Next
}
#[derive(Clone)]
pub enum MoveKind {
    Word(MoveDir),
    Line(MoveDir),
    Char(MoveDir)
}
// let words be something, and see where clion tabs
#[derive(Clone)]
pub enum Command {
    Move(MoveKind),
    CommandInput,
    Jump,
    Find,
    Save,
    Open,
    Quit
}

use self::Command::{Jump, Find, Save, Open};

impl From<&Command> for &str {
    fn from(cmd: &Command) -> Self {
        match cmd {
            Jump => "[goto]: ",
            Find => "[find]: ",
            Save => "[save]: ",
            Open => "[open]: ",
            _ => ""
        }
    }
}

impl From<&Command> for String {
    fn from(cmd: &Command) -> Self {
        match cmd {
            Jump => "[goto]: ".into(),
            Find => "[find]: ".into(),
            Save => "[save]: ".into(),
            Open => "[open]: ".into(),
            _ => "".into()
        }
    }
}

pub enum Operation {
    Insert(OffsetAbsolute, OpParam),
    Delete(OffsetAbsolute, usize),
}

pub enum OpParam {
    Range(Box<str>),
    Char(char)
}

pub enum UserEvent {
    Command(Command),
    Operation(Operation),
}
