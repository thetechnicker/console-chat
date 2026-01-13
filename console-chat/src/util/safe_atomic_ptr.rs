use derive_deref::{Deref, DerefMut};
use std::ptr::null_mut;
use std::sync::atomic::{AtomicPtr, Ordering};

#[derive(Debug, Deref, DerefMut)]
pub struct SafeAtomicPtr<T> {
    ptr: AtomicPtr<T>,
}
impl<T: Default> Default for SafeAtomicPtr<T> {
    fn default() -> Self {
        Self {
            ptr: AtomicPtr::new(Box::leak(Box::new(T::default()))),
        }
    }
}

impl<T> SafeAtomicPtr<T> {
    fn new(value: T) -> Self {
        Self {
            ptr: AtomicPtr::new(Box::leak(Box::new(value))),
        }
    }

    fn load(&self) -> Option<&T> {
        let ptr = self.ptr.load(Ordering::SeqCst);
        if ptr.is_null() {
            None
        } else {
            unsafe { Some(&*ptr) }
        }
    }

    fn store(&self, value: T) {
        let ptr = self.ptr.swap(Box::leak(Box::new(value)), Ordering::SeqCst);
        if !ptr.is_null() {
            unsafe { drop(Box::from_raw(ptr)) }
        }
    }
}

impl<T> Drop for SafeAtomicPtr<T> {
    fn drop(&mut self) {
        let ptr = self.ptr.swap(null_mut(), Ordering::SeqCst);
        if !ptr.is_null() {
            unsafe { drop(Box::from_raw(ptr)) }
        }
    }
}

fn test_attributes<T>() {
    fn is_send<T: Send>() {}
    fn is_sync<T: Sync>() {}

    is_send::<SafeAtomicPtr<T>>();
    is_sync::<SafeAtomicPtr<T>>();
}
