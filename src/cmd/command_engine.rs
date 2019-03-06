use crate::cmd::Command;
use std::sync::{Arc, Mutex};
use crate::data::text_buffer::Textbuffer;
use std::sync::MutexGuard;
use std::thread::sleep;
use std::time::Duration;

pub enum Action {
    Insert(usize, char),
    InsertWord(usize, String),
    Delete(usize, char),
    Remove(usize, char),
    Undo,
    Redo
}

pub enum ActionResult {
    OK,
    ERR
}

pub struct CommandEngine {
    history: Vec<Action>,
    forward_history: Vec<Action>,
    buffer_ref: Arc<Mutex<Textbuffer>>
}

impl CommandEngine {
    pub fn new(buffer: Arc<Mutex<Textbuffer>>) -> CommandEngine {
        CommandEngine {
            history: vec![],
            forward_history: vec![],
            buffer_ref: buffer.clone()
        }
    }

    pub fn register_buffer(&mut self, buf_ref: Arc<Mutex<Textbuffer>>) {
        self.buffer_ref = buf_ref.clone();
    }

    pub fn execute(&mut self, action: Action) -> ActionResult {
        match &action {
            Action::Insert(pos, ch) => {
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
                ActionResult::OK
            },
            Action::InsertWord(pos, data) => {
                let mut guard = self.buffer_ref.lock().unwrap();
                let bufpos = guard.get_textpos().absolute;
                if *pos == bufpos {
                    guard.insert_data(&data);
                } else {
                    guard.set_textpos(*pos);
                    guard.insert_data(data);
                }
                self.history.push(action);
                ActionResult::OK
            },
            Action::Delete(pos, ch) => {
                let mut guard = self.buffer_ref.lock().unwrap();
                let bufpos = guard.get_textpos().absolute;
                if *pos == bufpos {
                    guard.delete();
                } else {
                    guard.set_textpos(*pos);
                    guard.delete()
                }
                self.history.push(action);
                ActionResult::OK
            },
            Action::Remove(pos, ch) => {
                // sleep(Duration::from_millis(1500));
                let mut guard = self.buffer_ref.lock().unwrap();
                guard.set_textpos(*pos);
                let bufpos = guard.get_textpos().absolute;
                if *pos == bufpos && *pos > 0 {
                    if let Some(c) = guard.remove() {
                        println!("\x1b[15;1H char to be removed: [{}], char that was removed: [{}]", ch, c);
                    }
                    self.history.push(action);
                    ActionResult::OK
                } else if *pos > 0 {
                    // guard.set_textpos(*pos);
                    guard.remove();
                    self.history.push(Action::Remove(bufpos-1, *ch));
                    ActionResult::OK
                } else {
                    ActionResult::ERR
                }
            },
            Action::Undo => {
                if let Some(act) = self.history.last() {
                    match act {
                        Action::Delete(pos, ch) => {
                            let mut guard= self.buffer_ref.lock().unwrap();
                            let bufpos = guard.get_textpos().absolute;
                            if *pos == bufpos {
                                guard.insert_ch(*ch);
                            } else {
                                guard.set_textpos(*pos);
                                guard.insert_ch(*ch);
                            }
                            self.forward_history.push(Action::Insert(*pos, *ch));
                            self.history.pop();
                            ActionResult::OK
                        },
                        Action::Insert(pos, ch) => {
                            let mut guard = self.buffer_ref.lock().unwrap();
                            let bufpos = guard.get_textpos().absolute;
                            if *pos == bufpos {
                                guard.delete();
                            } else {
                                guard.set_textpos(*pos);
                                guard.delete()
                            }
                            self.forward_history.push(Action::Delete(*pos, *ch));
                            self.history.pop();
                            ActionResult::OK
                        },
                        Action::Remove(pos, ch) => {
                            let mut guard = self.buffer_ref.lock().unwrap();
                            let bufpos = guard.get_textpos().absolute;
                            if *pos == bufpos {
                                guard.insert_ch(*ch);
                            } else {
                                guard.set_textpos(*pos);
                                guard.insert_ch(*ch);
                            }
                            self.forward_history.push(Action::Insert(*pos, *ch));
                            self.history.pop();
                            ActionResult::OK
                        }
                        _ => {
                            sleep(Duration::from_millis(2500));
                            unimplemented!("This is not implemented yet!!!");
                            sleep(Duration::from_millis(500));
                            ActionResult::ERR
                        }
                    }
                } else {
                    ActionResult::ERR
                }
            },
            Action::Redo => {
                ActionResult::ERR
            }
        }
    }
}