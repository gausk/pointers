use std::cell::UnsafeCell;
use std::ops::{Deref, DerefMut};
use std::sync::atomic::AtomicIsize;
use std::sync::atomic::Ordering;

/// This type of lock allows a number of readers or at most one writer at any point in time.
/// The write portion of this lock typically allows modification of the underlying data (exclusive access)
/// and the read portion of this lock typically allows for read-only access (shared access).
pub struct RwLock<T> {
    value: UnsafeCell<T>,
    // -1 -> Write, 0 -> Nobody >1 -> Read
    state: AtomicIsize,
}

unsafe impl<T: Send> Send for RwLock<T> {}
unsafe impl<T: Send + Sync> Sync for RwLock<T> {}

impl<T> RwLock<T> {
    pub fn new(value: T) -> RwLock<T> {
        RwLock {
            value: UnsafeCell::new(value),
            state: AtomicIsize::new(0),
        }
    }

    pub fn read(&self) -> RwLockReadGuard<'_, T> {
        while self
            .state
            .fetch_update(Ordering::Acquire, Ordering::Relaxed, |x| {
                if x < 0 { None } else { Some(x + 1) }
            })
            .is_err()
        {}
        RwLockReadGuard { lock: self }
    }

    pub fn write(&self) -> RwLockWriteGuard<'_, T> {
        while self
            .state
            .compare_exchange(0, -1, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            std::hint::spin_loop();
        }
        RwLockWriteGuard { lock: self }
    }
}

pub struct RwLockReadGuard<'a, T> {
    lock: &'a RwLock<T>,
}

impl<T> Deref for RwLockReadGuard<'_, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.lock.value.get() }
    }
}

impl<T> Drop for RwLockReadGuard<'_, T> {
    fn drop(&mut self) {
        let prev_value = self.lock.state.fetch_sub(1, Ordering::Release);
        assert!(prev_value >= 1);
    }
}

pub struct RwLockWriteGuard<'a, T> {
    lock: &'a RwLock<T>,
}

impl<T> Deref for RwLockWriteGuard<'_, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.lock.value.get() }
    }
}

impl<T> DerefMut for RwLockWriteGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.lock.value.get() }
    }
}

impl<T> Drop for RwLockWriteGuard<'_, T> {
    fn drop(&mut self) {
        self.lock.state.store(0, Ordering::Release);
    }
}

#[cfg(test)]
mod tests {
    use super::RwLock;

    #[test]
    fn test_rwlock() {
        let lock = RwLock::new(5);

        // many reader locks can be held at once
        {
            let r1 = lock.read();
            let r2 = lock.read();
            assert_eq!(*r1, 5);
            assert_eq!(*r2, 5);
        } // read locks are dropped at this point

        // only one write lock may be held, however
        {
            let mut w = lock.write();
            *w += 1;
            assert_eq!(*w, 6);
        }
    }

    #[test]
    fn test_parallel_readers() {
        use std::sync::Arc;
        use std::thread;

        let lock = Arc::new(super::RwLock::new(123));

        let mut threads = vec![];

        for _ in 0..10 {
            let lk = lock.clone();
            threads.push(thread::spawn(move || {
                for _ in 0..1_000_00 {
                    let r = lk.read();
                    assert_eq!(*r, 123);
                }
            }));
        }

        for t in threads {
            t.join().unwrap();
        }
    }

    #[test]
    fn test_writer_exclusivity() {
        use std::sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        };
        use std::thread;

        let lock = Arc::new(super::RwLock::new(0));
        let active_writers = Arc::new(AtomicUsize::new(0));
        let max_writers = Arc::new(AtomicUsize::new(0));

        let mut threads = vec![];

        for _ in 0..8 {
            let lk = lock.clone();
            let aw = active_writers.clone();
            let mw = max_writers.clone();
            threads.push(thread::spawn(move || {
                for _ in 0..1000 {
                    let _w = lk.write();

                    // Track concurrent writers.
                    let count = aw.fetch_add(1, Ordering::SeqCst) + 1;
                    mw.fetch_max(count, Ordering::SeqCst);

                    // simulate "work"
                    std::thread::yield_now();

                    aw.fetch_sub(1, Ordering::SeqCst);
                }
            }));
        }

        for t in threads {
            t.join().unwrap();
        }

        assert_eq!(
            max_writers.load(Ordering::SeqCst),
            1,
            "More than one writer entered critical section!"
        );
    }
}
