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
use crate::editor::view::ViewOperations::ClearLineRest;

#[derive(Debug)]
pub struct WinDim(pub u16, pub u16);

pub enum ViewOperations {
    ClearLineRest,
}

impl ViewOperations {
    fn as_output(&self) -> &str {
        match self {
            ViewOperations::ClearLineRest => "\x1b[0K"
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

#[derive(Copy)]
pub struct ViewCursor {
    pub row: usize,
    pub col: usize
}

impl Default for ViewCursor {
    fn default() -> Self {
        ViewCursor { row: 1, col: 1}
    }
}

impl Clone for ViewCursor {
    fn clone(&self) -> Self {
        ViewCursor { row: self.row, col: self.col }
    }
}

impl Display for ViewCursor {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        write!(f, "\x1b[{};{}H", self.row, self.col)
    }
}

impl Drop for View {
    fn drop(&mut self) {
        let esc = 27u8;
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
            Some(v)
        } else {
            None
        }
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
        print!("{}{}{}[1;1H",
               self.view_cfg.bg_color.colorize(res.as_ref()),
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

    pub fn on_open_file(&mut self) {
        let open_title = "[open]: ";
        self.statline_view_cursor.col = open_title.len() + 1;
        print!("{}{}{}{}{}", self.status_line_position, self.view_cfg.stat_line_color.0, self.view_cfg.stat_line_color.1, open_title, ViewOperations::ClearLineRest.as_output());
        stdout().flush();
    }

    pub fn on_save_file(&mut self) {
        let open_title = "[save]: ";
        self.statline_view_cursor.col = open_title.len() + 1;
        print!("{}{}{}{}{}", self.status_line_position, self.view_cfg.stat_line_color.0, self.view_cfg.stat_line_color.1, open_title, ViewOperations::ClearLineRest.as_output());
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
        print!("{}{}{}{}{}{}", self.status_line_position, ViewOperations::ClearLineRest.as_output(), self.view_cfg.stat_line_color.0, self.view_cfg.stat_line_color.1, title, content);
        stdout().flush();
    }

    pub fn update_statline_with(&mut self, data: &str) {
        print!("{}{}{}{}\r", self.status_line_position, self.view_cfg.stat_line_color.0, self.view_cfg.stat_line_color.1, data);
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
        self.buffer_ref.lock().unwrap().set_textpos(dat.len() - 1);
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

    pub fn update_with_line(&mut self, data: &str) {
        let empty_space = self.win_size.0 as usize - data.len();
        let mut vc = self.view_cursor;
        vc.col = 1;
        print!("{}{}{}{}{}", vc, ViewOperations::ClearLineRest.as_output(), self.view_cfg.fg_color, self.view_cfg.bg_color, &data.chars().chain(" ".repeat(empty_space as usize).chars()).collect::<String>());
        stdout().flush();
    }

    pub fn update_cursor(&self) {
        let WinDim(x, _y) = self.win_size;
        let cursor_output_pos = WinDim(x-6, 1);
        let WinDim(valx, valy) = cursor_output_pos;
        let vc_pos = ViewCursor {col: valx as usize, row: valy as usize};
        let vop = ViewOperations::ClearLineRest;
        print!("{}{}{};{}", vc_pos, vop.as_output(), self.view_cursor.col, self.view_cursor.row);
        print!("{}", self.view_cursor);
        stdout().flush();
    }

    pub fn set_viewed_buf(&mut self, buf: Arc<Mutex<Textbuffer>>) {
        let bufref = buf.clone();
        self.buffer_ref = bufref;
    }

    pub fn move_right(&mut self, _steps: usize) {
        unimplemented!()
    }

    pub fn move_left(&mut self, _steps: usize) {
        unimplemented!()
    }

    pub fn move_up(&mut self, _steps: usize) {
        unimplemented!()
    }

    pub fn move_down(&mut self, _steps: usize) {
        let begin = self.line_range.start + 1;
        let end = self.line_range.end + 1;
        self.line_range = begin .. end;
    }

    // using the Rc<RefCell> is needed for being able to register with the text buffer, this view is watching.
    // It is kind of annoying, since the only thing we ever mutate, is the actual observer/listener at the buffer.
    // A view, never, _never_ mutates the buffer data.
    pub fn register_as_listener(&mut self) {

    }

    pub fn draw_view(&self) {
        /* TODO: get data from buffer in the range of top_line .. (top_line + (winsize.x * winsize.y)
            scan content, for new lines, and filter out any newlines that won't fit on screen
            i.e, newline count > winsize.y. newline count being, find newlines between absolute
            positions top_line.absolute to (top_line + (winsize.x * winsize.y), then any newline with
            index higher than (top_line.line_number + winsize.y)
        */
    }
}

