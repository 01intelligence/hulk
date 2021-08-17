use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};

use async_trait::async_trait;
use deadpool::managed::{Manager, Object, Pool, RecycleResult};

pub(super) struct TypedGuard<T: Default + Send>(Object<TypedManager<T>>);

pub(super) struct TypedPool<T: Default + Send>(Pool<TypedManager<T>>);

impl<T: Default + Send> From<Object<TypedManager<T>>> for TypedGuard<T> {
    fn from(obj: Object<TypedManager<T>>) -> Self {
        TypedGuard(obj)
    }
}

impl<T: Default + Send> Deref for TypedGuard<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}

impl<T: Default + Send> DerefMut for TypedGuard<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.deref_mut()
    }
}

impl<T: Default + Send> TypedPool<T> {
    pub(super) fn new(max_size: usize) -> Self {
        TypedPool(Pool::new(TypedManager(PhantomData), max_size))
    }
}

impl<T: Default + Send> Deref for TypedPool<T> {
    type Target = Pool<TypedManager<T>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub(super) struct TypedManager<T>(PhantomData<fn() -> T>);

#[async_trait]
impl<T: Default + Send> Manager for TypedManager<T> {
    type Type = T;
    type Error = std::convert::Infallible;

    async fn create(&self) -> Result<Self::Type, Self::Error> {
        Ok(T::default())
    }

    async fn recycle(&self, _: &mut Self::Type) -> RecycleResult<Self::Error> {
        Ok(())
    }
}

pub(super) struct BytesGuard<const SIZE: usize>(Object<BytesManager<SIZE>>);

pub(super) struct BytesPool<const SIZE: usize>(Pool<BytesManager<SIZE>>);

impl<const SIZE: usize> From<Object<BytesManager<SIZE>>> for BytesGuard<SIZE> {
    fn from(obj: Object<BytesManager<SIZE>>) -> Self {
        BytesGuard(obj)
    }
}

impl<const SIZE: usize> Deref for BytesGuard<SIZE> {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.0.deref()[..]
    }
}

impl<const SIZE: usize> DerefMut for BytesGuard<SIZE> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0.deref_mut()[..]
    }
}

impl<const SIZE: usize> BytesPool<SIZE> {
    pub(super) fn new(max_size: usize) -> Self {
        BytesPool(Pool::new(BytesManager, max_size))
    }
}

impl<const SIZE: usize> Deref for BytesPool<SIZE> {
    type Target = Pool<BytesManager<SIZE>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub(super) struct BytesManager<const SIZE: usize>;

#[async_trait]
impl<const SIZE: usize> Manager for BytesManager<SIZE> {
    type Type = Vec<u8>;
    type Error = std::convert::Infallible;

    async fn create(&self) -> Result<Self::Type, Self::Error> {
        Ok(vec![0u8; SIZE])
    }

    async fn recycle(&self, _: &mut Self::Type) -> RecycleResult<Self::Error> {
        Ok(())
    }
}
