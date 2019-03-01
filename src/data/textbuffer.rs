use data::gapbuffer::GapBuffer;
use super::BufferString;
use editor::view::View;

pub struct Cursor {
    pub line: usize,
    pub column: usize
}

pub struct Textbuffer {
    lines: Vec<GapBuffer<char>>,
    scratch: Vec<String>,
    observer: Option<Box<View>>,
    cursor: Cursor
}

impl Textbuffer {
    pub fn new() -> Textbuffer {
        Textbuffer {
            lines: Vec::new(),
            scratch: Vec::new(),
            cursor: Cursor { line: 0, column: 0},
            observer: None
        }
    }

    pub fn insert_line(&mut self, line: String, pos: usize) {
        if pos > self.lines.len() {
            panic!("Index out of bounds");
        } else if pos == self.lines.len() {
            self.lines.push(GapBuffer::new());
            self.lines.get(pos).unwrap().map_to(line.chars());
        }
    }

    pub fn cut_line(&mut self, line: usize) {
        if line < self.lines.len() {
            let sz = self.lines.get(line).unwrap().len();
            self.scratch.push(self.lines.remove(line).read_string(0..sz+1))
        }
    }

    pub fn insert_ch(&mut self, ch: char) {
        let Cursor {line:l, column: c} = *self.cursor;
        self.lines.get(l).unwrap().insert(char);
        self.cursor.column += 1;
    }

    pub fn register(&mut self, listener: Box<View>) {
        self.observer = Option::from(listener);
    }

    pub fn publish(&self, evt: Event)
}