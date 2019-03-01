use comms::observer::{EventListener, Event};
use data::textbuffer::Textbuffer;
use std::rc::Rc;
use std::cell::RefCell;

impl EventListener for View {
    fn on_event(&self, evt: Event) {
        match evt {
            Event::INSERTION(x) => {
                // move blinky cursor x steps and update line
            },
            Event::DELETION(x) => {
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
    view_cursor: ViewCursor
}

impl View {
    // using the Rc<RefCell> is needed for being able to register with the text buffer, this view is watching.
    // It is kind of annoying, since the only thing we ever mutate, is the actual observer/listener at the buffer.
    // A view, never, _never_ mutates the buffer data.
    fn register_with(&self, buffer: Rc<RefCell<Textbuffer>>) {

    }
}

