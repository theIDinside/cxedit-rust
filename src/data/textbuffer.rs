use crate::data::gapbuffer::GapBuffer;
use crate::editor::view::View;
use std::cmp::Ordering;
use crate::cmd::MoveKind;
use crate::cmd::MoveDir::{Next, Previous};
use crate::data::BufferString;
pub enum ObjectKind {
    Word,
    Line,
    Block
}

pub enum RangeType {
    FullInclusive, // "hello world" means world, has position (6, 10), since 10 is included, but it is NOT the length of "world", that 10-6 != 5
    EndExclusive, // "hello world" means world, has position (6, 11), since 11 is not included, but it is the length of "world", since 11-6 = 5
    FullExclusive, // "hello world" means world, has position (5, 11), since 11 is not included, but it is the length of "world", since 11-5 != 5
}

pub enum TextObject {
    Word(usize, usize, RangeType),
    Line(usize, usize, RangeType),
    Block(usize, usize, RangeType)
}

enum Input {
    Character(char),
    Whitespace(char),
}

#[derive(Clone)]
pub struct TextPosition {
    pub absolute: usize,
    line_start_absolute: usize,
    line_number: usize
}

impl Ord for TextPosition {
    fn cmp(&self, other: &Self) -> Ordering {
        self.absolute.cmp(&other.absolute)
    }
}

impl Eq for TextPosition {

}

impl PartialOrd for TextPosition {
    fn partial_cmp(&self, other: &TextPosition) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl PartialEq for TextPosition {
    fn eq(&self, other: &TextPosition) -> bool {
        self.absolute == other.absolute
    }
}


impl TextPosition {
    pub fn new() -> TextPosition {
        TextPosition {
            absolute: 0,
            line_start_absolute: 0,
            line_number: 0
        }
    }

    pub fn get_line_start_abs(&self) -> usize {
        self.line_start_absolute
    }
}

impl From<(usize, usize, usize)> for TextPosition {
    fn from((absolute, line_start_absolute, line_number): (usize, usize, usize)) -> Self {
        TextPosition {
            absolute,
            line_start_absolute,
            line_number
        }
    }
}

#[derive(Clone, Copy)]
pub enum Cursor {
    Absolute(usize),
    Buffer
}

impl Cursor {
    pub fn to_row_col(&self, tb: &Textbuffer) -> (usize, usize) {
        match self {
            Cursor::Absolute(pos) => {
                let text_pos = tb.get_line_number_at(*pos);
                (pos - text_pos.line_start_absolute, text_pos.line_number)
            },
            _ => {
                (0, 0)
            }
        }
    }
}

pub struct Textbuffer {
    data: GapBuffer<char>,
    scratch: Vec<String>,
    observer: Option<Box<View>>,
    cursor: Cursor
}

impl Textbuffer {
    pub fn new() -> Textbuffer {
        let gb = GapBuffer::new();
        Textbuffer {
            cursor: Cursor::Absolute(gb.get_pos()),
            data: gb,
            scratch: Vec::new(),
            observer: None
        }
    }

    pub fn find_range_of(&self, cursor: Cursor, kind: ObjectKind) -> (TextPosition, TextPosition) {
        match kind {
            ObjectKind::Word => {
                let b = self.data.iter_begin_to_cursor(cursor).rev().position(|c| *c == ' ' || *c == '\n').and_then(|c| Some(c + 1)).unwrap_or(0usize);
                let e = self.data.iter_cursor_to_end(cursor).position(|c| *c == ' ' || *c == '\n').and_then(|c| Some(c - 1)).unwrap_or(self.data.len());
                let mut bTp = self.get_line_number_at(b);
                let mut eTp = bTp.clone();
                bTp.absolute = b;
                eTp.absolute = e;
                (bTp, eTp)
            },
            ObjectKind::Line => {
                let b = self.data.iter_begin_to_cursor(cursor).rev().position(|c| *c == '\n').and_then(|c| Some(c + 1)).unwrap_or(0usize);
                let e = self.data.iter_cursor_to_end(cursor).position(|c| *c == '\n').unwrap_or(self.data.len());
                let mut bTp = self.get_line_number_at(b);
                let mut eTp = bTp.clone();
                bTp.absolute = b;
                eTp.absolute = e;
                (bTp, eTp)
            },
            ObjectKind::Block => {
                // N.B! This is just an example.. not all programming languages have { } blocks.
                let mut lvl = 1;
                let b = self.data.iter_begin_to_cursor(cursor).rev().position(|c| *c == '{').unwrap_or(0usize);
                let e = self.data.iter_cursor_to_end(cursor).position(|c| {
                  if *c == '{' {
                      lvl += 1;
                  } else if *c == '}' {
                      lvl -= 1;
                    }
                return *c == '}' && lvl == 0;
                }).unwrap_or(self.data.len());
                let mut bTp = self.get_line_number_at(b);
                let mut eTp = self.get_line_number_at(e);
                bTp.absolute = b;
                eTp.absolute = e;
                (bTp, eTp)
            }
        }

    }

    pub fn get_line_at_cursor(&mut self) -> String {
        self.data.read_string(
            self.data
            .iter_begin_to_cursor(Cursor::Buffer).rev()
            .position(|ch| *ch == '\n')
            .unwrap_or(0)
            ..
            self.data
            .iter_cursor_to_end(Cursor::Buffer)
            .position(|ch| *ch == '\n')
            .unwrap_or(self.data.len()))
    }

    pub fn get_line_number(&self) -> usize {
        let lv: Vec<char> = self.data.iter_begin_to_cursor(self.cursor).rev().filter(|ch| **ch == '\n').map(|c| *c).collect();
        lv.len() + 1
    }

    pub fn get_line_number_at(&self, pos: usize) -> TextPosition {
        let mut tp = TextPosition::new();
        let lv: Vec<char> = self.data.iter_begin_to_cursor(Cursor::Absolute(pos)).rev().filter(|ch| **ch == '\n').map(|c| *c).collect();
        tp.line_start_absolute = self.data.iter_begin_to_cursor(Cursor::Absolute(pos)).rev().position(|ch| *ch == '\n').and_then(|pos| Some(pos + 1)).unwrap_or(0usize);
        tp.line_number = lv.len() + 1;
        tp.absolute = pos;
        tp
    }

    pub fn insert_data(&mut self, data: String) {
        self.data.map_to(data.chars());
    }

    pub fn insert_ch(&mut self, ch: char) {
        if let Cursor::Absolute(abspos) = self.cursor {
            self.data.set_gap_position(abspos);
        }
        self.data.insert(ch);
        self.cursor = Cursor::Absolute(self.data.get_pos())
    }

    pub fn move_cursor(&mut self, movement: MoveKind) {
        match movement {
            MoveKind::Char(dir) => {
                match dir {
                    Previous => {},
                    Next => {}
                }
            },
            MoveKind::Word(dir) => {
                match dir {
                    Previous => {},
                    Next => {}
                }
            },
            MoveKind::Line(dir) => {
                match dir {
                    Previous => {},
                    Next => {}
                }
            }
        }
    }

    pub fn remove(&mut self) {
        if let Some(c) = self.data.remove() {
            println!("Removed character: {}\r\n", c);
        }
    }

    pub fn delete(&mut self) {
        self.data.delete();
    }

    pub fn line_from_buffer_index(&self, absolute: usize) -> Option<TextPosition> {
        let safe_pos_value = std::cmp::min(absolute, self.data.len());
        let line_begin: Vec<usize> = (0..safe_pos_value + 1).rev().filter(|index| *index == 0 || self.data[*index] == '\n').collect();
        if line_begin.is_empty() {
            None
        } else {
            let mut tp = TextPosition::new();
            tp.line_start_absolute = *line_begin.first().unwrap();
            tp.line_number = line_begin.len() - 1;
            tp.absolute = absolute;
            Some(tp)
        }
    }

    pub fn get_line_start_abs(&self, line_number: usize) -> Option<TextPosition> {
        let lines_endings: Vec<usize> =
            (0..self.data.len())
                .filter(|idx| self.data[*idx] == '\n')
                .skip(line_number-1)
                .collect();
        // lines_endings[line_number - 1] points to the character \n, we actually want the character position, after that, for us to "actually" be on the new line
        let this_line_abs = lines_endings[0] + 1;
        Some(TextPosition::from((this_line_abs, this_line_abs, line_number)))
    }

    pub fn register_view(&mut self, v: Box<View>) {
        self.observer = Some(v)
    }

    pub fn from_file(f_name: String) -> Textbuffer {
        use std::fs::read_to_string as read_content;
        use std::path::Path;
        let p = Path::new(&f_name);
        let contents = read_content(p).unwrap();
        let mut tb = Textbuffer {
            data: GapBuffer::new_with_capacity(contents.len()),
            scratch: vec![],
            observer: None,
            cursor: Cursor::Absolute(0)
        };
        tb.data.map_to(contents.chars());
        tb
    }

    pub fn dump_to_string(&self) -> String {
        use crate::data::BufferString;
        self.data.read_string(0..self.data.len()+1)
    }
}
