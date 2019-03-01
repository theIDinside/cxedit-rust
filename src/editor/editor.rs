use data::Gap::GapBuffer;

pub struct Editor {
    buffers: Vec<Box<GapBuffer<char>>>,
    current: usize,
}


impl Editor {
    pub fn new() -> Editor {
        Editor {
            buffers: vec![],
            current: 0
        }
    }
}