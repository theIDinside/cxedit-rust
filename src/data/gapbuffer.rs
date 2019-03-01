use std::ops::Range;
use std::ptr::copy as copyrange;
use super::BufferString;
use super::Buffer;

pub struct GapBuffer<T> {
    data: Vec<T>,
    gap: Range<usize>
}

impl <T> GapBuffer<T> {
    pub fn new() -> GapBuffer<T> {
        GapBuffer { data: Vec::new(), gap: 0..0 }
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
            panic!("Index {} out of bounds", pos);
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

    pub fn remove(&mut self) -> Option<T> {
        if self.gap.end == self.capacity() {
            return None;
        }
        let e = unsafe {
            std::ptr::read(self.space(self.gap.end))
        };
        self.gap.end += 1;
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
}

impl<T> super::Buffer<T> for GapBuffer<T> {
    fn insert(&mut self, data: T) {
        self.insert(data);
    }

    fn remove(&mut self) {
        self.remove();
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
            }
            tmpbuf
        };
        return res;
    }
}

impl BufferString for GapBuffer<Vec<GapBuffer<char>>> {
    fn read_string(&self, range: std::ops::Range<usize>) -> String {
        if range.end < self.len() {
            let mut tmpbuf = String::new():
            for i in range {
                match self.get(i) {
                    Some(line)
                }
            }
        }
        "".to_string()
    }
}