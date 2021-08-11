use std::ops::{Deref, DerefMut};

use anyhow::Error;

use crate::pool::slab::BytesGuard;

mod deadpool;
mod slab;

pub struct TypedPoolGuard<'a, T: Default + Sync + Send>(TypedPoolGuardInner<'a, T>);
enum TypedPoolGuardInner<'a, T: Default + Sync + Send> {
    DeadPool(deadpool::TypedGuard<T>),
    Slab(slab::TypedGuard<'a, T>),
}

impl<'a, T: Default + Sync + Send> Deref for TypedPoolGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        match &self.0 {
            TypedPoolGuardInner::DeadPool(guard) => guard.deref(),
            TypedPoolGuardInner::Slab(guard) => guard.deref(),
        }
    }
}

impl<'a, T: Default + Sync + Send> DerefMut for TypedPoolGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match &mut self.0 {
            TypedPoolGuardInner::DeadPool(guard) => guard.deref_mut(),
            TypedPoolGuardInner::Slab(guard) => guard.deref_mut(),
        }
    }
}

pub struct TypedPool<T: Default + Sync + Send>(TypedPoolInner<T>);
enum TypedPoolInner<T: Default + Sync + Send> {
    DeadPool(deadpool::TypedPool<T>),
    Slab(slab::TypedPool<T>),
}

impl<T: Default + Sync + Send> TypedPool<T> {
    pub fn new(max_size: usize) -> Self {
        TypedPool(TypedPoolInner::DeadPool(deadpool::TypedPool::new(max_size)))
    }

    async fn get(&self) -> anyhow::Result<TypedPoolGuard<'_, T>> {
        match &self.0 {
            TypedPoolInner::DeadPool(pool) => match pool.try_get().await {
                Ok(guard) => Ok(TypedPoolGuard(TypedPoolGuardInner::DeadPool(guard.into()))),
                Err(err) => Err(err.into()),
            },
            TypedPoolInner::Slab(pool) => match pool.get() {
                Ok(guard) => Ok(TypedPoolGuard(TypedPoolGuardInner::Slab(guard))),
                Err(err) => Err(err),
            },
        }
    }
}

pub struct BytesPoolGuard<'a, const SIZE: usize>(BytesPoolGuardInner<'a, SIZE>);
enum BytesPoolGuardInner<'a, const SIZE: usize> {
    DeadPool(deadpool::BytesGuard<SIZE>),
    Slab(slab::BytesGuard<'a, SIZE>),
}

impl<'a, const SIZE: usize> Deref for BytesPoolGuard<'a, SIZE> {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        match &self.0 {
            BytesPoolGuardInner::DeadPool(pool) => pool.deref(),
            BytesPoolGuardInner::Slab(pool) => pool.deref(),
        }
    }
}

impl<'a, const SIZE: usize> DerefMut for BytesPoolGuard<'a, SIZE> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match &mut self.0 {
            BytesPoolGuardInner::DeadPool(pool) => pool.deref_mut(),
            BytesPoolGuardInner::Slab(pool) => pool.deref_mut(),
        }
    }
}

pub struct BytesPool<const SIZE: usize>(BytesPoolInner<SIZE>);
enum BytesPoolInner<const SIZE: usize> {
    DeadPool(deadpool::BytesPool<SIZE>),
    Slab(slab::BytesPool<SIZE>),
}

impl<const SIZE: usize> BytesPool<SIZE> {
    pub fn new(max_size: usize) -> Self {
        BytesPool(BytesPoolInner::DeadPool(deadpool::BytesPool::new(max_size)))
    }

    async fn get(&self) -> anyhow::Result<BytesPoolGuard<'_, SIZE>> {
        match &self.0 {
            BytesPoolInner::DeadPool(pool) => match pool.try_get().await {
                Ok(guard) => Ok(BytesPoolGuard(BytesPoolGuardInner::DeadPool(guard.into()))),
                Err(err) => Err(err.into()),
            },
            BytesPoolInner::Slab(pool) => match pool.get() {
                Ok(guard) => Ok(BytesPoolGuard(BytesPoolGuardInner::Slab(guard))),
                Err(err) => Err(err),
            },
        }
    }
}
