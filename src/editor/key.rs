use crate::{Serialize, Deserialize};
#[derive(Hash, Eq, PartialOrd, PartialEq, Clone, Serialize, Deserialize, Debug)]
pub enum KeyCode {
    CtrlBackspace,
    CtrlA,
    CtrlB,
    CtrlC,
    CtrlG,
    CtrlS,
    CtrlV,
    CtrlO,
    CtrlQ,
    CtrlZ,
    CtrlW,
    Enter,
    Tab,
    Esc,
    Backspace,
    Character(char),
    Escaped(EscapeKeyCode),
    None,
}

impl KeyCode {
    pub fn to_idx(&self) -> usize {
        match self {
            KeyCode::CtrlA=> 1,
            KeyCode::CtrlB=> 2,
            KeyCode::CtrlC=> 3,
            KeyCode::CtrlG=> 7,
            KeyCode::CtrlBackspace=> 8,
            KeyCode::Tab=> 9,
            KeyCode::Enter =>13,
            KeyCode::CtrlO =>15,
            KeyCode::CtrlQ =>17,
            KeyCode::CtrlS =>19,
            KeyCode::CtrlV =>22,
            KeyCode::CtrlW => 23,
            KeyCode::CtrlZ =>26,
            _ => 0
        }
    }

    pub fn from_idx(idx: usize) -> KeyCode {
        match idx {
            1 => KeyCode::CtrlA,
            2 => KeyCode::CtrlB,
            3 => KeyCode::CtrlC,
            19 => KeyCode::CtrlS,
            15 => KeyCode::CtrlO,
            17 => KeyCode::CtrlQ,
            26 => KeyCode::CtrlZ,
            _ => KeyCode::None
        }
    }
}

#[derive(Hash, Eq, PartialOrd, PartialEq, Clone, Serialize, Deserialize, Debug)]
pub enum EscapeKeyCode {
    Left,
    Right,
    Up,
    Down
}

impl EscapeKeyCode {
    pub fn output(&self) -> &str {
        match self {
            EscapeKeyCode::Down => "\x1b[1B",
            EscapeKeyCode::Up => "\x1b[1A",
            EscapeKeyCode::Left => "\x1b[1D",
            EscapeKeyCode::Right => "\x1b[1C"
        }
    }
}