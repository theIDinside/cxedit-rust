use crate::editor::view::View;
use std::path::Path;
use crate::data::text_buffer::Textbuffer;
use std::sync::{Arc, Mutex};
use std::os::unix::io::RawFd;
use termios::Termios;
use termios::tcsetattr;
use std::str::from_utf8;
use std::fs::{read_to_string};
use libc::read as read;
use std::io::stdout;
use std::io::{Write};
use crate::cmd::MoveKind;
use crate::cmd::MoveDir;
use crate::editor::view::ViewCursor;
pub enum KeyCode {
    CtrlBackspace,
    CtrlA,
    CtrlB,
    CtrlS,
    CtrlO,
    CtrlQ,
    Enter,
    Tab,
    Esc,
    Backspace,
    Character(char),
    Escaped(EscapeKeyCode),
    None,
}
pub enum EscapeKeyCode {
    Left,
    Right,
    Up,
    Down
}

pub enum InputMode {
    StatLine,
    Document
}

pub struct Editor {
    buffers: Vec<Arc<Mutex<Textbuffer>>>,
    views: Vec<View>,
    current_view: usize,
    current_buffer: usize,
    running: bool,
    original_terminal_settings: Option<Termios>,
    input_mode: InputMode,
}

impl Drop for Editor {
    fn drop(&mut self) {
        if let Some(settings) = self.original_terminal_settings {
            match tcsetattr(0, libc::TCSANOW, &settings) {
                Ok(_) => {
                    println!("Restored terminal settings");
                },
                Err(e) => {
                    println!("Some error occurred! {}", e);
                }
            }
        }
    }
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

pub enum StatlineCommand {
    OpenFile(String)
}

impl Editor {
    pub fn new() -> Editor {
        Editor {
            buffers: vec![],
            views: vec![],
            current_buffer: 0,
            current_view: 0,
            running: false,
            original_terminal_settings: None,
            input_mode: InputMode::Document
        }
    }

    pub fn init(&mut self, settings: Option<Termios>) {
        self.original_terminal_settings = settings;
        self.buffers.push(Arc::new(Mutex::new(Textbuffer::new())));
        let mut v = View::new().unwrap_or_else(|| View::new().unwrap());
        v.set_viewed_buf(self.buffers[0].clone());
        v.init();
        self.views.push(v);
        self.running = true;
    }

    pub fn open(&mut self, f: &Path) {
        let rc_buf = Arc::new(Mutex::new(Textbuffer::from_file(f.to_str().unwrap().to_string())));
        if self.views.len() == 0 {
            let mut v = View::new().unwrap_or_else(|| View::new().unwrap());
            v.set_viewed_buf(rc_buf);
            self.views.push(v);
        } else {
            self.views[self.current_view].set_viewed_buf(rc_buf);
        }
    }

    pub fn statline_input(&mut self) -> Option<StatlineCommand> {
        let mut input = String::new();
        loop {
            match self.handle_keypress() {
                KeyCode::Character(ch) => {
                    input.push(ch);
                    self.views[self.current_view].write_statline_character(ch);
                },
                KeyCode::Backspace => {
                    input.pop();
                    let mut old_content = String::from("[open]: ");
                    old_content.push_str(&input);
                    self.views[self.current_view].update_statline_with(&old_content);
                }
                KeyCode::Enter => {
                    return Some(StatlineCommand::OpenFile(input.clone()));
                },
                KeyCode::Esc => {
                    return None;
                },
                _ => {
                    // any other input than enter, a character key or escape, is invalid at this point and will do nothing.
                }
            }
        }
    }

    pub fn get_view(&mut self) -> &mut View {
        &mut self.views[self.current_view]
    }

    pub fn run(&mut self) {
        // TODO: setup code, and also
        // println!("Entering editor main loop:\r");
        while self.running {
            match self.handle_keypress() {
                KeyCode::Character(c) => {
                    self.buffers[0].lock().unwrap().insert_ch(c);
                    self.views[self.current_view].write_character(c);
                },
                KeyCode::Backspace => {
                    let line_number = self.buffers[0].lock().unwrap().get_line_number_editing();
                    self.buffers[0].lock().unwrap().remove();
                    if line_number < self.buffers[0].lock().unwrap().get_line_number_editing() {
                        // TODO: Redraw entire screen, because removing a line, will alter positions of every line after it
                    }
                    let line = self.buffers[0].lock().unwrap().get_line_at_cursor();
                    self.views[0].view_cursor.col = 1;
                    self.views[0].update_with_line(&line);
                },
                KeyCode::Tab => self.buffers[0].lock().unwrap().insert_data("    ".into()),
                KeyCode::Enter => {
                    self.buffers[0].lock().unwrap().insert_ch('\n');
                    self.views[self.current_view].write_character('\n');
                },
                KeyCode::Esc => {},
                KeyCode::CtrlBackspace => {},
                KeyCode::CtrlS => {
                    /* TODO: open status line if we do not have a filename, write in filename
                            validate provided path, open a new file with that name -> write contents.
                            reset statusline.
                    */
                }
                KeyCode::CtrlO => {
                    self.views[self.current_view].on_open_file();
                    let cmd = self.statline_input();
                    if let Some(StatlineCommand::OpenFile(fname)) = cmd {
                        match read_to_string(Path::new(&fname)) {
                            Ok(data) => {
                                let lines_to_print = self.get_view().win_size.1 - 1;
                                let mut lindex = 0;
                                self.buffers[self.current_buffer].lock().unwrap().clear_buffer_contents();
                                self.buffers[self.current_buffer].lock().unwrap().insert_data(&data);
                                let d_to_print = data.chars().take_while(|ch| {
                                    if *ch == '\n' {
                                        lindex += 1;
                                    }
                                    lindex < lines_to_print
                                }).collect::<String>();
                                self.views[self.current_view].init();
                                d_to_print.chars().for_each(|c| {
                                    self.views[self.current_view].write_character(c);
                                });
                                self.buffers[self.current_buffer].lock().unwrap().set_textpos(0);
                                self.views[self.current_view].view_cursor = ViewCursor::default();
                            },
                            Err(_e) => {

                            }
                        }
                    }
                    self.views[self.current_view].restore_statline();
                },
                KeyCode::CtrlA => {
                    let tp = self.buffers[self.current_buffer].lock().unwrap().get_textpos();
                    print!("Text buffer position: absolute: {}, line_start_absolute: {}, line_number: {}, line column position: {}\r\n", tp.absolute, tp.line_start_absolute, tp.line_number, tp.get_line_position());
                    print!("Text buffer get line at buffer cursor: {}\r\n", self.buffers[self.current_buffer].lock().unwrap().get_line_number());
                    print!("View cursor position: {},{}", self.views[self.current_view].view_cursor.col, self.views[self.current_view].view_cursor.row);
                    stdout().flush();
                    self.views[self.current_view].update_cursor();
                },
                KeyCode::CtrlB => {
                    self.views[self.current_view].reset();
                },
                KeyCode::CtrlQ => {
                    println!("Buffer content: {} \r\nQuitting...\r\n", self.buffers[0].lock().unwrap().dump_to_string());
                    self.running = false;
                },
                KeyCode::Escaped(_esk) => {
                    // println!("Trying to move cursor...");
                    match _esk {
                        EscapeKeyCode::Right => {
                            let old_pos = self.buffers[self.current_buffer].lock().unwrap().get_textpos();
                            let pos = self.buffers[self.current_buffer].lock().unwrap().move_cursor(MoveKind::Char(MoveDir::Next)).unwrap();
                            if old_pos != pos {
                                if pos.line_number > old_pos.line_number {
                                    println!("Trying to move right...");
                                    self.views[self.current_view].view_cursor.row += 1;
                                    self.views[self.current_view].view_cursor.col = 1;
                                } else {
                                    self.views[self.current_view].view_cursor.col += 1;
                                }
                                self.views[self.current_view].update_cursor();
                            }
                        },
                        EscapeKeyCode::Left => {
                            let old_pos = self.buffers[self.current_buffer].lock().unwrap().get_textpos();
                            let pos = self.buffers[self.current_buffer].lock().unwrap().move_cursor(MoveKind::Char(MoveDir::Previous)).unwrap();
                            if old_pos != pos {
                                if pos.line_number < old_pos.line_number {
                                    self.views[self.current_view].view_cursor.row -= 1;
                                    self.views[self.current_view].view_cursor.col = pos.get_line_position() + 1;
                                } else {
                                    if self.views[self.current_view].view_cursor.col > 1 {
                                        self.views[self.current_view].view_cursor.col -= 1;
                                    }
                                }
                                self.views[self.current_view].update_cursor();
                            }
                        }
                        EscapeKeyCode::Up => {
                            self.buffers[self.current_buffer].lock().unwrap().move_cursor(MoveKind::Line(MoveDir::Previous));
                        }
                        EscapeKeyCode::Down => {
                            self.buffers[self.current_buffer].lock().unwrap().move_cursor(MoveKind::Line(MoveDir::Next));
                        }
                    }
                    // print!("{}", _esk.output());stdout().lock().flush();
                },
                KeyCode::None => println!("Could not handle keypress!\r")
            }
            self.views[self.current_view].draw_view();
        }
    }

    pub fn interpret_input_sequence(&self) -> KeyCode {
        use libc::STDIN_FILENO;
        let res = unsafe {
            let mut ch_seq: [u8; 3] = [0,0,0];
            let kc = if read(STDIN_FILENO, ch_seq.as_mut_ptr() as *mut libc::c_void, 1) == -1 {
            // here, we are saying "we have not read anything more than 27 (Escape keycode) and/or there was an error reading the next
            // part of the sequence, therefore -> return as if only "escape" was pressed on the keyboard.
                KeyCode::Esc
            } else if read(STDIN_FILENO, ch_seq.as_mut_ptr().offset(1) as *mut libc::c_void, 1) == -1 {
                KeyCode::Esc // same here
            } else {
            // we successfully read more bytes in sequence after the escape keycode was found. Interpret what that sequence means.
                if ch_seq[0] as char == '[' {
                    match ch_seq[1] as char {
                        'A' =>  {
                            // up
                            KeyCode::Escaped(EscapeKeyCode::Up)
                        },
                        'B' =>  {
                            // down
                            KeyCode::Escaped(EscapeKeyCode::Down)
                        },
                        'C' =>  {
                            // right
                            KeyCode::Escaped(EscapeKeyCode::Right)
                        },
                        'D' =>  {
                            // left
                            KeyCode::Escaped(EscapeKeyCode::Left)
                        },
                        _ => {
                            KeyCode::None // handled for not yet implemented key sequences
                        }
                    }
                } else {
                    KeyCode::None
                }
            };
            kc
        };
        res
    }

    /// This function will not be able to handle any character keys other than English to begin with.
    /// Since this will be a text editor meant solely for programming, it doesn't feel necessary either...
    /// no sane compiler will compile variables with names that aren't in english.

    pub fn handle_keypress(&mut self) -> KeyCode {
        use libc::STDIN_FILENO;
        // :read from keyboard
        //   if input == some key sequence (e.g. CTRL+S) -> create command
        //   else if input == some char -> create operation(data)
        // let mut c: u8;
        let mut ch_buf: [u8; 3] = [0,0,0];
        unsafe {
            while read(STDIN_FILENO, ch_buf.as_mut_ptr() as *mut libc::c_void, 1) == 0 {}
            let input = match ch_buf[0] {
                1 => KeyCode::CtrlA,
                2 => KeyCode::CtrlB,
                8 => KeyCode::CtrlBackspace,
                9 => KeyCode::Tab,
                13 => KeyCode::Enter,
                15 => KeyCode::CtrlO,
                17 => KeyCode::CtrlQ,
                19 => KeyCode::CtrlS,
                27 => {
                    self.interpret_input_sequence()
                },
                195 => {
                    if read(STDIN_FILENO, ch_buf.as_mut_ptr().offset(1) as *mut libc::c_void, 1) < 1 {
                        KeyCode::None
                    } else {
                        KeyCode::Character(from_utf8(&ch_buf[..]).unwrap().chars().take(1).collect::<Vec<char>>()[0])
                    }
                },
                127 => KeyCode::Backspace,
                c @ _ if c >= 32 || c < 127 => {
                    KeyCode::Character(c as char)
                },
                c @ _ if c > 127 => {
                    KeyCode::None
                },
                _ => {
                    KeyCode::None
                }
            };
            input
        }
    }

    pub fn setup_rawmode(&mut self, fd: RawFd) -> std::io::Result<Termios> {
        let mut term = Termios::from_fd(fd)?;
        let original_setting = Termios::from_fd(0).unwrap();

        term.c_iflag &= !(libc::BRKINT | libc::ICRNL | libc::INPCK | libc::ISTRIP | libc::IXON);
        term.c_oflag &= !(libc::OPOST);
        term.c_cflag |= libc::CS8 | libc::CREAD | libc::CLOCAL;
        term.c_lflag &= !(libc::ECHO | libc::ICANON | libc::IEXTEN | libc::ISIG);

        term.c_cc[libc::VMIN] = 0;
        term.c_cc[libc::VTIME] = 1;
        term.c_lflag = 0;


        termios::cfsetspeed(&mut term, libc::B9600)?;
        termios::tcsetattr(fd, libc::TCSANOW, &term)?;
        termios::tcflush(fd, libc::TCIOFLUSH)?;
        self.original_terminal_settings = Some(original_setting);
        Ok(original_setting)
    }
}