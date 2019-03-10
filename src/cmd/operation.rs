
type Word = BufferObject::CharRange;
type Line = BufferObject::CharRange;

pub enum BufferObject {
    Character(char),
    CharRange(Vec<u8>),
}

pub struct Insert {
    at_position: usize,
    data: BufferObject
}

pub struct Delete {
    at_position: usize,
    data: BufferObject
}