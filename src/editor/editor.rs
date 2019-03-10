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
use crate::cmd::command_engine::Operation;
use crate::cmd::command_engine::OperationResult;
use crate::editor::view::WinDim;
use std::thread::sleep;
use std::time::Duration;
use std::ops::Range;
use crate::data::text_buffer::TextPosition;
use crate::cmd::StatlineCommand;
use crate::cmd::StatlineCommandFlag;
use crate::cmd::StatlineCommandFlagList;
use crate::cmd::SeekFrom;
use crate::data::text_buffer::Cursor;
use crate::data::text_buffer::ObjectKind;

pub fn debug_sleep(msg: Option<String>, val: Option<u64>) {
    println!("\x1b[25;10H{}", msg.unwrap_or(" ".into()));
    sleep(Duration::from_millis(val.unwrap_or(1500)));
}

pub enum Mode {
    Normal,
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
    _input_mode: Mode,
    config: Config,
    cmd_engine: CommandEngine
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
            buffers: vec![Arc::new(Mutex::new(Textbuffer::new()))],
            views: vec![],
            current_buffer: 0,
            current_view: 0,
            running: false,
            original_terminal_settings: None,
            _input_mode: Mode::Normal,
            config: Config::default(),
            cmd_engine: CommandEngine::new(Arc::new(Mutex::new(Textbuffer::new())))
        }
    }

    pub fn is_pristine(&self) -> bool {
        !self.buffers[0].lock().unwrap().is_dirty()
    }

    pub fn init(&mut self, settings: Option<Termios>) {
        self.original_terminal_settings = settings;
        self.config = Config::read_config(Path::new("config.rc"));

        if self.buffers.len() != 0 {
            self.buffers.push(Arc::new(Mutex::new(Textbuffer::new())));
        }
        let mut v = View::new().unwrap_or_else(|| View::new().unwrap());
        v.set_viewed_buf(self.buffers[0].clone());
        v.init();
        self.views.push(v);
        self.running = true;
        self.cmd_engine.register_buffer(self.buffers[0].clone());
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
                    let line_count: usize = data.chars().filter(|c| *c == '\n').collect::<Vec<char>>().len() + 1;
                    self.buffers[0].lock().unwrap().line_count = line_count;
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

    pub fn enter_statline_command(&mut self) -> Option<StatlineCommand> {
        let title = "[command]: ";
        self.views[0].statline_view_cursor.col = title.len() + 1;
        print!("{}{}{}{}{}", self.views[0].status_line_position, self.views[0].view_cfg.stat_line_color.0, self.views[0].view_cfg.stat_line_color.1, title, ViewOperations::ClearLineRest);
        stdout().flush();
        let title_len = title.len();
        let mut vc = self.views[0].status_line_position;
        vc.col = title_len + 1usize;
        let mut cmd_string_buffer_index = 0;

        let a = loop {
            if 0 == 0 {
                break None;
            }
            break Some(StatlineCommand::Error("Erroneous input".into()));
        };
        a
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
                            let (file, flags) = input.split_at(input.find(" ").unwrap_or(input.len()-1));
                            Some(StatlineCommand::OpenFile(Some(file.into()), StatlineCommandFlagList::from(flags).has_to_vec()))
                        },
                        Command::Save => {
                            let (file, flags) = input.split_at(input.find(" ").unwrap_or(input.len()-1));
                            Some(StatlineCommand::SaveFile(Some(file.into()), StatlineCommandFlagList::from(flags).has_to_vec()))
                        },
                        Command::Find => {
                            if input.len() > 0 {
                                Some(StatlineCommand::Find(Some(input), SeekFrom::Start))
                            } else {
                                None
                            }
                        },
                        Command::Jump => {
                            input.parse::<usize>().ok().and_then(|v| Some(StatlineCommand::Goto(Some(v)))).or(Some(StatlineCommand::Error(format!("Couldn't parse line number from {}", input))))
                        },
                        Command::Move(_mk) => unimplemented!(),
                        Command::Quit => {
                            unimplemented!();
                        },
                        Command::CommandInput => {
                            unimplemented!();
                        },
                        Command::Action(_) => unimplemented!()
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
                },
                KeyCode::Tab => {
                  // TODO: statline autocompletion
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

    pub fn statline_error_msg(&mut self, msg: &str) {
        self.views[0].on_statline_error(msg);
    }

    pub fn take_command_input(&mut self) {

    }

    // TODO: split up run function, to remove the intense complexity and spaghettization of code
    pub fn run(&mut self) {
        // TODO: setup code, and also
        // println!("Entering editor main loop:\r");
        let mut running = self.running;
        while running {
            let kp = self.handle_keypress();
            /*
            match self._input_mode {
                Mode::Movement => {

                },
                Mode::Input => {
                    self.take_normal_input();
                },
                Mode::Command => {
                    let cmd = cmd_engine.read_command().and_then(|cmd| cmd.exec())
                    cmd.exec();
                },
            }
            */
            self.views[0].restore_statline();
            match kp {
                KeyCode::Character(c) => {
                    let abs_pos =self.buffers[0].lock().unwrap().get_textpos();
                    let pos = abs_pos.absolute;
                    match self.cmd_engine.execute(Operation::Insert(pos, c)) {
                        OperationResult::OK => {
                            self.views[self.current_view].draw_view();
                        },
                        OperationResult::ERR(errmsg) => {

                        }
                    }
                },
                KeyCode::Enter => {
                    let pos = *&self.buffers[0].lock().unwrap().get_textpos().absolute;
                    match self.cmd_engine.execute(Operation::Insert(pos, '\n')) {
                        OperationResult::OK => {
                            self.views[0].draw_view();
                        },
                        OperationResult::ERR(errmsg)=> {

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
                        match self.cmd_engine.execute(Operation::Remove(pos, c)) {
                            OperationResult::OK => {
                            self.views[0].draw_view();
                            },
                            OperationResult::ERR(errmsg)=> {

                            }
                        }
                    }
                },
                KeyCode::CtrlW => {
                    self.views[0].restore_statline();
                    let completed = match &self.cmd_engine.combo_trigger {
                        Some(kc) if *kc == KeyCode::CtrlW => {
                            if let Some(cmd) = self.config.get_combo_bindings(kc).and_then(|map| map.get(&KeyCode::CtrlW)) {
                                match cmd {
                                    Command::Action(Operation::Copy(ObjectKind::Line)) => {
                                        let r= self.buffers[0].lock().unwrap().find_range_of(Cursor::Buffer, ObjectKind::Line);
                                        if r.is_some() {
                                            let range = r.unwrap();
                                            let msg: String = self.buffers[0].lock().unwrap().get_data(range.clone()).chars().filter(|c| *c != '\n').collect();
                                            self.views[0].on_statline_error(format!("Found range: {}..{}, with contents: '{}'", &range.start, &range.end, msg).as_ref());
                                        }
                                    },
                                    _ => {}
                                }
                                true
                            } else {
                                false
                            }
                        },
                        Some(_) => {
                            true
                        },
                        None => {
                            let r= self.buffers[0].lock().unwrap().find_range_of(Cursor::Buffer, ObjectKind::Word);
                            if r.is_some() {
                                let range = r.unwrap();
                                let msg = self.buffers[0].lock().unwrap().get_data(range.clone());
                                self.views[0].on_statline_error(format!("Found range: {}..{}, with contents: '{}'", &range.start, &range.end, msg).as_ref());
                            }
                            false
                        }
                    };
                    if !completed {
                        self.cmd_engine.combo_trigger = Some(KeyCode::CtrlW)
                    } else {
                        self.cmd_engine.combo_trigger = None
                    }
                }
                KeyCode::Tab => {
                    let pos = *&self.buffers[0].lock().unwrap().get_textpos().absolute;
                    match self.cmd_engine.execute(Operation::InsertData(pos, "    ".into())) {
                        OperationResult::OK => {
                            self.views[self.current_view].draw_view();
                        },
                        OperationResult::ERR(errmsg)=> {}
                    }
                },
                KeyCode::Esc => {},
                KeyCode::CtrlBackspace => {
                    let current_pos = self.buffers[0].lock().unwrap().get_textpos().absolute;

                },
                KeyCode::CtrlG => {
                    self.views[self.current_view].on_goto();
                    let cmd = self.statline_input(Command::Jump);
                    if let Some(StatlineCommand::Goto(Some(line))) = cmd {
                        let line_pos = self.buffers[0].lock().unwrap().get_line_abs_index(line);
                        self.buffers[0].lock().unwrap().set_textpos(line_pos.clone().unwrap().absolute);
                        self.views[0].view_cursor = ViewCursor::from(line_pos.unwrap());
                        self.views[0].draw_view();
                    } else if let Some(StatlineCommand::Error(msg)) = cmd {
                        self.statline_error_msg(&"[goto error]: ".chars().chain(msg.chars()).collect::<String>());
                    } else {
                        self.statline_error_msg("Unknown input error, command not performed!");
                    }
                },
                KeyCode::CtrlV => {},
                KeyCode::CtrlS => {
                    /* TODO: open status line if we do not have a filename, write in filename
                            validate provided path, open a new file with that name -> write contents.
                            reset statusline.
                    */
                    // TODO: implement 2 functions, one that will write without asking for new file name/ask if ok, and one that does
                    // TODO: implement Config for editor. Then request mapping of key to command via self.get_keybinding(KeyCode::CtrlS)
                    self.views[0].on_save_file();
                    let cmd = self.statline_input(Command::Save);
                    if let Some(SaveFile(Some(suggested_fname), flags)) = cmd {
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
                    if let Some(StatlineCommand::OpenFile(Some(fname), flags)) = cmd {
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
                    match self.cmd_engine.execute(Operation::Undo) {
                        OperationResult::OK => {
                            self.views[0].draw_view();
                        },
                        OperationResult::ERR(errmsg)=> {

                        }
                    }
                },
                KeyCode::CtrlA => {
                    // N.B! This is a debug function ONLY. Used in the beginning for testing display functions, cursor navigation etc
                    // This will become something else entirely.
                    let tp = self.buffers[self.current_buffer].lock().unwrap().get_textpos();
                    print!("Text buffer position: absolute: {}, line_start_absolute: {}, line_number: {}, line column position: {}\r\n", tp.absolute, tp.line_start_absolute, tp.line_index, tp.get_line_position());
                    print!("Text buffer get line at buffer cursor: {}\r\n", self.buffers[self.current_buffer].lock().unwrap().get_line_number());
                    print!("View cursor position: {},{}", self.views[self.current_view].view_cursor.col, self.views[self.current_view].view_cursor.row);
                    stdout().flush();

                    self.views[self.current_view].update_cursor();
                },
                KeyCode::CtrlB => {
                    // self.views[self.current_view].draw_view();
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
                            let pos = self.buffers[self.current_buffer].lock().unwrap().move_cursor(MoveKind::Char(MoveDir::Next)).unwrap();
                            self.views[0].view_cursor = ViewCursor::from(pos.clone());
                            if (self.views[0].view_cursor.row - self.views[0].line_range.start) == self.views[0].win_size.1 as usize {
                                self.views[0].scroll_down();
                                self.views[0].draw_view();
                                // self.views[0].view_cursor.row -= self.views[0].line_range.start;
                            } else {
                                self.views[0].view_cursor.row -= self.views[0].line_range.start;
                                stdout().flush();
                            }
                            self.views[0].statline_update_line_number(pos.line_index+1, pos.get_line_position()+1);
                        },
                        EscapeKeyCode::Left => {
                            let pos = self.buffers[self.current_buffer].lock().unwrap().move_cursor(MoveKind::Char(MoveDir::Previous)).unwrap();
                            self.views[0].view_cursor = ViewCursor::from(pos.clone());
                            if (self.views[0].view_cursor.row - self.views[0].line_range.start) == 0 {
                                self.views[0].scroll_up();
                                self.views[0].draw_view();
                            } else {
                                self.views[0].view_cursor.row -= self.views[0].line_range.start;
                                stdout().flush();
                            }
                            self.views[0].statline_update_line_number(pos.line_index+1, pos.get_line_position()+1);
                        },
                        EscapeKeyCode::Up => {
                            let pos = self.buffers[self.current_buffer].lock().unwrap().move_cursor(MoveKind::Line(MoveDir::Previous)).unwrap();
                            self.views[0].view_cursor = ViewCursor::from(pos.clone());
                            if (self.views[0].view_cursor.row - self.views[0].line_range.start) == 0 {
                                self.views[0].scroll_up();
                                self.views[0].draw_view();
                            } else {
                                self.views[0].view_cursor.row -= self.views[0].line_range.start;
                                // print!("{}", self.views[0].view_cursor);
                                stdout().flush();
                            }
                            self.views[0].statline_update_line_number(pos.line_index+1, pos.get_line_position()+1);
                        },
                        EscapeKeyCode::Down => {
                            // self.buffers[self.current_buffer].lock().unwrap().move_cursor();
                            let pos = self.buffers[0].lock().unwrap().move_cursor(MoveKind::Line(MoveDir::Next)).unwrap();
                            self.views[0].view_cursor = ViewCursor::from(pos.clone());
                            if (self.views[0].view_cursor.row - self.views[0].line_range.start) == self.views[0].win_size.1 as usize {
                                self.views[0].scroll_down();
                                self.views[0].draw_view();
                                // self.views[0].view_cursor.row -= self.views[0].line_range.start;
                            } else {
                                self.views[0].view_cursor.row -= self.views[0].line_range.start;
                                // print!("{}", self.views[0].view_cursor);
                                stdout().flush();
                            }
                            // self.views[0].draw_view();
                            self.views[0].restore_statline();
                            self.views[0].statline_update_line_number(pos.line_index+1, pos.get_line_position()+1);
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
                7 => KeyCode::CtrlG,
                8 => KeyCode::CtrlBackspace,
                9 => KeyCode::Tab,
                13 => KeyCode::Enter,
                15 => KeyCode::CtrlO,
                17 => KeyCode::CtrlQ,
                19 => KeyCode::CtrlS,
                22 => KeyCode::CtrlV,
                23 => KeyCode::CtrlW,
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