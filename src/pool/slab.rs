use std::ops::{Deref, DerefMut};
use std::sync::{Arc, RwLock};

use slab::Slab;

pub(super) struct TypedGuard<'a, T> {
    pool: &'a TypedPool<T>,
    key: usize,
}

impl<'a, T> Drop for TypedGuard<'a, T> {
    fn drop(&mut self) {
        let mut pool = self.pool.0.write().unwrap();
        pool.1.push(self.key);
    }
}

impl<'a, T> Deref for TypedGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        let pool = self.pool.0.read().unwrap();
        let val = pool.0.get(self.key).unwrap();
        unsafe { (val as *const T).as_ref().unwrap() }
    }
}

impl<'a, T> DerefMut for TypedGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        let pool = self.pool.0.read().unwrap();
        let val = pool.0.get(self.key).unwrap();
        unsafe { (val as *const T as *mut T).as_mut().unwrap() }
    }
}

pub(super) struct TypedPool<T>(Arc<RwLock<(Slab<T>, Vec<usize>)>>);

impl<T: Default> TypedPool<T> {
    pub(super) fn new(max_size: usize) -> Self {
        TypedPool(Arc::new(RwLock::new((
            Slab::with_capacity(max_size),
            Vec::with_capacity(max_size),
        ))))
    }

    pub(super) fn get(&self) -> anyhow::Result<TypedGuard<T>> {
        let mut pool = self.0.write().unwrap();
        let key = pool.1.pop().unwrap_or_else(|| pool.0.insert(T::default()));
        Ok(TypedGuard { pool: self, key })
    }
}

pub(super) struct BytesGuard<'a, const SIZE: usize> {
    pool: &'a BytesPool<SIZE>,
    key: usize,
}

impl<'a, const SIZE: usize> Drop for BytesGuard<'a, SIZE> {
    fn drop(&mut self) {
        let mut pool = self.pool.0.write().unwrap();
        pool.1.push(self.key);
    }
}

impl<'a, const SIZE: usize> Deref for BytesGuard<'a, SIZE> {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        let pool = self.pool.0.read().unwrap();
        let vec = pool.0.get(self.key).unwrap();
        unsafe { std::slice::from_raw_parts(vec.as_ptr(), SIZE) }
    }
}

impl<'a, const SIZE: usize> DerefMut for BytesGuard<'a, SIZE> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        let pool = self.pool.0.read().unwrap();
        let vec = pool.0.get(self.key).unwrap();
        unsafe { std::slice::from_raw_parts_mut(vec.as_ptr() as *mut u8, SIZE) }
    }
}

pub(super) struct BytesPool<const SIZE: usize>(Arc<RwLock<(Slab<Vec<u8>>, Vec<usize>)>>);

impl<const SIZE: usize> BytesPool<SIZE> {
    pub(super) fn new(max_size: usize) -> Self {
        BytesPool(Arc::new(RwLock::new((
            Slab::with_capacity(max_size),
            Vec::with_capacity(max_size),
        ))))
    }

    pub(super) fn get(&self) -> anyhow::Result<BytesGuard<SIZE>> {
        let mut pool = self.0.write().unwrap();
        let key = pool
            .1
            .pop()
            .unwrap_or_else(|| pool.0.insert(Vec::with_capacity(SIZE)));
        Ok(BytesGuard { pool: self, key })
    }
}
