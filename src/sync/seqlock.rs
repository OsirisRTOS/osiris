use core::cell::UnsafeCell;
use core::hint::spin_loop;
use core::sync::atomic::{AtomicUsize, Ordering};

/// Single-writer, multi-reader seqlock.
///
/// Odd `seq` means a write is in progress; even means data is stable.
/// Readers spin on odd seq and retry if seq changes while reading.
pub struct Seqlock<T> {
    seq: AtomicUsize,
    data: UnsafeCell<T>,
}

unsafe impl<T: Send> Send for Seqlock<T> {}
unsafe impl<T: Send> Sync for Seqlock<T> {}

impl<T: Copy> Seqlock<T> {
    pub const fn new(val: T) -> Self {
        Self {
            seq: AtomicUsize::new(0),
            data: UnsafeCell::new(val),
        }
    }

    /// Overwrite the value. Only one writer at a time is supported.
    pub fn write(&self, val: T) {
        self.seq.fetch_add(1, Ordering::SeqCst); // even → odd: write in progress
        unsafe { core::ptr::write_volatile(self.data.get(), val) };
        self.seq.fetch_add(1, Ordering::SeqCst); // odd → even: write complete
    }

    /// Read the current value. Retries if a write is in progress or races a write.
    pub fn read(&self) -> T {
        loop {
            let seq1 = self.seq.load(Ordering::SeqCst);
            if seq1 & 1 != 0 {
                spin_loop();
                continue;
            }
            // Safety: seq is even so no write is in progress on this single-core target.
            let val = unsafe { core::ptr::read_volatile(self.data.get()) };
            let seq2 = self.seq.load(Ordering::SeqCst);
            if seq1 == seq2 {
                return val;
            }
        }
    }
}
