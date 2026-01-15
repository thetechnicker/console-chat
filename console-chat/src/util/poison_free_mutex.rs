use std::sync::{Mutex, MutexGuard, PoisonError};

#[derive(Debug)]
pub struct PoisonFreeMutex<T> {
    inner: Mutex<T>,
    on_reset: fn(PoisonError<MutexGuard<T>>) -> MutexGuard<T>,
}

impl<T> PoisonFreeMutex<T>
where
    T: Default,
{
    pub fn new_default(value: T) -> Self {
        Self {
            inner: Mutex::new(value),
            on_reset: Self::reset,
        }
    }

    fn reset(poison: PoisonError<MutexGuard<T>>) -> MutexGuard<T> {
        let mut guard = poison.into_inner();
        *guard = T::default();
        guard
    }
}

impl<T> PoisonFreeMutex<T> {
    pub fn new(value: T, reset: fn(PoisonError<MutexGuard<T>>) -> MutexGuard<T>) -> Self {
        Self {
            inner: Mutex::new(value),
            on_reset: reset,
        }
    }

    pub fn lock(&self) -> MutexGuard<T> {
        match self.inner.lock() {
            Ok(guard) => guard,
            Err(poisoned) => (self.on_reset)(poisoned),
        }
    }
}

// Example usage
#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn basic_test() {
        let mutex = PoisonFreeMutex::new(0, |poisoned| {
            // Reset value when poisoned
            println!("Mutex poisoned; resetting.");
            poisoned.into_inner()
        });

        {
            let mut data = mutex.lock();
            *data += 1; // Modify the data
        }

        println!("Data: {:?}", *mutex.lock());
    }
}
