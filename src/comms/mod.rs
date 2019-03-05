pub mod observer {

    pub enum EventData {
        Char(char),
        CharRange(String) // later perhaps we can use a more efficient data structure.. for now, lets just use string.
    }

    /**
        Events contain a pos field (unamed enum field). This is where the event "begins", in the
        buffer data structure.
    */
    pub enum Event {
        INSERTION(usize, EventData),
        DELETION(usize,EventData),    // deletion goes pos..->
        REMOVAL(usize,EventData)      // removal goes <-..pos
    }

    pub trait EventListener {
        fn on_event(&self, evt: Event);
    }
}