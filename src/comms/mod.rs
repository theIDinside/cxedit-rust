pub mod observer {
    pub enum Event {
        INSERTION(usize),
        DELETION(usize)
    }

    pub trait EventListener {
        fn on_event(&self, evt: Event);
    }
}