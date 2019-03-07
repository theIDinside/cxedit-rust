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
use std::thread::sleep;
use std::time::Duration;
use std::thread::current;
use crate::editor::editor::debug_sleep;

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
    pub line_index: usize
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
            line_index: 0
        }
    }

    pub fn get_line_start_abs(&self) -> usize {
        self.line_start_absolute
    }

    pub fn get_line_position(&self) -> usize {
        self.absolute - self.line_start_absolute
    }

    pub fn get_line_number(&self) -> usize {
        self.line_index + 1
    }
}

impl From<(usize, usize, usize)> for TextPosition {
    fn from((absolute, line_start_absolute, line_number): (usize, usize, usize)) -> Self {
        TextPosition {
            absolute,
            line_start_absolute,
            line_index: line_number
        }
    }
}

impl Default for TextPosition {
    fn default() -> Self {
        TextPosition {
            absolute: 0,
            line_start_absolute: 0,
            line_index: 0
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
                (pos - text_pos.line_start_absolute, text_pos.line_index)
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
    pub line_count: usize
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
            line_count: 1,
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
        /*if end > self.len() {
            print ln!("Index out of bounds!");
            sleep(Duration::from_millis(1500));
            panic!("Index out of bounds!")
        } else {
        }*/
            self.data.read_string(begin..end)
    }

    pub fn get_line_number(&self) -> usize {
        let lv1: Vec<usize> = (0..self.data.get_pos()).into_iter().rev().filter(|idx| self.data[*idx] == '\n').collect();
        lv1.len()
    }

    pub fn get_line_number_editing(&self) -> usize {
        self.cursor.line_index
    }

    pub fn get_text_position_info(&self, pos: usize) -> TextPosition {
        let mut tp = TextPosition::new();
        let lv: Vec<char> = (0..pos).into_iter().rev().filter(|i| self.data[*i] == '\n').map(|i| self.data[i]).collect();
        let lineno = lv.len();
        tp.line_start_absolute = (0..pos).into_iter().rposition(|i| self.data[i] == '\n').and_then(|pos| Some(pos + 1)).unwrap_or(0usize);
        tp.line_index = lineno;
        tp.absolute = pos;
        tp
    }

    /**
    * The line number is 0-indexed. When converting a line in the text document, to a line on screen,
    * for writing characters to screen, use ViewCursor::from(&TextPosition) to convert to proper
    * terminal row/cell indexing. (which uses a 1,1 index).
    */
    pub fn get_line_end_pos(&self, line: usize) -> Option<TextPosition> {
        let buf_len = self.len();
        if buf_len == 0 {
            return Some(TextPosition::default());
        }
        let mut line_counter = 0;

        let nlcp = (0..self.data.len()).into_iter().take_while(|index| {
            if self.data[*index] == '\n' {
                line_counter += 1;
            }
            line_counter < line+1
        }).collect::<Vec<usize>>().into_iter().filter(|i| self.data[*i] == '\n').collect::<Vec<usize>>();
        line_counter = 0;
        let line_end = *nlcp.last().unwrap_or(&buf_len);
        let line_begin =
            (0..line_end).into_iter()
                .rposition(|i| self.data[i] == '\n').and_then(|v| Some(v+1)).unwrap_or(0usize);
        // println!("\x1b[25;10H Line begin: {} End: {}: #{}", line_begin, line_end, line);
        Some(TextPosition::from((line_end, line_begin, nlcp.len())))
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

    pub fn get_line_abs_index(&self, line_number: usize) -> Option<TextPosition> {
        let lines: Vec<usize> =
            (0..self.len())
                .into_iter()
                .filter(|&index| index == 0 || self.data[index] == '\n')
                .collect();
        let line_pos = lines.get(line_number-1).and_then(|&value| Some(value+1)).unwrap_or(0usize);
        Some(TextPosition::from((line_pos, line_pos, line_number-1)))
    }

    pub fn get_line_start_abs(&self, line_number: usize) -> Option<TextPosition> {
        let lines_endings: Vec<usize> =
            (0..self.data.len())
                .into_iter()
                .filter(|idx| self.data[*idx] == '\n')
                .collect::<Vec<usize>>().into_iter().enumerate().take_while(|(index, _)| {
                index <= &(line_number)
            }).map(|(_, l)| {
                l
            }).collect();
        if line_number > lines_endings.len() && lines_endings.len() == 0 {
            Some(TextPosition::from((self.len(), 0, 0)))
        } else if line_number >= lines_endings.len() {
            let this_line_abs = lines_endings.last().and_then(|v| Some(v+1)).unwrap_or(0);
            Some(TextPosition::from((this_line_abs, this_line_abs, lines_endings.len()-1)))
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
                            self.cursor = self.get_text_position_info(self.cursor.absolute-1);
                        }
                        Some(self.cursor.clone())
                    },
                    Next => {
                        if self.cursor.absolute < self.data.len() {
                            self.cursor.absolute += 1;
                            if let Some(ch) = self.data.get(self.cursor.absolute-1) {
                                if *ch == '\n' {
                                    self.cursor.line_start_absolute = self.cursor.absolute;
                                    self.cursor.line_index += 1;
                                }
                            }
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
                        if self.cursor.absolute < self.len() && (self.cursor.line_index + 1) < self.line_count {
                            if let Some(next_line_start) = self.find_next_line_abs_offset(self.cursor.absolute) {
                                let column_pos = self.cursor.get_line_position();
                                self.cursor.line_index += 1;
                                self.cursor.line_start_absolute = next_line_start;
                                let end = self.find_next_line_abs_offset(next_line_start).and_then(|val| Some(val-1)).unwrap_or(self.len());
                                let line_len = end - next_line_start;
                                if column_pos > line_len {
                                    self.cursor.absolute = next_line_start + line_len;
                                } else {
                                    self.cursor.absolute = next_line_start + column_pos;
                                }
                            }
                        }
                    }
                }
                Some(self.cursor.clone())
            }
        }
    }

    pub fn find_next_line_abs_offset(&self, current: usize) -> Option<usize> {
        (current..self.len())
            .into_iter()
            .position(|i| {
                let a = self.data.get(i);
                if a.is_some() && *a.unwrap() == '\n' {
                    true
                } else {
                    false
                }
            })
            .and_then(|val| {
                Some(current+val+1)
            })
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
        self.cursor = TextPosition::default();
        self.line_count = 1;
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
        let line_begin = (0..safe_pos_value).rposition(|index| index == 0 || self.data[index] == '\n').and_then(|v| Some(v+1)).unwrap_or(0usize);
        let line_numbers = (0..safe_pos_value).rev().filter(|&index| index == 0 || self.data[index] == '\n').collect::<Vec<usize>>();
        let mut tp = TextPosition::new();
        tp.line_start_absolute = line_begin;
        tp.line_index = line_numbers.len() - 1;
        tp.absolute = absolute;
        Some(tp)
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
            line_count: contents.chars().filter(|c| *c == '\n').collect::<Vec<char>>().len() + 1,
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
