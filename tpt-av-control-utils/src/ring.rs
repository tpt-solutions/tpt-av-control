//! A bounded, lock-free single-producer / single-consumer ring buffer.
//!
//! This is the bridge between the network/MIDI threads (which may allocate)
//! and the real-time audio/video thread (which may not): the RT thread only
//! calls [`SpscRing::pop`], which never allocates, never blocks, and never
//! takes a lock.
//!
//! Safety note: this module contains the only `unsafe` code in the
//! workspace. It implements the standard bounded SPSC algorithm (head/tail
//! monotonic counters with acquire/release ordering); the invariants it
//! relies on are:
//!
//! - exactly one thread calls [`push`](SpscRing::push), exactly one calls
//!   [`pop`](SpscRing::pop);
//! - capacity is fixed at construction and slots are only written by the
//!   producer between `tail..head + capacity` and read by the consumer
//!   between `head..tail`.

use std::cell::UnsafeCell;
use std::mem::MaybeUninit;
use std::sync::atomic::{AtomicUsize, Ordering};

use super::ControlError;

/// A bounded lock-free SPSC queue.
///
/// Created with a fixed capacity; after construction, `push`/`pop` perform
/// no allocation. Send + Sync for `T: Send`.
pub struct SpscRing<T> {
    slots: Box<[UnsafeCell<MaybeUninit<T>>]>,
    capacity_mask: usize,
    head: AtomicUsize, // consumer cursor (next index to pop)
    tail: AtomicUsize, // producer cursor (next index to push)
}

// SPSC: the producer only touches `tail` and slots in `tail..head+cap`, the
// consumer only `head` and slots in `head..tail`. `T` crosses threads via
// the acquire/release pairs in push/pop.
unsafe impl<T: Send> Send for SpscRing<T> {}
unsafe impl<T: Send> Sync for SpscRing<T> {}

impl<T> SpscRing<T> {
    /// Creates a ring with the given capacity (rounded up to a power of two).
    /// This is the only allocation the ring ever performs.
    pub fn new(capacity: usize) -> Self {
        let capacity = capacity.max(1).next_power_of_two();
        let mut slots = Vec::with_capacity(capacity);
        for _ in 0..capacity {
            slots.push(UnsafeCell::new(MaybeUninit::uninit()));
        }
        Self {
            slots: slots.into_boxed_slice(),
            capacity_mask: capacity - 1,
            head: AtomicUsize::new(0),
            tail: AtomicUsize::new(0),
        }
    }

    /// The usable capacity of the ring.
    pub fn capacity(&self) -> usize {
        self.capacity_mask + 1
    }

    /// Current number of items in the ring (a snapshot; the other thread
    /// may change it immediately after).
    pub fn len(&self) -> usize {
        let tail = self.tail.load(Ordering::Acquire);
        let head = self.head.load(Ordering::Acquire);
        tail.wrapping_sub(head)
    }

    /// Whether the ring currently holds no items.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// # Examples
    ///
    /// ```
    /// use tpt_av_control_utils::SpscRing;
    /// let ring: SpscRing<u64> = SpscRing::new(8);
    /// ring.push(1).unwrap();
    /// assert_eq!(ring.pop(), Some(1));
    /// assert_eq!(ring.pop(), None);
    /// ```
    /// Pushes an item (producer side). Returns it back via [`ControlError`]
    /// if the ring is full. Never allocates or blocks.
    pub fn push(&self, value: T) -> Result<(), ControlError> {
        let tail = self.tail.load(Ordering::Relaxed);
        let head = self.head.load(Ordering::Acquire);
        if tail.wrapping_sub(head) > self.capacity_mask {
            return Err(ControlError::QueueFull);
        }
        // Producer-only write into the slot one behind the consumer window.
        unsafe {
            (*self.slot(tail).get()).as_mut_ptr().write(value);
        }
        self.tail.store(tail.wrapping_add(1), Ordering::Release);
        Ok(())
    }

    /// Pops an item (consumer side). Returns `None` when empty. Never
    /// allocates or blocks — safe for the real-time thread.
    pub fn pop(&self) -> Option<T> {
        let head = self.head.load(Ordering::Relaxed);
        let tail = self.tail.load(Ordering::Acquire);
        if head == tail {
            return None;
        }
        // Consumer-only read of a slot the producer has released.
        let value = unsafe { (*self.slot(head).get()).as_ptr().read() };
        self.head.store(head.wrapping_add(1), Ordering::Release);
        Some(value)
    }

    fn slot(&self, index: usize) -> &UnsafeCell<MaybeUninit<T>> {
        &self.slots[index & self.capacity_mask]
    }
}

impl<T> Drop for SpscRing<T> {
    fn drop(&mut self) {
        // Both threads are gone by the time the ring is dropped; drop any
        // items still in flight.
        let tail = self.tail.load(Ordering::Relaxed);
        let head = self.head.load(Ordering::Relaxed);
        for index in head..tail {
            unsafe {
                // SAFETY: slots in head..tail were written by the producer
                // and not yet read by the consumer.
                (*self.slot(index).get()).assume_init_drop();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn push_pop_in_order() {
        let ring: SpscRing<u64> = SpscRing::new(4);
        assert!(ring.is_empty());
        for i in 0..4 {
            ring.push(i).unwrap();
        }
        assert!(ring.push(4).is_err());
        for i in 0..4 {
            assert_eq!(ring.pop(), Some(i));
        }
        assert_eq!(ring.pop(), None);
    }

    #[test]
    fn wraps_around() {
        let ring: SpscRing<u64> = SpscRing::new(2);
        ring.push(1).unwrap();
        assert_eq!(ring.pop(), Some(1));
        ring.push(2).unwrap();
        ring.push(3).unwrap();
        assert_eq!(ring.pop(), Some(2));
        assert_eq!(ring.pop(), Some(3));
    }

    #[test]
    fn capacity_rounds_to_power_of_two() {
        let ring: SpscRing<u8> = SpscRing::new(5);
        assert_eq!(ring.capacity(), 8);
    }

    #[test]
    fn two_threads_transfer_all_items() {
        let ring: Arc<SpscRing<u64>> = Arc::new(SpscRing::new(64));
        let producer = Arc::clone(&ring);
        let consumer = Arc::clone(&ring);
        const COUNT: u64 = 100_000;
        let p = thread::spawn(move || {
            for i in 0..COUNT {
                while producer.push(i).is_err() {
                    std::hint::spin_loop();
                }
            }
        });
        let c = thread::spawn(move || {
            for i in 0..COUNT {
                loop {
                    if let Some(v) = consumer.pop() {
                        assert_eq!(v, i);
                        break;
                    }
                    std::hint::spin_loop();
                }
            }
        });
        p.join().unwrap();
        c.join().unwrap();
    }

    #[test]
    fn drop_flushes_pending_items() {
        use std::sync::atomic::AtomicUsize;
        static DROPS: AtomicUsize = AtomicUsize::new(0);
        struct Counted(#[allow(dead_code)] usize);
        impl Drop for Counted {
            fn drop(&mut self) {
                DROPS.fetch_add(1, Ordering::SeqCst);
            }
        }
        DROPS.store(0, Ordering::SeqCst);
        {
            let ring = SpscRing::new(8);
            for i in 0..5 {
                ring.push(Counted(i)).unwrap();
            }
            assert_eq!(DROPS.load(Ordering::SeqCst), 0);
        }
        assert_eq!(DROPS.load(Ordering::SeqCst), 5);
    }
}
