use std::sync::{Arc, Mutex};
use crate::data::text_buffer::Textbuffer;
use std::thread::sleep;
use std::time::Duration;
use std::collections::HashMap;
use crate::{Serialize as S, Deserialize as D};
use crate::data::text_buffer::ObjectKind;
use crate::editor::key::KeyCode;


pub enum Position {
    Absolute(usize),
    Relative(usize)
}

// using private alias types. Makes reading the source code easier and quicker to grasp.
type MacroName = String;
type AbsolutePos = usize;

#[derive(Clone, S, D, Debug)]
pub enum Operation {
    Insert(AbsolutePos, char),
    InsertData(AbsolutePos, String),
    Delete(AbsolutePos, char),
    Remove(AbsolutePos, char),
    Copy(ObjectKind),
    MacroRecord,
    MacroStop,
    MacroPlay(MacroName),
    Undo,
    Redo,
}

pub struct Macro {
    data: String,
    lines: usize,
    len: usize
}

impl Macro {
    pub fn play(&self) -> &str {
        &self.data
    }
}

impl Default for Macro {
    fn default() -> Self {
        Macro {
            data: String::new(),
            lines: 0,
            len: 0
        }
    }
}

pub enum OperationResult {
    OK,
    ERR(String)
}

pub struct CommandEngine {
    history: Vec<Operation>,
    forward_history: Vec<Operation>,
    buffer_ref: Arc<Mutex<Textbuffer>>,
    macros: HashMap<String, Macro>,
    pub combo_trigger: Option<KeyCode>,
    _macro_recording: bool,
}

impl CommandEngine {
    pub fn new(buffer: Arc<Mutex<Textbuffer>>) -> CommandEngine {
        CommandEngine {
            history: vec![],
            forward_history: vec![],
            buffer_ref: buffer.clone(),
            macros: HashMap::new(),
            _macro_recording: false,
            combo_trigger: None
        }
    }

    pub fn set_last_key(&mut self, kc: KeyCode) {
        self.combo_trigger = Some(kc)
    }

    pub fn register_buffer(&mut self, buf_ref: Arc<Mutex<Textbuffer>>) {
        self.buffer_ref = buf_ref.clone();
    }

    pub fn execute(&mut self, action: Operation) -> OperationResult {

        match &action {
            Operation::Insert(pos, ch) => {
                let mut guard = self.buffer_ref.lock().unwrap();
                let bufpos = guard.get_textpos().absolute;
                if *pos == bufpos {
                    guard.insert_ch(*ch);
                } else {
                    guard.set_textpos(*pos);
                    guard.insert_ch(*ch);
                }
                self.history.push(action);
                self.forward_history.clear();
                OperationResult::OK
            },
            Operation::InsertData(pos, data) => {
                let mut guard = self.buffer_ref.lock().unwrap();
                let bufpos = guard.get_textpos().absolute;
                if *pos == bufpos {
                    guard.insert_data(data);
                } else {
                    guard.set_textpos(*pos);
                    guard.insert_data(data);
                }
                self.history.push(action);
                OperationResult::OK
            },
            Operation::Delete(pos, _) => {
                let mut guard = self.buffer_ref.lock().unwrap();
                let bufpos = guard.get_textpos().absolute;
                if *pos == bufpos {
                    guard.delete();
                } else {
                    guard.set_textpos(*pos);
                    guard.delete();
                }
                self.history.push(action);
                OperationResult::OK
            },
            Operation::Remove(pos, ch) => {
                // sleep(Duration::from_millis(1500));
                let mut guard = self.buffer_ref.lock().unwrap();
                guard.set_textpos(*pos);
                let bufpos = guard.get_textpos().absolute;
                if *pos == bufpos && *pos > 0 {
                    guard.remove();
                    self.history.push(action);
                    OperationResult::OK
                } else if *pos > 0 {
                    // guard.set_textpos(*pos);
                    if let Some(c) = guard.remove() {
                        if c != *ch {
                            println!("ERROR C != CH");
                        }
                    };
                    self.history.push(Operation::Remove(bufpos-1, *ch));
                    OperationResult::OK
                } else {
                    OperationResult::ERR(format!("Couldn't remove {} at {}", ch, pos))
                }
            },
            Operation::Undo => {
                if let Some(act) = self.history.last() {
                    match act {
                        Operation::Delete(pos, ch) => {
                            let mut guard= self.buffer_ref.lock().unwrap();
                            let bufpos = guard.get_textpos().absolute;
                            if *pos == bufpos {
                                guard.insert_ch(*ch);
                            } else {
                                guard.set_textpos(*pos);
                                guard.insert_ch(*ch);
                            }
                            self.forward_history.push(Operation::Insert(*pos, *ch));
                            self.history.pop();
                            OperationResult::OK
                        },
                        Operation::Insert(pos, ch) => {
                            let mut guard = self.buffer_ref.lock().unwrap();
                            let bufpos = guard.get_textpos().absolute;
                            if *pos == bufpos {
                                guard.delete();
                            } else {
                                guard.set_textpos(*pos);
                                guard.delete();
                            }
                            self.forward_history.push(Operation::Remove(*pos, *ch));
                            self.history.pop();
                            OperationResult::OK
                        },
                        Operation::Remove(pos, ch) => {
                            let mut guard = self.buffer_ref.lock().unwrap();
                            guard.set_textpos(*pos-1);
                            guard.insert_ch(*ch);
                            self.forward_history.push(Operation::Insert(*pos, *ch));
                            self.history.pop();
                            OperationResult::OK
                        },
                        Operation::InsertData(pos, data) => {
                            let mut guard = self.buffer_ref.lock().unwrap();
                            let bufpos = guard.get_textpos().absolute;
                            if *pos == bufpos {
                                for i in 0..data.len() {
                                    if let Some(ch) = guard.delete() {
                                        self.forward_history.push(Operation::Remove(*pos+i, ch));
                                    }
                                }
                            } else {
                                guard.set_textpos(*pos);
                                for i in 0..data.len() {
                                    if let Some(ch) = guard.delete() {
                                        self.forward_history.push(Operation::Remove(*pos+i, ch));
                                    }
                                }
                            }
                            self.history.pop();
                            OperationResult::OK
                        },
                        Operation::MacroPlay(macroname) => {
                            let m = &self.macros[macroname];
                            unimplemented!("Undoing played macros not yet implemented!")
                        },
                        _ => {
                            unimplemented!("This is not implemented yet!!!");
                            sleep(Duration::from_millis(2500));
                            OperationResult::ERR("Not yet implemented!".into())
                        }
                    }
                } else {
                    OperationResult::ERR("History queue empty.".into())
                }
            },
            Operation::Redo => {
                unimplemented!("Redoing last undo command not yet implemented");
                OperationResult::ERR(format!("Redo command not implemented yet"))
            },
            Operation::MacroPlay(name) => {
                unimplemented!("Playing macros not yet implemented");
                OperationResult::ERR(format!("Macro command not implemented yet"))
            }
            Operation::MacroRecord => {
                unimplemented!("Recording macros not yet implemented");
                OperationResult::ERR(format!("Macro command not implemented yet"))
            }
            Operation::MacroStop => {
                unimplemented!("Recording macros not yet implemented");
                OperationResult::ERR(format!("Macro command not implemented yet"))
            },
            Operation::Copy(obj) => {
                OperationResult::ERR(format!("Macro command not implemented yet"))
            }
        }
    }
}