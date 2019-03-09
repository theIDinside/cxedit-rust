#[macro_use]
extern crate serde_json;
extern crate serde;

pub mod actions;
pub mod cmd;
pub mod data;
pub mod editor;
pub mod comms;
pub mod cfg;
pub mod ipc;

use serde::{Serialize, Deserialize};