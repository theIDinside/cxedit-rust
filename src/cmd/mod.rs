pub mod command_engine;
type OffsetAbsolute = usize;

pub enum MoveDir {
    Previous,
    Next
}

pub enum MoveKind {
    Word(MoveDir),
    Line(MoveDir),
    Char(MoveDir)
}
// let words be something, and see where clion tabs
pub enum Command {
    Move(MoveKind),
    Jump,
    Find,
    Save,
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

pub enum KeyEvent {

}