use std::cell::RefCell;
use std::fmt;

/// A wrapper type where `clone()` acts like a move operation.
///
/// When cloned, the value is taken from the source, leaving it empty.
/// This violates the typical Clone contract but is useful in specific scenarios
/// where Clone bounds are required but move semantics are desired.
#[derive(PartialEq)]
pub struct TakeOnClone<T> {
    inner: RefCell<Option<T>>,
}

impl<T> TakeOnClone<T> {
    /// Creates a new `TakeOnClone` containing a value.
    pub fn new(value: T) -> Self {
        Self {
            inner: RefCell::new(Some(value)),
        }
    }

    /// Creates an empty `TakeOnClone`.
    pub fn empty() -> Self {
        Self {
            inner: RefCell::new(None),
        }
    }

    /// Takes the value out, leaving the source empty.
    pub fn take(&self) -> Option<T> {
        self.inner.borrow_mut().take()
    }

    /// Gets a reference to the inner value if present.
    pub fn get(&self) -> Option<std::cell::Ref<'_, T>> {
        let borrow = self.inner.borrow();
        if borrow.is_some() {
            Some(std::cell::Ref::map(borrow, |opt| opt.as_ref().unwrap()))
        } else {
            None
        }
    }

    /// Gets a mutable reference to the inner value if present.
    pub fn get_mut(&self) -> Option<std::cell::RefMut<'_, T>> {
        let borrow = self.inner.borrow_mut();
        if borrow.is_some() {
            Some(std::cell::RefMut::map(borrow, |opt| opt.as_mut().unwrap()))
        } else {
            None
        }
    }

    /// Checks if the value is present.
    pub fn is_some(&self) -> bool {
        self.inner.borrow().is_some()
    }

    /// Checks if the value is absent.
    pub fn is_none(&self) -> bool {
        self.inner.borrow().is_none()
    }

    /// Unwraps the value, panicking if empty.
    pub fn unwrap(self) -> T {
        self.inner
            .into_inner()
            .expect("called unwrap on empty TakeOnClone")
    }

    /// Replaces the value, returning the old value if any.
    pub fn replace(&self, value: T) -> Option<T> {
        self.inner.borrow_mut().replace(value)
    }
}

impl<T> Clone for TakeOnClone<T> {
    /// Clones by taking the value from the source.
    ///
    /// After this operation, the source will be empty.
    /// If the source is already empty, the clone will also be empty.
    fn clone(&self) -> Self {
        Self {
            inner: RefCell::new(self.inner.borrow_mut().take()),
        }
    }
}

impl<T: fmt::Debug> fmt::Debug for TakeOnClone<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TakeOnClone")
            .field("inner", &self.inner.borrow())
            .finish()
    }
}

impl<T> Default for TakeOnClone<T> {
    fn default() -> Self {
        Self::empty()
    }
}

impl<T> From<T> for TakeOnClone<T> {
    fn from(value: T) -> Self {
        Self::new(value)
    }
}

impl<T> From<Option<T>> for TakeOnClone<T> {
    fn from(value: Option<T>) -> Self {
        Self {
            inner: RefCell::new(value),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn test_clone_moves_value() {
        let original = TakeOnClone::new(42);
        assert!(original.is_some());

        let cloned = original.clone();

        // Original is now empty
        assert!(original.is_none());
        // Clone has the value
        assert!(cloned.is_some());
        assert_eq!(*cloned.get().unwrap(), 42);
    }

    #[test]
    fn test_clone_empty() {
        let original = TakeOnClone::<i32>::empty();
        let cloned = original.clone();

        assert!(original.is_none());
        assert!(cloned.is_none());
    }

    #[test]
    fn test_with_arc() {
        let data = Arc::new(vec![1, 2, 3]);
        let original = TakeOnClone::new(data);

        let cloned = original.clone();

        assert!(original.is_none());
        assert_eq!(cloned.get().unwrap().len(), 3);
    }

    #[test]
    fn test_multiple_clones() {
        let original = TakeOnClone::new(String::from("hello"));

        let first = original.clone();
        assert!(original.is_none());
        assert!(first.is_some());

        let second = original.clone();
        assert!(second.is_none());

        let third = first.clone();
        assert!(first.is_none());
        assert_eq!(*third.get().unwrap(), "hello");
    }

    #[test]
    fn test_take() {
        let container = TakeOnClone::new(100);

        let value = container.take();
        assert_eq!(value, Some(100));
        assert!(container.is_none());

        let value2 = container.take();
        assert_eq!(value2, None);
    }

    #[test]
    fn test_replace() {
        let container = TakeOnClone::new(1);

        let old = container.replace(2);
        assert_eq!(old, Some(1));
        assert_eq!(*container.get().unwrap(), 2);
    }
}
