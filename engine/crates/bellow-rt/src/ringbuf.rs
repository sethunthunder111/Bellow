//! Lock-free SPSC ring buffer for control → audio thread communication.
//!
//! Uses a fixed-capacity backing store. Producer (control) and consumer
//! (audio) each own one atomic index. No locks, no alloc, no syscalls.

use std::cell::UnsafeCell;
use std::sync::atomic::{AtomicUsize, Ordering};

pub struct RingBuf<T> {
    buffer: Box<[UnsafeCell<T>]>,
    write: AtomicUsize,
    read: AtomicUsize,
}

// Safety: T must be Send because producer (control) writes and consumer
// (audio) reads from different threads.
unsafe impl<T: Send> Send for RingBuf<T> {}
unsafe impl<T: Send> Sync for RingBuf<T> {}

impl<T: Copy + Default> RingBuf<T> {
    pub fn new(capacity: usize) -> Self {
        let mut v = Vec::with_capacity(capacity);
        for _ in 0..capacity {
            v.push(UnsafeCell::new(T::default()));
        }
        Self {
            buffer: v.into_boxed_slice(),
            write: AtomicUsize::new(0),
            read: AtomicUsize::new(0),
        }
    }

    pub fn capacity(&self) -> usize {
        self.buffer.len()
    }

    /// Push a single item. Returns false if full.
    /// Only the producer thread should call this.
    pub fn push(&self, item: T) -> bool {
        let write = self.write.load(Ordering::Relaxed);
        let read = self.read.load(Ordering::Acquire);
        let cap = self.capacity();
        let next = (write + 1) % cap;
        if next == read {
            return false; // full
        }
        // Safety: we own the write index; no other thread writes to this slot.
        unsafe {
            *self.buffer[write].get() = item;
        }
        self.write.store(next, Ordering::Release);
        true
    }

    /// Pop a single item. Returns None if empty.
    /// Only the consumer thread should call this.
    pub fn pop(&self) -> Option<T> {
        let read = self.read.load(Ordering::Relaxed);
        let write = self.write.load(Ordering::Acquire);
        if read == write {
            return None; // empty
        }
        // Safety: we own the read index; no other thread reads from this slot.
        let item = unsafe { *self.buffer[read].get() };
        let next = (read + 1) % self.capacity();
        self.read.store(next, Ordering::Release);
        Some(item)
    }

    pub fn len(&self) -> usize {
        let write = self.write.load(Ordering::Relaxed);
        let read = self.read.load(Ordering::Relaxed);
        if write >= read {
            write - read
        } else {
            write + self.capacity() - read
        }
    }

    pub fn is_empty(&self) -> bool {
        self.write.load(Ordering::Relaxed) == self.read.load(Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_push_pop() {
        let rb = RingBuf::<u32>::new(8);
        assert!(rb.push(42));
        assert_eq!(rb.pop(), Some(42));
        assert_eq!(rb.pop(), None);
    }

    #[test]
    fn fill_and_drain() {
        let rb = RingBuf::<u32>::new(4);
        assert!(rb.push(1));
        assert!(rb.push(2));
        assert!(rb.push(3));
        assert!(!rb.push(4)); // full (capacity 4, one slot wasted)
        assert_eq!(rb.pop(), Some(1));
        assert_eq!(rb.pop(), Some(2));
        assert_eq!(rb.pop(), Some(3));
        assert_eq!(rb.pop(), None);
    }
}
