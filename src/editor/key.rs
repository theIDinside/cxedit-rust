use std::hash::Hash;

#[derive(Hash, Eq, PartialOrd, PartialEq, Clone)]
pub enum KeyCode {
    CtrlBackspace,
    CtrlA,
    CtrlB,
    CtrlC,
    CtrlS,
    CtrlO,
    CtrlQ,
    CtrlZ,
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
            KeyCode::CtrlA => 1,
            KeyCode::CtrlB => 2,
            KeyCode::CtrlC => 3,
            KeyCode::CtrlS => 19,
            KeyCode::CtrlO => 15,
            KeyCode::CtrlQ => 17,
            KeyCode::CtrlZ => 26,
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

#[derive(Hash, Eq, PartialOrd, PartialEq, Clone)]
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