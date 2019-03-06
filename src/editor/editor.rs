use std::path::Path;
use crate::data::text_buffer::Textbuffer;
use std::sync::{Arc, Mutex};
use std::os::unix::io::RawFd;
use termios::{Termios, tcsetattr};
use std::str::from_utf8;
use std::fs::{read_to_string};
use libc::read as read;
use std::io::{Write, stdout};
use crate::cmd::{MoveKind, MoveDir, Command};
use crate::editor::{view::ViewCursor, view::View};
use self::StatlineCommand::{SaveFile};
use crate::editor::view::ViewOperations;
use crate::cfg::Config;
use crate::editor::key::{KeyCode, EscapeKeyCode};
use crate::cmd::command_engine::CommandEngine;
use crate::cmd::command_engine::Action;
use crate::cmd::command_engine::ActionResult;
use crate::editor::view::WinDim;

pub enum InputMode {
    Insert,
    Movement,
    Command
}

pub struct Editor {
    buffers: Vec<Arc<Mutex<Textbuffer>>>,
    views: Vec<View>,
    current_view: usize,
    current_buffer: usize,
    running: bool,
    original_terminal_settings: Option<Termios>,
    _input_mode: InputMode,
    config: Config,
    c_e: CommandEngine
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

#[derive(Clone)]
pub enum StatlineCommand {
    OpenFile(Option<String>),
    SaveFile(Option<String>)
}

impl Editor {
    pub fn new() -> Editor {
        Editor {
            buffers: vec![Arc::new(Mutex::new(Textbuffer::new()))],
            views: vec![],
            current_buffer: 0,
            current_view: 0,
            running: false,
            original_terminal_settings: None,
            _input_mode: InputMode::Insert,
            config: Config::default(),
            c_e: CommandEngine::new(Arc::new(Mutex::new(Textbuffer::new())))
        }
    }

    pub fn init(&mut self, settings: Option<Termios>) {
        self.original_terminal_settings = settings;
        if self.buffers.len() != 0 {
            self.buffers.push(Arc::new(Mutex::new(Textbuffer::new())));
        }
        let mut v = View::new().unwrap_or_else(|| View::new().unwrap());
        v.set_viewed_buf(self.buffers[0].clone());
        v.init();
        self.views.push(v);
        self.running = true;
        self.c_e.register_buffer(self.buffers[0].clone());
    }

    pub fn open(&mut self, f: &Path) {
        if self.views.len() == 0 {

        } else {
            match read_to_string(f) {
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
                    self.views[0].write_statline_line("[open]: ", &format!("successfully opened {}", f.display()))
                },
                Err(_e) => {

                }
            }
        }
    }

    pub fn statline_input(&mut self, cmd: Command) -> Option<StatlineCommand> {
        let mut input = String::new();
        let stat_line_title = String::from(&cmd);
        let title_len = stat_line_title.len();
        let mut vc = self.views[0].status_line_position;
        vc.col = title_len + 1usize;
        let mut buf_index = 0;
        loop {
            match self.handle_keypress() {
                KeyCode::Character(ch) => {
                    vc.col += 1;
                    if buf_index == input.len() {
                        input.push(ch);
                        self.views[self.current_view].write_statline_character(ch);
                    } else {
                        self.views[self.current_view].statline_view_cursor = vc;
                        input.insert(buf_index, ch);
                        let old_content = stat_line_title.clone().chars().chain(input.chars()).collect::<String>();
                        self.views[self.current_view].update_statline_with(&old_content, &vc);
                    }
                    buf_index += 1;
                },
                KeyCode::Backspace => {
                    if buf_index == input.len() && buf_index > 0 {
                        input.pop();
                        buf_index -= 1;
                        vc.col -= 1;
                    } else if buf_index > 0 {
                        input.remove(buf_index-1);
                        buf_index -= 1;
                        vc.col -= 1;
                    }
                    let old_content = stat_line_title.clone().chars().chain(input.chars()).collect::<String>();
                    self.views[self.current_view].update_statline_with(&old_content, &vc);
                }
                KeyCode::Enter => {
                    return match cmd {
                        Command::Open => {
                            Some(StatlineCommand::OpenFile(Some(input.clone())))
                        },
                        Command::Save => {
                            Some(StatlineCommand::SaveFile(Some(input.clone())))
                        },
                        Command::Find => unimplemented!(),
                        Command::Jump => unimplemented!(),
                        Command::Move(_mk) => unimplemented!(),
                        Command::Quit => {
                            unimplemented!();
                        },
                        Command::CommandInput => {
                            unimplemented!();
                        }
                    };
                },
                KeyCode::Escaped(esc_kc) => {
                    match esc_kc {
                        EscapeKeyCode::Down => {    // TODO: perform command history scroll down

                        },
                        EscapeKeyCode::Up => {      // TODO: perform command history scroll up

                        },
                        EscapeKeyCode::Right => {   // TODO: perform step right on status line
                            if buf_index < input.len() {
                                print!("{}", ViewOperations::StepRight);
                                buf_index += 1;
                                vc.col += 1;
                                stdout().flush();
                            }
                        },
                        EscapeKeyCode::Left => {    // TODO: perform step left on status line
                            if buf_index > 0 {
                                print!("{}", ViewOperations::StepLeft);
                                buf_index -= 1;
                                vc.col -= 1;
                                stdout().flush();
                            }
                        },
                    }
                }
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

    // TODO: split up run function, to remove the intense complexity and spaghettization of code
    pub fn run(&mut self) {
        // TODO: setup code, and also
        // println!("Entering editor main loop:\r");
        let mut running = self.running;
        while running {
            let kp = self.handle_keypress();
            self.views[0].restore_statline();
            match kp {
                KeyCode::Character(c) => {
                    let pos = *&self.buffers[0].lock().unwrap().get_textpos().absolute;
                    match self.c_e.execute(Action::Insert(pos, c)) {
                        ActionResult::OK => {
                            self.views[self.current_view].draw_view();
                        },
                        ActionResult::ERR => {

                        }
                    }
                    // self.buffers[0].lock().unwrap().insert_ch(c);
                },
                KeyCode::Enter => {
                    let pos = *&self.buffers[0].lock().unwrap().get_textpos().absolute;
                    match self.c_e.execute(Action::Insert(pos, '\n')) {
                        ActionResult::OK => {
                            self.views[0].draw_view();
                        },
                        ActionResult::ERR => {

                        }
                    }
                },
                KeyCode::Backspace => {
                    let pos = {
                        let guard = self.buffers[0].lock().unwrap();
                        guard.get_textpos().absolute
                    };
                    if pos > 0 {
                        let c = self.buffers[0].lock().unwrap().get_at(pos-1).unwrap();
                        match self.c_e.execute(Action::Remove(pos, c)) {
                            ActionResult::OK => {
                            self.views[0].draw_view();
                            },
                            ActionResult::ERR => {

                            }
                        }
                    }
                },
                KeyCode::Tab => {
                    let pos = *&self.buffers[0].lock().unwrap().get_textpos().absolute;
                    match self.c_e.execute(Action::InsertData(pos, "    ".into())) {
                        ActionResult::OK => {
                            self.views[self.current_view].draw_view();
                        },
                        ActionResult::ERR => {

                        }
                    }
                    /*
                    self.buffers[0].lock().unwrap().insert_data("    ");
                    for _ in 0..4 {
                        self.views[0].write_character(' ');
                    }*/
                },
                KeyCode::Esc => {},
                KeyCode::CtrlBackspace => {},
                KeyCode::CtrlS => {
                    /* TODO: open status line if we do not have a filename, write in filename
                            validate provided path, open a new file with that name -> write contents.
                            reset statusline.
                    */
                    // TODO: implement 2 functions, one that will write without asking for new file name/ask if ok, and one that does
                    // TODO: implement Config for editor. Then request mapping of key to command via self.get_keybinding(KeyCode::CtrlS)
                    self.views[0].on_save_file();
                    let cmd = self.statline_input(Command::Save);
                    if let Some(SaveFile(Some(suggested_fname))) = cmd {
                        let p = Path::new(&suggested_fname);
                        match self.buffers[0].lock().unwrap().save_to_file(p, None) {
                            Ok(file_size) => {
                                self.views[0].write_statline_line("[saved]: ", suggested_fname.chars().chain(" successfully! Size: ".chars()).chain(file_size.to_string().chars()).collect::<String>().as_ref());
                            },
                            Err(e) => {
                                self.views[0].write_statline_line("[error]: ", &format!("{}", e))
                            }
                        }
                    } else {
                        self.views[0].restore_statline();
                    }
                    self.buffers[0].lock().unwrap().set_pristine();
                },
                KeyCode::CtrlO => {
                    self.views[self.current_view].on_open_file();
                    let cmd = self.statline_input(Command::Open);
                    if let Some(StatlineCommand::OpenFile(Some(fname))) = cmd {
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
                                self.views[0].write_statline_line("[open]: ", &format!("successfully opened {}", fname))
                            },
                            Err(_e) => {

                            }
                        }
                    } else if let None = cmd {
                        self.views[0].restore_statline();
                    }
                },
                KeyCode::CtrlZ => {
                    match self.c_e.execute(Action::Undo) {
                        ActionResult::OK => {
                            self.views[0].draw_view();
                        },
                        ActionResult::ERR => {

                        }
                    }
                },
                KeyCode::CtrlA => {
                    // N.B! This is a debug function ONLY. Used in the beginning for testing display functions, cursor navigation etc
                    // This will become something else entirely.
                    let tp = self.buffers[self.current_buffer].lock().unwrap().get_textpos();
                    print!("Text buffer position: absolute: {}, line_start_absolute: {}, line_number: {}, line column position: {}\r\n", tp.absolute, tp.line_start_absolute, tp.line_number, tp.get_line_position());
                    print!("Text buffer get line at buffer cursor: {}\r\n", self.buffers[self.current_buffer].lock().unwrap().get_line_number());
                    print!("View cursor position: {},{}", self.views[self.current_view].view_cursor.col, self.views[self.current_view].view_cursor.row);
                    stdout().flush();

                    self.views[self.current_view].update_cursor();
                },
                KeyCode::CtrlB => {
                    self.views[self.current_view].draw_view();
                },
                KeyCode::CtrlC => {
                    // TODO: this is how reading from our config will look like, so that the bindings can be customizable
                    let _command: Option<&Command> = self.config.get_binding(KeyCode::CtrlC);
                },
                KeyCode::CtrlQ => {
                    running = false;
                },
                KeyCode::Escaped(_esk) => {
                    match _esk {
                        EscapeKeyCode::Right => {
                            let old_pos = self.buffers[self.current_buffer].lock().unwrap().get_textpos();
                            let pos = self.buffers[self.current_buffer].lock().unwrap().move_cursor(MoveKind::Char(MoveDir::Next)).unwrap();
                            if old_pos != pos {
                                if pos.line_number > old_pos.line_number {
                                    self.views[self.current_view].view_cursor.row += 1;
                                    self.views[self.current_view].view_cursor.col = 1;
                                } else {
                                    self.views[self.current_view].view_cursor.col += 1;
                                }
                                let pos = self.buffers[0].lock().unwrap().get_absolute_cursor_pos();
                                let linepos = self.buffers[0].lock().unwrap().get_line_number_editing();
                                let WinDim(x, _y) = self.views[0].win_size;
                                let cursor_output_pos = WinDim(x-12, 1);
                                let WinDim(valx, valy) = cursor_output_pos;
                                let vc_pos = ViewCursor {col: valx as usize, row: valy as usize};
                                let vop = ViewOperations::ClearLineRest;
                                print!("{}{}{};{}|{};{}", vc_pos, vop, pos, linepos, self.views[0].view_cursor.col, self.views[0].view_cursor.row);
                                print!("{}", self.views[0].view_cursor);
                                stdout().flush();
                                // self.views[self.current_view].update_cursor();
                            }
                        },
                        EscapeKeyCode::Left => {
                            let pos = self.buffers[self.current_buffer].lock().unwrap().move_cursor(MoveKind::Char(MoveDir::Previous)).unwrap();
                                self.views[0].view_cursor = ViewCursor::from(pos.clone());
                                let WinDim(x, _y) = self.views[0].win_size;
                                let cursor_output_pos = WinDim(x-12, 1);
                                let WinDim(valx, valy) = cursor_output_pos;
                                let vc_pos = ViewCursor {col: valx as usize, row: valy as usize};
                                let vop = ViewOperations::ClearLineRest;
                                print!("{}{}{};{}|{};{}", vc_pos, vop, pos.get_line_position(), &pos.line_number, self.views[0].view_cursor.col, self.views[0].view_cursor.row);
                                print!("{}", self.views[0].view_cursor);
                                stdout().flush();
                        },
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
            // self.views[self.current_view].draw_view();
        }
    }

    pub fn interpret_input_sequence(&self) -> KeyCode {
        use libc::STDIN_FILENO;
        let res = unsafe {
            let mut ch_seq: [u8; 3] = [0,0,0];
            let kc = if read(STDIN_FILENO, ch_seq.as_mut_ptr() as *mut libc::c_void, 1) == 0 {
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
                3 => KeyCode::CtrlC,
                8 => KeyCode::CtrlBackspace,
                9 => KeyCode::Tab,
                13 => KeyCode::Enter,
                15 => KeyCode::CtrlO,
                17 => KeyCode::CtrlQ,
                19 => KeyCode::CtrlS,
                26 => KeyCode::CtrlZ,
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