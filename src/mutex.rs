use std::cell::UnsafeCell;
use std::hint::spin_loop;
use std::ops::{Deref, DerefMut};
use std::sync::atomic::{AtomicBool, Ordering};

/// A mutual exclusion primitive useful for protecting shared data
///
/// This mutex will block threads waiting for the lock to become available.
pub struct Mutex<T> {
    value: UnsafeCell<T>,
    locked: AtomicBool,
}

unsafe impl<T: Send> Sync for Mutex<T> {}

impl<T> Mutex<T> {
    pub fn new(value: T) -> Self {
        Self {
            value: UnsafeCell::new(value),
            locked: AtomicBool::new(false),
        }
    }

    pub fn lock(&self) -> MutexGuard<'_, T> {
        while self
            .locked
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            // std::Mutex uses futex internally using libc hence perform better than us.
            spin_loop();
        }
        MutexGuard { mutex: self }
    }
}

pub struct MutexGuard<'a, T> {
    mutex: &'a Mutex<T>,
}

unsafe impl<T: Sync> Sync for MutexGuard<'_, T> {}

impl<'a, T> Deref for MutexGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &*self.mutex.value.get() }
    }
}

impl<'a, T> DerefMut for MutexGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.mutex.value.get() }
    }
}

impl<T> Drop for MutexGuard<'_, T> {
    fn drop(&mut self) {
        self.mutex.locked.store(false, Ordering::Release);
    }
}

#[cfg(test)]
mod tests {
    use super::Mutex;
    use std::sync::Arc;
    use std::thread;
    use std::time::SystemTime;

    #[test]
    fn test_mutex() {
        let mutex = Arc::new(Mutex::new(0));
        let c_mutex = Arc::clone(&mutex);

        thread::spawn(move || {
            *c_mutex.lock() = 10;
        })
        .join()
        .expect("thread::spawn failed");
        assert_eq!(*mutex.lock(), 10);
    }

    #[test]
    fn test_mutex_contention_increment() {
        let time = SystemTime::now();
        let mutex = Arc::new(Mutex::new(0usize));
        let mut handles = vec![];

        for _ in 0..40 {
            let m = Arc::clone(&mutex);
            handles.push(thread::spawn(move || {
                for _ in 0..100000 {
                    let mut guard = m.lock();
                    *guard += 1;
                }
            }));
        }

        for h in handles {
            h.join().unwrap();
        }
        assert_eq!(*mutex.lock(), 4000000);
        println!(
            "Time taken in my Mutex: {}ms",
            time.elapsed().unwrap().as_millis()
        );
    }

    #[test]
    fn test_mutex_contention_with_std_mutex() {
        use std::sync::Mutex;
        let time = SystemTime::now();
        let mutex = Arc::new(Mutex::new(0usize));
        let mut handles = vec![];

        for _ in 0..40 {
            let m = Arc::clone(&mutex);
            handles.push(thread::spawn(move || {
                for _ in 0..100000 {
                    let mut guard = m.lock().unwrap();
                    *guard += 1;
                }
            }));
        }

        for h in handles {
            h.join().unwrap();
        }
        assert_eq!(*mutex.lock().unwrap(), 4000000);
        println!(
            "Time taken in std Mutex: {}ms",
            time.elapsed().unwrap().as_millis()
        );
    }
}
