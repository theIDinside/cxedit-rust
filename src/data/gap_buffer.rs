use std::ops::Range;
use std::ptr::copy as copyrange;
use super::BufferString;
use std::ops::Index;
use crate::data::text_buffer::Cursor;

pub struct GapBuffer<T> {
    data: Vec<T>,
    gap: Range<usize>
}

impl <T> GapBuffer<T> {
    pub fn new() -> GapBuffer<T> {
        GapBuffer {
            data: Vec::new(),
            gap: 0..0,
        }
    }

    pub fn capacity(&self) -> usize {
        self.data.capacity()
    }

    pub fn len(&self) -> usize {
        self.capacity() - self.gap.len()
    }

    pub fn get_pos(&self) -> usize {
        self.gap.start
    }

    // unsafe fn. Should only be called after index has been determined to be within valid range.
    unsafe fn space(&self, index: usize) -> *const T {
        self.data.as_ptr().offset(index as isize)
    }

    unsafe fn space_mut(&mut self, index: usize) -> *mut T {
        self.data.as_mut_ptr().offset(index as isize)
    }

    fn index_to_raw(&self, index: usize) -> usize {
        if index < self.gap.start {
            index
        } else {
            index + self.gap.len()
        }
    }

    pub fn get(&self, index: usize) -> Option<&T> {
        let raw = self.index_to_raw(index);
        if raw < self.capacity() {
            unsafe {
                Some(&*self.space(raw))
            }
        } else {
            None
        }
    }

    pub fn set_gap_position(&mut self, pos: usize) {
        if pos > self.len() {
            panic!("GapBuffer index {} out of bounds", pos);
        }
        unsafe {
            let gap = self.gap.clone();
            if pos > gap.start {
                let distance = pos - gap.start;
                copyrange(self.space(gap.end), self.space_mut(gap.start), distance);
            } else if pos < gap.start {
                let distance = gap.start - pos;
                copyrange(self.space(pos), self.space_mut(gap.end - distance), distance);
            }
            self.gap = pos .. pos+ gap.len();
        }
    }

    pub fn insert(&mut self, elem: T) {
        if self.gap.len() == 0 {
            self.enlarge_gap();
        }

        unsafe {
            let index = self.gap.start;
            std::ptr::write(self.space_mut(index), elem);
        }
        self.gap.start += 1;
    }

    pub fn map_to<Iter>(&mut self, iterable: Iter) where Iter: IntoIterator<Item=T> {
        for item in iterable {
            self.insert(item);
        }
    }

    /**
    Works like the "delete" key when you edit text. It deletes what is AFTER the cursor. (gap.end + 1)
    */
    pub fn delete(&mut self) -> Option<T> {
        if self.gap.end == self.capacity() {
            return None;
        }
        let e = unsafe {
            std::ptr::read(self.space(self.gap.end))
        };
        self.gap.end += 1;
        Some(e)
    }

    /**
    Works like the "backspace" key when you edit text. It deletes what is BEFORE the cursor. (gap.start - 1)
    */
    pub fn remove(&mut self) -> Option<T> {
        if self.gap.start == 0 {
            return None;
        }
        let e = unsafe {
            std::ptr::read(self.space(self.gap.start - 1))
        };
        self.gap.start -= 1;
        Some(e)
    }

    fn enlarge_gap(&mut self) {
        use std::ptr::copy_nonoverlapping as copyNoOverlap;
        let mut newcap = self.capacity() * 2;
        if newcap == 0 {
            // existing vector data is empty.. choosing 16 bytes = 128 bit gap. Perhaps this will optimize string copies using AVX? We'll see.
            newcap = 16;
        }

        let mut newbuf = Vec::with_capacity(newcap);
        let aftergap = self.capacity() - self.gap.end;
        let newgap = self.gap.start .. newbuf.capacity() - aftergap;
        unsafe {
            copyNoOverlap(self.space(0), newbuf.as_mut_ptr(), self.gap.start);
            let newgap_end = newbuf.as_mut_ptr().offset(newgap.end as isize);
            copyNoOverlap(self.space(self.gap.end), newgap_end, aftergap);
        }
        self.data = newbuf;
        self.gap = newgap;
    }

    pub fn new_with_capacity(cap: usize) -> GapBuffer<T> {
        let mut gb = GapBuffer::new();
        let buf = Vec::with_capacity(cap);
        let gap = 0..16;
        gb.gap = gap;
        gb.data = buf;
        gb
    }

    pub fn iter_begin_to_cursor(&self, cursor: Cursor) -> GapBufferIterator<T> {
        let pos = match cursor {
            Cursor::Absolute(pos) => pos,
            Cursor::Buffer => self.get_pos(),
        };
        GapBufferIterator {
            start: 0,
            end: pos,
            buffer: self
        }
    }

    pub fn iter_cursor_to_end(&self, cursor: Cursor) -> GapBufferIterator<T> {
        let pos = match cursor {
            Cursor::Absolute(pos) => pos,
            Cursor::Buffer => self.get_pos(),
        };
        GapBufferIterator {
            start: pos,
            end: self.len(),
            buffer: self
        }
    }

    pub fn iter(&self) -> GapBufferIterator<T> {
        GapBufferIterator {
            start: 0,
            end: self.len(),
            buffer: self
        }
    }
}

pub struct GapBufferIterator<'a, T> {
    start: usize,
    end: usize,
    buffer: &'a GapBuffer<T>
}

impl<'a, T> Iterator for GapBufferIterator<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.start < self.end {
            if let Some(c) = self.buffer.get(self.start) {
                self.start += 1;
                Some(c)
            } else {
                None
            }
        } else {
            None
        }
    }

    fn position<P>(&mut self, mut predicate: P) -> Option<usize> where
        Self: Sized,
        P: FnMut(Self::Item) -> bool, {
        while let Some(ch) = self.next() {
         if predicate(ch) {
             return Some(self.start);
         }
        }
        None
    }

    fn rposition<P>(&mut self, mut predicate: P) -> Option<usize> where P: FnMut(Self::Item) -> bool, Self: ExactSizeIterator + DoubleEndedIterator, {
        while let Some(ch) = self.next_back() {
            if predicate(ch) {
                return Some(self.end)
            }
        }
        None
    }
}


impl<'a, T> DoubleEndedIterator for GapBufferIterator<'a, T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.end >= self.start {
            if let Some(c) = self.buffer.get(self.end) {
                self.end -= 1;
                Some(c)
            } else {
                None
            }
        } else {
            None
        }
    }
}

impl Index<usize> for GapBuffer<char> {
    type Output = char;
    fn index(&self, index: usize) -> &Self::Output {
        self.get(index).unwrap()
    }
}

impl<T> super::Buffer<T> for GapBuffer<T> {
    fn insert(&mut self, data: T) {
        self.insert(data);
    }

    fn remove(&mut self) {
        self.delete();
    }
}

impl<T> Drop for GapBuffer<T> {
    fn drop(&mut self) {
        unsafe {
            for i in 0 .. self.gap.start {
                std::ptr::drop_in_place(self.space_mut(i));
            }
            for i in self.gap.end .. self.capacity() {
                std::ptr::drop_in_place(self.space_mut(i));
            }
        }
    }
}

impl BufferString for GapBuffer<char> {
    fn read_string(&self, range: std::ops::Range<usize>) -> String {
        let res = if range.end < self.len() {
            let mut tmpbuf = String::with_capacity(range.len());
            for i in range {
                match self.get(i) {
                    Some(c) => {
                        tmpbuf.insert(i, *c);
                        // tmpbuf.insert(i, *c);
                    },
                    _ => {

                    }
                }
            }
            tmpbuf
        } else {
            let mut tmpbuf = String::with_capacity(self.len());
            for i in range {
                match self.get(i) {
                    Some(c) => tmpbuf.push(*c),
                    _ => {}
                }
                if i == self.len() {
                    return tmpbuf;
                }
            }
            tmpbuf
        };
        return res;
    }
}