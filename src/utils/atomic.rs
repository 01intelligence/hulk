use std::ops::{Add, Sub};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

/// Atomic extra utility methods with memory ordering [`Ordering::Relaxed`].
pub trait AtomicExt<T: Add + Sub> {
    fn inc(&self);
    fn dec(&self);
    fn add(&self, other: T);
    fn sub(&self, other: T);
    fn get(&self) -> T;
}

impl AtomicExt<u64> for AtomicU64 {
    #[inline(always)]
    fn inc(&self) {
        self.add(1);
    }

    #[inline(always)]
    fn dec(&self) {
        self.sub(1);
    }

    #[inline(always)]
    fn add(&self, other: u64) {
        let _ = self.fetch_add(other, Ordering::Relaxed);
    }

    #[inline(always)]
    fn sub(&self, other: u64) {
        let _ = self.fetch_sub(other, Ordering::Relaxed);
    }

    fn get(&self) -> u64 {
        self.load(Ordering::Relaxed)
    }
}

impl AtomicExt<usize> for AtomicUsize {
    #[inline(always)]
    fn inc(&self) {
        self.add(1);
    }

    #[inline(always)]
    fn dec(&self) {
        self.sub(1);
    }

    #[inline(always)]
    fn add(&self, other: usize) {
        let _ = self.fetch_add(other, Ordering::Relaxed);
    }

    #[inline(always)]
    fn sub(&self, other: usize) {
        let _ = self.fetch_sub(other, Ordering::Relaxed);
    }

    fn get(&self) -> usize {
        self.load(Ordering::Relaxed)
    }
}
