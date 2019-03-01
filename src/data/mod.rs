pub mod gapbuffer;
pub mod textbuffer;

pub trait Buffer<T> where T: Sized {
    fn insert(&mut self, data: T);
    fn remove(&mut self);
}

pub trait BufferString {
    fn read_string(&self, range: std::ops::Range<usize>) -> String;
}


#[cfg(test)]
mod tests {
    use super::gapbuffer::GapBuffer as GB;
    // use super::Gap::GapBuffer as GB;
    use super::BufferString;
    use crate::data::Gap::GapBuffer;

    #[test]
    fn test_gapbuffer_insert() {
        let mut gb = GB::new();
        gb.map_to("hello world!".chars());
        assert_eq!(gb.get(0), Some(&'h'));
    }

    #[test]
    fn test_gapbuffer_read_string() {
        let mut gb = GB::new();
        gb.map_to("hello world!".chars());
        assert_eq!(gb.read_string(0..5), "hello");
    }

    #[test]
    fn test_gapbuffer_dump_to_string() {
        let mut gb = GB::new();
        gb.map_to("hello world!".chars());
        assert_eq!(gb.read_string(0..25), "hello world!");
    }

    #[test]
    fn test_insert_move_insert() {
        let mut gb = GB::new();
        gb.map_to("hello world!".chars());
        assert_eq!(gb.read_string(0..25), "hello world!");
        gb.set_gap_position(6);
        gb.map_to("fucking ".chars());
        let a = gb.read_string(0..25);
        assert_eq!(gb.read_string(0..25), "hello fucking world!");
    }

    #[test]
    fn test_insert_lines() {
        let mut gb: GapBuffer<Vec<GapBuffer<char>>> = GB::new();
    }

}