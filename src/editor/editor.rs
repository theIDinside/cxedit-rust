use crate::editor::view::View;
use std::path::Path;
use crate::data::textbuffer::Textbuffer;
use std::sync::{Arc, Mutex};
use std::os::unix::io::RawFd;
use termios::Termios;
use termios::tcsetattr;
use std::str::from_utf8;

use libc::read as read;

pub enum KeyCode {
    CtrlBackspace,
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



impl From<i8> for KeyCode {
    fn from(v: i8) -> Self {
        match v {
            9 => KeyCode::Tab,
            10 => KeyCode::Enter,
            27 => KeyCode::Esc,
            127 => KeyCode::Backspace,
            c @ _ if 32 <= c || c < 127 => KeyCode::Character((c as u8) as char),
            _ => {
                println!("Could not convert value to keycode");
                KeyCode::None
            }
        }
    }
}

pub struct Editor {
    buffers: Vec<Arc<Mutex<Textbuffer>>>,
    views: Vec<View>,
    current_view: usize,
    current_buffer: usize,
    running: bool,
    original_terminal_settings: Option<Termios>
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


impl Editor {
    pub fn new() -> Editor {
        Editor {
            buffers: vec![],
            views: vec![],
            current_buffer: 0,
            current_view: 0,
            running: false,
            original_terminal_settings: None
        }
    }

    pub fn init(&mut self, settings: Option<Termios>) {
        self.original_terminal_settings = settings;
        self.buffers.push(Arc::new(Mutex::new(Textbuffer::new())));
        let mut v = View::new();
        v.set_viewed_buf(self.buffers[0].clone());
        self.running = true;
    }

    pub fn open(&mut self, f: &Path) {
        let mut rcBuf = Arc::new(Mutex::new(Textbuffer::from_file(f.to_str().unwrap().to_string())));
        if self.views.len() == 0 {
            let mut v = View::new();
            v.set_viewed_buf(rcBuf);
            self.views.push(v);
        }
    }

    pub fn run(&mut self) {
        // TODO: setup code, and also
        let esc = 27u8;
        print!("{}[2J{}[1;1H", esc as char, esc as char);
        println!("Entering editor main loop:\r");
        while self.running {
            match self.handle_keypress() {
                KeyCode::Character(c) => { self.buffers[0].lock().unwrap().insert_ch(c) },
                KeyCode::Backspace => self.buffers[0].lock().unwrap().remove(),
                KeyCode::Tab => self.buffers[0].lock().unwrap().insert_data("    ".into()),
                KeyCode::Enter => self.buffers[0].lock().unwrap().insert_ch('\n'),
                KeyCode::Esc => {},
                KeyCode::CtrlBackspace => {},
                KeyCode::CtrlQ => {
                    println!("Buffer content: {} \r\nQuitting...\r\n", self.buffers[0].lock().unwrap().dump_to_string());
                    self.running = false;
                },
                KeyCode::Escaped(_esk) => {},
                KeyCode::None => println!("Could not handle keypress!\r")
            }
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
                8 => KeyCode::CtrlBackspace,
                9 => KeyCode::Tab,
                13 => KeyCode::Enter,
                17 => KeyCode::CtrlQ,
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