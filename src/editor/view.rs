use crate::comms::observer::{EventListener, Event};
use crate::data::text_buffer::Textbuffer;
use libc::{STDOUT_FILENO, STDIN_FILENO, c_int, c_ulong, winsize};
use std::mem::zeroed;

static TIOCGWINSZ: c_ulong = 0x5413; // Enum value basically, for requesting terminal window size

use std::sync::{Arc, Mutex};
use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Error as FmtError;
use crate::data::text_buffer::TextPosition;
use std::fmt::Error;
use std::io::stdout;
use std::io::Write;

use crate::editor::color::{SetColor, Color};
use std::thread::sleep;
use std::time::Duration;
use crate::editor::editor::debug_sleep;
use std::ops::Range;

#[derive(Debug)]
pub struct WinDim(pub u16, pub u16);

pub enum ViewOperations {
    ClearLineRest,
    StepRight,
    StepLeft,
    StepUp,
    StepDown,
    LineStart
}

impl Display for ViewOperations {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        let res = match self {
            ViewOperations::ClearLineRest => "\x1b[0K",
            ViewOperations::StepRight => "\x1b[1C",
            ViewOperations::StepLeft => "\x1b[1D",
            ViewOperations::StepUp => "\x1b[1A",
            ViewOperations::StepDown => "\x1b[1B",
            ViewOperations::LineStart => "\x1b[9D",
        };
        write!(f, "{}", res)
    }
}

impl ViewOperations {
    fn as_output(&self) -> &str {
        match self {
            ViewOperations::ClearLineRest => "\x1b[0K",
            ViewOperations::StepRight => "\x1b[1C",
            ViewOperations::StepLeft => "\x1b[1D",
            ViewOperations::StepUp => "\x1b[1A",
            ViewOperations::StepDown => "\x1b[1B",
            ViewOperations::LineStart => "\x1b[9D",
        }
    }
}



impl Display for WinDim {
    fn fmt(&self, f: &mut Formatter) -> Result<(), FmtError> {
        write!(f, "w x h: ({} x {})", self.0, self.1)
    }
}

pub struct WinSize {
    ws_row: u16,
    ws_col: u16,
    ws_xpixel: u16,
    ws_ypixel: u16
}

impl From<WinSize> for WinDim {
    fn from(ws: WinSize) -> Self {
        WinDim(ws.ws_col, ws.ws_row)
    }
}

impl WinSize {
    fn new() -> WinSize {
        WinSize {
            ws_row: 0,
            ws_col: 0,
            ws_xpixel: 0,
            ws_ypixel: 0
        }
    }
}

extern "C" {
    fn ioctl(fd: c_int, request: c_ulong, ...) -> c_int;
}

pub trait ScreenUpdate {
    fn update(&self);
    fn update_at(&self, row: usize);
}

/*
    TODO: Build view and buffer using the Observer pattern. This way, we decouple some overly complex
        responsibility from the controller (editor.rs), so that when the buffer contents gets updated,
        it will notify the view with an Event, and whatever content that might have been added or removed,
        along with it's "anchor" position (either beginning, for insertion & deletion, or end for removal)
*/
impl EventListener for View {
    fn on_event(&self, evt: Event) {
        match evt {
            Event::INSERTION(_x, _evt_data) => {
                // move blinky cursor x steps and update line
            },
            Event::DELETION(_x, _evt_data) => {
                // move blinky cursor x steps and update line
            },
            Event::REMOVAL(_x, _evt_data) => {

            }
        }
    }
}

pub struct ViewConfig {
    bg_color: SetColor,
    fg_color: SetColor,
    stat_line_color: (SetColor, SetColor)
}

impl Default for ViewConfig {
    fn default() -> Self {
        ViewConfig {
            bg_color: SetColor::Background(Color::Blue),
            fg_color: SetColor::Foreground(Color::White),
            stat_line_color: (SetColor::Background(Color::BrightCyan), SetColor::Background(Color::Black))
        }
    }
}

#[derive(Copy, Clone)]
pub struct ViewCursor {
    pub row: usize,
    pub col: usize
}

impl From<TextPosition> for ViewCursor {
    fn from(tp: TextPosition) -> Self {
        ViewCursor {
            row: tp.line_index + 1,
            col: tp.get_line_position() + 1
        }
    }
}

impl Default for ViewCursor {
    fn default() -> Self {
        ViewCursor { row: 1, col: 1}
    }
}


impl Display for ViewCursor {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        write!(f, "\x1b[{};{}H", self.row, self.col)
    }
}

pub trait ViewRange {
    fn shift_forward(&mut self, len: usize);
    fn shift_backward(&mut self, len: usize);
}

impl ViewRange for std::ops::Range<usize> {
    fn shift_forward(&mut self, len: usize) {
        self.start += len;
        self.end += len;
    }

    fn shift_backward(&mut self, len: usize) {
        if len > self.start {
            panic!("Shifted usize range out of bounds (usize can't be negative)");
        }
        self.start -= len;
        self.end -= len;
    }
}

impl Drop for View {
    fn drop(&mut self) {
        let esc = 27u8;
        // TODO: uncomment last line when bugs are sorted/no major bugs are being found, or comment it out when they are.
        //      if this prints, any error message that might panic! the application, will be lost from the stdout.
        // \x1b[m ends whatever ansi escape sequence currently written to the terminal,
        // and restores to terminal default
        print!("\x1b[m {}[2J{}[1;1H", esc as char, esc as char);
    }
}

pub struct View {
    pub view_cursor: ViewCursor,
    pub statline_view_cursor: ViewCursor,
    pub size: (usize, usize),
    pub line_range: std::ops::Range<usize>,
    pub buffer_ref: Arc<Mutex<Textbuffer>>,
    pub  top_line: TextPosition,
    pub status_line_position: ViewCursor,
    pub win_size: WinDim,
    pub view_cfg: ViewConfig
}

impl View {
    pub fn new() -> Option<Self> {
        let mut v = View {
            view_cursor: ViewCursor{ row: 1, col: 1},
            statline_view_cursor: ViewCursor {row: 1, col: 1},
            size: (0, 0),
            line_range: 0..0,
            buffer_ref: Arc::new(Mutex::new(Textbuffer::new())),
            top_line: TextPosition::new(),
            win_size: WinDim(0, 0),
            status_line_position: ViewCursor { row: 0, col: 0},
            view_cfg: ViewConfig::default()
        };

        if let Some(winsize) = v.get_window_size() {
            v.win_size = WinDim::from(winsize);
            v.status_line_position.row = v.win_size.1 as usize;
            v.statline_view_cursor = v.status_line_position;
            v.line_range = 0..(v.win_size.1 as usize -1);
            Some(v)
        } else {
            None
        }
    }



    pub fn get_text_area_height(&self) -> usize {
        self.win_size.1 as usize - 1
    }

    pub fn init(&mut self) {
        let esc = 27u8;
        print!("{}[2J{}[1;1H", esc as char, esc as char);
        let mut a = " ".repeat(self.win_size.0 as usize);
        a.push('\n');
        a.push('\r');
        let res = a.repeat(self.win_size.1 as usize - 1);
        let mut status = " ".repeat(self.win_size.0 as usize);
        let status_title = "[status]: ";
        self.statline_view_cursor.col = status_title.len() + 1;
        status.replace_range(0..status_title.len(), status_title);
        print!("{}{}{}{}[1;1H",
               self.view_cfg.bg_color.colorize(res.as_ref()),
               self.view_cfg.stat_line_color.1,
               self.view_cfg.stat_line_color.0.colorize(status.as_ref()),
               esc as char);
        stdout().flush();
        self.view_cursor = ViewCursor::default();
        // clear the screen
        // paint the screen with default colors (or color settings provided via .rc file)
        // set up status line
        // paint status line
    }

    pub fn get_window_size(&self) -> Option<WinSize> {
        unsafe {
            let mut window: winsize = zeroed();
            let mut win_size = WinSize::new();
            let result = ioctl(STDOUT_FILENO, TIOCGWINSZ, &mut window);
            if result == -1 {
                window = zeroed();
                let result = ioctl(STDIN_FILENO, TIOCGWINSZ, &mut window);
                if result != -1 {
                    win_size.ws_ypixel = window.ws_ypixel;
                    win_size.ws_xpixel = window.ws_xpixel;
                    win_size.ws_row = window.ws_row;
                    win_size.ws_col = window.ws_col;
                    Some(win_size)
                } else {
                    None
                }
            } else {
                win_size.ws_ypixel = window.ws_ypixel;
                win_size.ws_xpixel = window.ws_xpixel;
                win_size.ws_row = window.ws_row;
                win_size.ws_col = window.ws_col;
                Some(win_size)
            }
        }
    }


    pub fn on_goto(&mut self) {
        let goto_title = "[goto]: ";
        self.statline_view_cursor.col = goto_title.len() + 1;
        print!("{}{}{}{}{}", self.status_line_position, self.view_cfg.stat_line_color.0, self.view_cfg.stat_line_color.1, goto_title, ViewOperations::ClearLineRest);
        stdout().flush();
    }

    pub fn on_open_file(&mut self) {
        let open_title = "[open]: ";
        self.statline_view_cursor.col = open_title.len() + 1;
        print!("{}{}{}{}{}", self.status_line_position, self.view_cfg.stat_line_color.0, self.view_cfg.stat_line_color.1, open_title, ViewOperations::ClearLineRest);
        stdout().flush();
    }

    pub fn statline_error_msg(&mut self, msg: &str) {
        let error_title = "[error]: ";
        self.statline_view_cursor.col = error_title.len() + 1;
        print!("{}{}{}{}{}{}", self.status_line_position, self.view_cfg.stat_line_color.0, self.view_cfg.stat_line_color.1, ViewOperations::ClearLineRest, error_title, msg);
    }

    pub fn on_save_file(&mut self) {
        let open_title = "[save]: ";
        self.statline_view_cursor.col = open_title.len() + 1;
        print!("{}{}{}{}{}", self.status_line_position, self.view_cfg.stat_line_color.0, self.view_cfg.stat_line_color.1, open_title, ViewOperations::ClearLineRest);
        stdout().flush();
    }

    pub fn restore_statline(&mut self) {
        let mut stat = " ".repeat(self.win_size.0 as usize);
        let stat_title = "[status]: ";
        stat.replace_range(0..stat_title.len(), stat_title);
        self.statline_view_cursor.col = stat_title.len() + 1;
        print!("{}{}{}{}\x1b[m{}", self.status_line_position, self.view_cfg.stat_line_color.0, self.view_cfg.stat_line_color.1, stat, self.view_cursor);
        stdout().flush();
    }

    pub fn write_statline_character(&mut self, ch: char) {
        print!("{}{}{}{}", self.statline_view_cursor, self.view_cfg.stat_line_color.0, self.view_cfg.stat_line_color.1, ch);
        stdout().flush();
        self.statline_view_cursor.col += 1;
    }

    pub fn write_statline_line(&self, title: &str, content: &str) {
        let empty = self.win_size.0 as usize - (title.len() + content.len());
        print!("{}{}{}{}{}{}{}", self.status_line_position, ViewOperations::ClearLineRest.as_output(), self.view_cfg.stat_line_color.0, self.view_cfg.stat_line_color.1, title, content, " ".repeat(empty));
        stdout().flush();
    }

    pub fn update_statline_with(&mut self, data: &str, cursor_pos: &ViewCursor) {
        self.statline_view_cursor = *cursor_pos;
        print!("{}{}{}{}{}\r{}", self.status_line_position, ViewOperations::ClearLineRest, self.view_cfg.stat_line_color.0, self.view_cfg.stat_line_color.1, data, cursor_pos);
        stdout().flush();
    }

    pub fn reset(&mut self) {
        let esc: u8 = 27;
        print!("{}[2J{}[1;1H", esc as char, esc as char);
        self.view_cursor = ViewCursor::default();
        let dat = self.buffer_ref.lock().unwrap().dump_to_string();
        for c in dat.chars() {
            self.write_character(c);
        }
        if dat.len() > 0
        {
            self.buffer_ref.lock().unwrap().set_textpos(dat.len());
        }
        // self.view_cursor = ViewCursor { row: pos.get_line_position(), col: pos.line_number + 1 };
        self.update_cursor();
        // stdout().flush();
    }

    pub fn write_character(&mut self, ch: char) {
        // TODO: check if the last character was a whitespace, if so, scan the buffer backwards one word, and check if it should be syntax colored
        if ch == '\n' {
            self.view_cursor.row += 1;
            self.view_cursor.col = 1;
            print!("\r\n");
            stdout().flush();
        } else {
            self.view_cursor.col += 1;
            print!("{}{}{}", self.view_cfg.bg_color, self.view_cfg.fg_color, ch);
            stdout().flush();
        }
    }

    pub fn write_character_buffered(&mut self, ch: char) {
        if ch == '\n' {
            self.view_cursor.row += 1;
            self.view_cursor.col = 1;
            print!("\r\n");
        } else {
            self.view_cursor.col += 1;
            //print!("{}{}{}", self.view_cfg.bg_color, self.view_cfg.fg_color, ch);
            print!("{}", ch);
        }
    }

    pub fn update_with_line(&mut self, data: &str) {
        let empty_space = self.win_size.0 as usize - data.len();
        let mut vc = self.view_cursor;
        vc.col = 1;
        print!("{}{}{}{}{}", vc, ViewOperations::ClearLineRest, self.view_cfg.fg_color, self.view_cfg.bg_color, &data.chars().chain(" ".repeat(empty_space as usize).chars()).collect::<String>());
        stdout().flush();
    }

    pub fn update_cursor(&self) {
        let WinDim(x, _y) = self.win_size;
        let cursor_output_pos = WinDim(x-6, 1);
        let WinDim(valx, valy) = cursor_output_pos;
        let vc_pos = ViewCursor {col: valx as usize, row: valy as usize};
        let vop = ViewOperations::ClearLineRest;
        print!("{}{}{};{}{}", vc_pos, vop.as_output(), self.view_cursor.col, self.view_cursor.row, self.view_cursor);
        stdout().flush();
    }

    pub fn set_viewed_buf(&mut self, buf: Arc<Mutex<Textbuffer>>) {
        let bufref = buf.clone();
        self.buffer_ref = bufref;
    }

    pub fn move_right(&mut self) {

        unimplemented!()
    }

    pub fn move_left(&mut self) {
        unimplemented!()
    }

    pub fn move_up(&mut self) {
        unimplemented!()
    }

    pub fn move_down(&mut self) {
        if self.view_cursor.row == self.line_range.end {

        }
    }
    // using the Rc<RefCell> is needed for being able to register with the text buffer, this view is watching.
    // It is kind of annoying, since the only thing we ever mutate, is the actual observer/listener at the buffer.
    // A view, never, _never_ mutates the buffer data.
    pub fn register_as_listener(&mut self) {

    }

    pub fn draw_cursor(&self) {
        print!("{}", self.view_cursor);
    }

    pub fn scroll_up(&mut self) {
        let shift = self.get_text_area_height() / 2;
        if shift > self.line_range.start && self.line_range.start != 0{
            self.line_range.shift_backward(self.line_range.start);
            self.top_line = self.buffer_ref.lock().unwrap().get_line_end_pos_0_idx(self.line_range.start).unwrap();
        } else if shift <= self.line_range.start {
            self.line_range.shift_backward(shift);
            self.top_line = self.buffer_ref.lock().unwrap().get_line_end_pos_0_idx(self.line_range.start).unwrap();
        }
    }

    pub fn scroll_down(&mut self) {
        let shift = self.get_text_area_height() / 2;
        self.line_range.shift_forward(shift);
        self.top_line = self.buffer_ref.lock().unwrap().get_line_end_pos_0_idx(self.line_range.start).unwrap();
    }

    pub fn check_at_boundary_cross(&mut self) {
        let tp = self.buffer_ref.lock().unwrap().get_textpos();
        self.view_cursor = ViewCursor::from(tp.clone());
        if self.view_cursor.row >= self.line_range.end && tp.line_index >= self.line_range.end {
            let diff = self.view_cursor.row - self.line_range.end;
            let begin = self.line_range.start;
            let end = self.line_range.end;
            self.line_range = (begin + diff)..(end + diff);
            self.top_line = self.buffer_ref.lock().unwrap().get_line_end_pos_0_idx(self.line_range.start).unwrap();
            // TODO: Remove this when you are 1000000000% certain scrolling functionality works. This fucking bullshit took me 2 days to get right.
            // debug_sleep(Some(format!("Moving after range.. abs_begin: {}", self.top_line.line_start_absolute)), Some(2500));
            let bottom = self.buffer_ref.lock().unwrap().get_line_end_pos(self.line_range.end).unwrap();
            // self.view_cursor = ViewCursor::from(bottom);
            if self.view_cursor.row >= self.win_size.1 as usize - 1 {
                let win_curs_diff = self.view_cursor.row - self.win_size.1 as usize;
                self.view_cursor.row -= (self.line_range.start);
                // self.view_cursor.row -= win_curs_diff;
            }
        } else if tp.line_index < self.line_range.start && self.view_cursor.row != 0 {
            let diff = self.line_range.start - tp.line_index;
            // let diff = (self.line_range.start) - (self.view_cursor.row);
            let begin = self.line_range.start;
            let end = self.line_range.end;
            self.line_range = (begin -diff)..(end - diff);
            self.top_line = self.buffer_ref.lock().unwrap().get_line_end_pos_0_idx(self.line_range.start).unwrap();
            // TODO: Remove this when you are 1000000000% certain scrolling functionality works. This fucking bullshit took me 2 days to get right.
            // debug_sleep(Some(format!("Moving before range.. abs_begin: {}", self.top_line.line_start_absolute)), Some(2500));
        } else if (self.view_cursor.row) <= self.line_range.start+1 && self.line_range.start == 0 {
            let line_pos = self.buffer_ref.lock().unwrap().get_textpos();
            // debug_sleep(Some(format!("We are trying to move at the topline: {}", line_pos.clone().absolute)), Some(2500));
            self.view_cursor = ViewCursor::from(line_pos);
        } else {
            self.view_cursor = ViewCursor::from(self.buffer_ref.lock().unwrap().get_textpos());
            self.view_cursor.row -= self.line_range.start;
        }
    }

    pub fn draw_view(&mut self) {
        self.check_at_boundary_cross();
        let tmp = self.view_cursor;
        let abs_begin = self.top_line.line_start_absolute; // and "anchor" into the buffer, so that we know where the top line of the view -> buffer is
        let line_count = self.win_size.1 - 1;
        let d = {
            let guard = self.buffer_ref.lock().unwrap();
            let line_end_abs = guard.get_line_abs_end_index(self.top_line.line_index + self.line_range.len()+1).unwrap().absolute;
            let d = guard.get_data_range(abs_begin, line_end_abs);
            let esc = 27u8;
            print!("{}[2J{}[1;1H", esc as char, esc as char);
            let mut a = " ".repeat(self.win_size.0 as usize);
            a.push('\n');
            a.push('\r');
            let res = a.repeat(self.win_size.1 as usize + 1);

            let STATUS_TITLE = "[status]: ";
            let fullstat = STATUS_TITLE.chars().chain(" ".repeat(self.win_size.0 as usize - STATUS_TITLE.len()).chars());

            let mut status = " ".repeat(self.win_size.0 as usize);
            let status_title = "[status]: ";
            self.statline_view_cursor.col = status_title.len() + 1;
            status.replace_range(0..status_title.len(), status_title);
            // let t = self.view_cfg.bg_color.colorize(res.as_ref()).chars().chain("\x1b[1;1H".chars()).collect::<&str>();
            print!("\x1b[2J\x1b[1;1H{}{}[1;1H",
                   self.view_cfg.bg_color.colorize(res.as_ref()),
                   esc as char);
            self.view_cursor = ViewCursor::default();
            // clear the screen
            // paint the screen with default colors (or color settings provided via .rc file)
            // set up status line
            // paint status line
            d
        };
        print!("{}{}", self.view_cfg.bg_color, self.view_cfg.fg_color);
        for c in d.chars() {
            self.write_character_buffered(c);
        }
        // self.view_cursor = ViewCursor::from(self.buffer_ref.lock().unwrap().get_textpos());
        self.view_cursor = tmp;
        // self.fix_range_old();
        // print!("{}", self.view_cursor);
        self.restore_statline();
        // stdout().flush();
        /* TODO: get data from buffer in the range of top_line .. (top_line + (winsize.x * winsize.y)
            scan content, for new lines, and filter out any newlines that won't fit on screen
            i.e, newline count > winsize.y. newline count being, find newlines between absolute
            positions top_line.absolute to (top_line + (winsize.x * winsize.y), then any newline with
            index higher than (top_line.line_number + winsize.y)
        */
    }
}

