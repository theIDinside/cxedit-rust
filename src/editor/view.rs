use crate::comms::observer::{EventListener, Event};
use crate::data::textbuffer::Textbuffer;

use std::sync::{Arc, Mutex};

pub trait ScreenUpdate {
    fn update(&self);
    fn update_at(&self, row: usize);
}

impl EventListener for View {
    fn on_event(&self, evt: Event) {
        match evt {
            Event::INSERTION(_x) => {
                // move blinky cursor x steps and update line
            },
            Event::DELETION(_x) => {
                // move blinky cursor x steps and update line
            }
        }
    }
}

struct ViewCursor {
    row: usize,
    col: usize
}

pub struct View {
    view_cursor: ViewCursor,
    size: (usize, usize),
    line_range: std::ops::Range<usize>,
    buffer_ref: Arc<Mutex<Textbuffer>>
}

impl View {

    pub fn new() -> Self {
        View {
            view_cursor: ViewCursor{ row: 0, col: 0},
            size: (0, 0),
            line_range: 0..0,
            buffer_ref: Arc::new(Mutex::new(Textbuffer::new()))
        }
    }

    pub fn set_viewed_buf(&mut self, buf: Arc<Mutex<Textbuffer>>) {
        let bufref = buf.clone();
        self.buffer_ref = bufref;
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

    }
}

