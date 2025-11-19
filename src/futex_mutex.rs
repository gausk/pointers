use linux_futex::{Futex, Private};
use std::cell::UnsafeCell;
use std::hint::spin_loop;
use std::ops::{Deref, DerefMut};
use std::sync::atomic::{AtomicBool, Ordering};

pub struct FutexMutex<T> {
    value: UnsafeCell<T>,
    futex: Futex<Private>,
}

unsafe impl<T: Send> Sync for FutexMutex<T> {}

impl<T> FutexMutex<T> {
    pub fn new(value: T) -> Self {
        Self {
            value: UnsafeCell::new(value),
            futex: Futex::new(0),
        }
    }

    pub fn lock(&self) -> FutexMutexGuard<'_, T> {
        while self
            .futex
            .value
            .compare_exchange(0, 1, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            self.futex.wait(1);
        }
        FutexMutexGuard { mutex: self }
    }
}

pub struct FutexMutexGuard<'a, T> {
    mutex: &'a FutexMutex<T>,
}

unsafe impl<T: Sync> Sync for FutexMutexGuard<'_, T> {}

impl<'a, T> Deref for FutexMutexGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &*self.mutex.value.get() }
    }
}

impl<'a, T> DerefMut for FutexMutexGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.mutex.value.get() }
    }
}

impl<T> Drop for FutexMutexGuard<'_, T> {
    fn drop(&mut self) {
        self.mutex.futex.value.store(0, Ordering::Release);
        self.mutex.futex.wake(1);
    }
}

#[cfg(test)]
mod tests {
    use super::FutexMutex;
    use std::sync::Arc;
    use std::thread;
    use std::time::Duration;
    use std::time::SystemTime;

    #[test]
    fn test_futex_mutex() {
        let mutex = Arc::new(FutexMutex::new(0));
        let c_mutex = Arc::clone(&mutex);

        thread::spawn(move || {
            *c_mutex.lock() = 10;
        })
        .join()
        .expect("thread::spawn failed");
        assert_eq!(*mutex.lock(), 10);
    }

    #[test]
    fn test_futex_mutex_contention_increment() {
        let time = SystemTime::now();
        let mutex = Arc::new(FutexMutex::new(0usize));
        let mut handles = vec![];

        for _ in 0..40 {
            let m = Arc::clone(&mutex);
            handles.push(thread::spawn(move || {
                for _ in 0..100 {
                    let mut guard = m.lock();
                    thread::sleep(Duration::from_millis(1));
                    *guard += 1;
                }
            }));
        }

        for h in handles {
            h.join().unwrap();
        }
        assert_eq!(*mutex.lock(), 4000);
        println!(
            "Time taken in my futex Mutex: {}ms",
            time.elapsed().unwrap().as_millis()
        );
    }
}
