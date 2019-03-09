pub mod command_engine;
type OffsetAbsolute = usize;
use crate::{Deserialize as Des, Serialize as Ser};

pub trait ToOption where Self: Clone {
    fn as_option(&self) -> Option<Self> {
        Some(self.clone())
    }
}

#[derive(Clone, Ser, Des, Debug)]
pub enum MoveDir {
    Previous,
    Next
}
#[derive(Clone, Ser, Des, Debug)]
pub enum MoveKind {
    Word(MoveDir),
    Line(MoveDir),
    Char(MoveDir)
}
// let words be something, and see where clion tabs
#[derive(Clone, Ser, Des, Debug)]
pub enum Command {
    Move(MoveKind),
    CommandInput,
    Jump,
    Find,
    Save,
    Open,
    Quit,
    Action(Operation)
}

use self::Command::{Jump, Find, Save, Open};
use crate::cmd::command_engine::Operation;

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

pub fn flag_match(ch: char) -> Option<StatlineCommandFlag> {
    match ch {
        '!' => Some(StatlineCommandFlag::Force),
        '-' => Some(StatlineCommandFlag::Deny),
        '?' => Some(StatlineCommandFlag::RequestPermission),
        '#' => Some(StatlineCommandFlag::SameWindow),
        '_' => Some(StatlineCommandFlag::SameWindowClose),
        _ => None
    }
}

pub struct StatlineCommandFlagList {
    flags: Vec<StatlineCommandFlag>
}

impl StatlineCommandFlagList {
    pub fn has_to_vec(&self) -> Option<Vec<StatlineCommandFlag>> {
        if self.flags.len() > 0 {
            Some(self.flags.clone())
        } else {
            None
        }
    }
}

impl From<&str> for StatlineCommandFlagList {
    fn from(flags: &str) -> Self {
        StatlineCommandFlagList { flags: flags.chars().filter_map(flag_match).collect() }
    }
}

#[derive(Clone)]
pub enum StatlineCommandFlag {
    Force,
    Deny,
    RequestPermission,
    SameWindow,
    SameWindowClose
}



#[derive(Clone)]
pub enum StatlineCommand {
    OpenFile(Option<String>, Option<Vec<StatlineCommandFlag>>),
    SaveFile(Option<String>, Option<Vec<StatlineCommandFlag>>),
    Goto(Option<usize>),
    Find(Option<String>),
    Error(String)
}
