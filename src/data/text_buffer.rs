use crate::data::gap_buffer::GapBuffer;
use crate::editor::view::View;
use std::cmp::Ordering;
use crate::cmd::MoveKind;
use crate::cmd::MoveDir::{Next, Previous};
use crate::data::BufferString;
use std::sync::Arc;
use crate::comms::observer::EventListener;
use crate::comms::observer::Event;
use crate::comms::observer::EventData;
use std::path::Path;
use std::fs::File;
use std::io::Write;
use crate::data::FileResult;
use crate::data::SaveFileError;
use crate::editor::FileOpt;
use std::error::Error;

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

#[derive(Clone)]
pub struct TextPosition {
    pub absolute: usize,
    pub line_start_absolute: usize,
    pub line_number: usize
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

    pub fn get_line_position(&self) -> usize {
        self.absolute - self.line_start_absolute
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
                let text_pos = tb.get_text_position_info(*pos);
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
    _scratch: Vec<String>,
    observer: Option<Arc<View>>,
    cursor: TextPosition,
    dirty: bool,
    line_count: usize
}

impl Textbuffer {
    pub fn new() -> Textbuffer {
        let gb = GapBuffer::new();
        let mut tp = TextPosition::new();
        tp.absolute = gb.get_pos();
        Textbuffer {
            cursor: tp,
            data: gb,
            _scratch: Vec::new(),
            observer: None,
            dirty: false,
            line_count: 0,
        }
    }

    pub fn get_textpos(&self) -> TextPosition {
        self.cursor.clone()
    }
    pub fn get_gap_textpos(&self) -> TextPosition {
        self.get_text_position_info(self.data.get_pos())
    }

    pub fn set_textpos(&mut self, pos: usize) {
        if pos <= self.len() {
            self.cursor = self.get_text_position_info(pos);
            self.data.set_gap_position(pos);
        }
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn set_pristine(&mut self) {
        self.dirty = false;
    }

    pub fn find_range_of(&self, cursor: Cursor, kind: ObjectKind) -> (TextPosition, TextPosition) {
        match kind {
            ObjectKind::Word => {
                let b = self.data.iter_begin_to_cursor(cursor).rev().position(|c| *c == ' ' || *c == '\n').and_then(|c| Some(c + 1)).unwrap_or(0usize);
                let e = self.data.iter_cursor_to_end(cursor).position(|c| *c == ' ' || *c == '\n').and_then(|c| Some(c - 1)).unwrap_or(self.data.len()-1);
                let mut b_tp = self.get_text_position_info(b);
                let mut e_tp = b_tp.clone();
                b_tp.absolute = b;
                e_tp.absolute = e;
                (b_tp, e_tp)
            },
            ObjectKind::Line => {
                let b = self.data.iter_begin_to_cursor(cursor).rev().position(|c| *c == '\n').and_then(|c| Some(c + 1)).unwrap_or(0usize);
                let e = self.data.iter_cursor_to_end(cursor).position(|c| *c == '\n').unwrap_or(self.data.len());
                let mut b_tp = self.get_text_position_info(b);
                let mut e_tp = b_tp.clone();
                b_tp.absolute = b;
                e_tp.absolute = e;
                (b_tp, e_tp)
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
                let mut b_tp = self.get_text_position_info(b);
                let mut e_tp = self.get_text_position_info(e);
                b_tp.absolute = b;
                e_tp.absolute = e;
                (b_tp, e_tp)
            }
        }
    }

    pub fn get_line_at_cursor(&self) -> String {
        let len = self.len();
        let line_begin_absolute = (0..self.data.get_pos()).into_iter().rposition(|idx| self.data[idx] == '\n').and_then(|pos| Some(pos+1)).unwrap_or(0usize);
        let line_end_absolute = (self.data.get_pos()..self.data.len()).into_iter().position(|idx| self.data[idx] == '\n' || idx == len-1).and_then(|pos| Some(pos+1)).unwrap_or(self.data.len());
        self.data.read_string(line_begin_absolute..line_end_absolute)
    }

    pub fn get_data_range(&self, begin: usize, end: usize) -> String {
        if end > self.len() {
            panic!("Index out of bounds!")
        } else {
            self.data.read_string(begin..end)
        }
    }

    pub fn get_line_number(&self) -> usize {
        let lv1: Vec<char> = (0..self.data.get_pos()).into_iter().rev().filter(|idx| self.data[*idx] == '\n').map(|i| self.data[i]).collect();
        lv1.len() + 1
    }

    pub fn get_line_number_editing(&self) -> usize {
        self.cursor.line_number
    }

    pub fn get_text_position_info(&self, pos: usize) -> TextPosition {
        let mut tp = TextPosition::new();
        let lv: Vec<char> = (0..pos).into_iter().rev().filter(|i| self.data[*i] == '\n').map(|i| self.data[i]).collect();
        let lineno = lv.len();
        tp.line_start_absolute = (0..pos).into_iter().rposition(|i| self.data[i] == '\n').and_then(|pos| Some(pos + 1)).unwrap_or(0usize);
        tp.line_number = lineno;
        tp.absolute = pos;
        tp
    }

    pub fn get_line_end_abs(&self, line_number: usize) -> Option<TextPosition> {
        let len = self.data.len();
        let lines_endings: Vec<usize> =
            (0..self.data.len())
                .into_iter()
                .filter(|idx| self.data[*idx] == '\n' || *idx == len-1)
                .collect::<Vec<usize>>().into_iter().enumerate().take_while(|(index, _)| {
                index <= &(line_number+1)
            }).map(|(_, l)| {
                l
            }).collect();

        if lines_endings.len() == line_number-1 && line_number -1 > 0 {
            let linebegin = lines_endings[line_number-1]+1;
            let line_end = (linebegin..self.data.len()).into_iter().rposition(|idx| self.data[idx] == '\n' || idx == len-1).unwrap_or(len);
            return Some(TextPosition::from((line_end, linebegin, line_number-1)));
        }

        if line_number > lines_endings.len() && lines_endings.len() == 0 {
            Some(TextPosition::from((self.data.len(), 0, 0)))
        } else if line_number > lines_endings.len() {
            let this_line_abs = lines_endings[lines_endings.len()-1]; // here it means, this value is self.data.len()
            Some(TextPosition::from((self.data.len(), this_line_abs, lines_endings.len())))
        } else if line_number == lines_endings.len() {
            let this_line_abs = lines_endings[line_number-1]; // here it means, this value is self.data.len()
            Some(TextPosition::from((self.data.len(), this_line_abs, lines_endings.len())))
        } else {
            let this_line_abs = lines_endings[line_number]; // here it means, this value is self.data.len()
            Some(TextPosition::from((lines_endings[line_number+1], this_line_abs, lines_endings.len())))
        }
    }

    pub fn get_line_start_abs(&self, line_number: usize) -> Option<TextPosition> {
        let lines_endings: Vec<usize> =
            (0..self.data.len())
                .into_iter()
                .filter(|idx| self.data[*idx] == '\n')
                .collect::<Vec<usize>>().into_iter().enumerate().take_while(|(index, _)| {
                index <= &(line_number+1)
            }).map(|(_, l)| {
                l
            }).collect();
        if line_number > lines_endings.len() && lines_endings.len() == 0 {
            Some(TextPosition::from((self.len(), 0, 1)))
        } else if line_number > lines_endings.len() {
            let this_line_abs = lines_endings[lines_endings.len()-1] + 1;
            Some(TextPosition::from((this_line_abs, this_line_abs, lines_endings.len())))
        } else {
            let this_line_abs = lines_endings[lines_endings.len()-1] + 1;
            Some(TextPosition::from((this_line_abs, this_line_abs, line_number)))
        }
    }

    pub fn insert_data(&mut self, data: &str) {
        self.data.set_gap_position(self.cursor.absolute);
        self.data.map_to(data.chars());
        self.cursor.absolute += data.len();
    }

    pub fn get_absolute_cursor_pos(&self) -> usize {
        self.cursor.absolute
    }

    pub fn insert_ch(&mut self, ch: char) {
        self.data.set_gap_position(self.cursor.absolute);
        self.data.insert(ch);
        if ch == '\n' {
            self.line_count += 1;
            self.cursor = self.get_text_position_info(self.data.get_pos());
        } else {
            self.cursor.absolute += 1;
        }
        if let Some(obs) = self.observer.as_ref() {
            obs.on_event(Event::INSERTION(self.cursor.absolute-1, EventData::Char(ch)));
        }
    }

    pub fn move_cursor(&mut self, movement: MoveKind) -> Option<TextPosition>  {
        match movement {
            MoveKind::Char(dir) => {
                match dir {
                    Previous => {
                        if self.cursor.absolute > 0 {
                            self.cursor = self.get_text_position_info(self.data.get_pos()-1);
                            self.set_textpos(self.cursor.absolute);
                        }
                        Some(self.cursor.clone())
                    },
                    Next => {
                        if self.cursor.absolute < self.data.len() {
                            self.cursor.absolute += 1;
                            if let Some(ch) = self.data.get(self.cursor.absolute-1) {
                                if *ch == '\n' {
                                    self.cursor = self.get_text_position_info(self.cursor.absolute);
                                }
                            }
                            self.set_textpos(self.cursor.absolute);
                        }
                        Some(self.cursor.clone())
                    }
                }
            },
            MoveKind::Word(dir) => {
                match dir {
                    Previous => {},
                    Next => {}
                }
                Some(self.cursor.clone())
            },
            MoveKind::Line(dir) => {
                match dir {
                    Previous => {

                    },
                    Next => {

                    }
                }
                Some(self.cursor.clone())
            }
        }
    }

    pub fn get_at(&self, pos: usize) -> Option<char> {
        if let Some(c) = self.data.get(pos) {
            Some(*c)
        } else {
            None
        }
    }

    pub fn remove(&mut self) -> Option<char> {
        if let Some(c) = self.data.remove() {
            if c == '\n' {
                self.line_count -= 1;
            }
            self.cursor = self.get_text_position_info(self.data.get_pos());
            Some(c)
        } else {
            None
        }
    }

    pub fn clear_buffer_contents(&mut self) {
        self.data = GapBuffer::new();
        self.cursor = TextPosition::new();
        self.line_count = 0;
    }

    pub fn delete(&mut self) -> Option<char> {
        if let Some(character) = self.data.delete() {
            if character == '\n' {
                self.line_count -= 1;
            }
            Some(character)
        } else {
            None
        }
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

    pub fn register_view(&mut self, v: Arc<View>) {
        self.observer = Some(v.clone())
    }

    pub fn from_file(f_name: String) -> Textbuffer {
        use std::fs::read_to_string as read_content;
        use std::path::Path;
        let p = Path::new(&f_name);
        let contents = read_content(p).unwrap();
        let mut tb = Textbuffer {
            data: GapBuffer::new_with_capacity(contents.len()),
            _scratch: vec![],
            observer: None,
            cursor: TextPosition::new(),
            line_count: contents.chars().filter(|c| *c == '\n').collect::<Vec<char>>().len(),
            dirty: false
        };
        tb.data.map_to(contents.chars());
        tb
    }

    pub fn dump_to_string(&self) -> String {
        use crate::data::BufferString;
        self.data.read_string(0..self.data.len()+1)
    }

    pub fn save_to_file(&self, file_name: &Path, save_opts: Option<FileOpt>) -> FileResult<usize> {
        if file_name.exists() {
            return Err(SaveFileError::FileExisted(file_name.to_str().unwrap().into()));
        }

        match save_opts {
            None => {
                match File::create(file_name) {
                    Ok(ref mut f) => {
                        f.write(self.dump_to_string().as_bytes()).map_err(|std_err| SaveFileError::Other(file_name.to_str().unwrap().into(), std_err.description().into()))
                    },
                    Err(e) => {
                        Err(SaveFileError::Other(file_name.to_str().unwrap().to_string(), e.description().into()))
                    }
                }
            },
            Some(fopt) => {
                match fopt {
                    FileOpt::NoOverwrite => {
                        Ok(0)
                    },
                    FileOpt::Overwrite => {
                        Ok(0)
                    }
                }
            }
        }
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }
}
