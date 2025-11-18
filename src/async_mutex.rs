use std::cell::UnsafeCell;
use tokio::sync::{AcquireError, Semaphore, SemaphorePermit};

pub struct AsyncMutex<T> {
    value: UnsafeCell<T>,
    locked: Semaphore,
}

unsafe impl<T: Send> Send for AsyncMutex<T> {}
unsafe impl<T: Sync> Sync for AsyncMutex<T> {}

impl<T> AsyncMutex<T> {
    pub fn new(value: T) -> AsyncMutex<T> {
        Self {
            value: UnsafeCell::new(value),
            locked: Semaphore::new(1),
        }
    }

    async fn lock(&self) -> Result<MutexGuard<'_, T>, AcquireError> {
        let permit = self.locked.acquire().await?;
        Ok(MutexGuard {
            mutex: self,
            permit,
        })
    }
}

pub struct MutexGuard<'a, T> {
    mutex: &'a AsyncMutex<T>,
    permit: SemaphorePermit<'a>,
}

unsafe impl<T: Sync> Sync for MutexGuard<'_, T> {}

impl<T> std::ops::Deref for MutexGuard<'_, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.mutex.value.get() }
    }
}

impl<T> std::ops::DerefMut for MutexGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.mutex.value.get() }
    }
}

#[cfg(test)]
mod tests {
    use super::AsyncMutex;
    use crate::arc::Arc;
    use std::thread;
    use std::time::SystemTime;

    #[tokio::test]
    async fn test_async_mutex() {
        let mutex = Arc::new(AsyncMutex::new(0));
        let c_mutex = Arc::clone(&mutex);

        tokio::spawn(async move {
            *c_mutex.lock().await.unwrap() = 10;
        })
        .await
        .unwrap();
        assert_eq!(*mutex.lock().await.unwrap(), 10);
    }

    #[tokio::test]
    async fn test_async_mutex_multiple_threads() {
        let time = SystemTime::now();
        let mutex = Arc::new(AsyncMutex::new(0usize));
        let mut handles = vec![];

        for _ in 0..40 {
            let m = mutex.clone();
            handles.push(tokio::spawn(async move {
                for _ in 0..100000 {
                    let mut guard = m.lock().await.unwrap();
                    *guard += 1;
                }
            }));
        }

        for h in handles {
            h.await.unwrap();
        }
        assert_eq!(*mutex.lock().await.unwrap(), 4000000);
        println!(
            "Time taken in my async Mutex: {}ms",
            time.elapsed().unwrap().as_millis()
        );
    }

    #[tokio::test]
    async fn test_std_async_mutex_multiple_threads() {
        use tokio::sync::Mutex;
        let time = SystemTime::now();
        let mutex = Arc::new(Mutex::new(0usize));
        let mut handles = vec![];

        for _ in 0..40 {
            let m = mutex.clone();
            handles.push(tokio::spawn(async move {
                for _ in 0..100000 {
                    let mut guard = m.lock().await;
                    *guard += 1;
                }
            }));
        }

        for h in handles {
            h.await.unwrap();
        }
        assert_eq!(*mutex.lock().await, 4000000);
        println!(
            "Time taken in async std Mutex: {}ms",
            time.elapsed().unwrap().as_millis()
        );
    }
}
