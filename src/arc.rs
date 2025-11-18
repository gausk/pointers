use std::marker::PhantomData;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Debug)]
pub struct Arc<T> {
    ptr: NonNull<ArcInner<T>>,
    _marker: PhantomData<ArcInner<T>>,
}

unsafe impl<T: Send + Sync> Send for Arc<T> {}
unsafe impl<T: Sync + Send> Sync for Arc<T> {}

pub struct ArcInner<T> {
    data: T,
    owner: AtomicUsize,
}

unsafe impl<T: Send + Sync> Send for ArcInner<T> {}
unsafe impl<T: Sync + Send> Sync for ArcInner<T> {}

impl<T> Clone for Arc<T> {
    fn clone(&self) -> Self {
        let ptr = unsafe { self.ptr.as_ref() };
        ptr.owner.fetch_add(1, Ordering::Relaxed);
        Self {
            ptr: self.ptr,
            _marker: PhantomData,
        }
    }
}

impl<T> std::ops::Deref for Arc<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        unsafe { &self.ptr.as_ref().data }
    }
}

impl<T> Arc<T> {
    pub fn new(data: T) -> Arc<T> {
        let inner = ArcInner {
            data,
            owner: AtomicUsize::new(1),
        };
        let data = Box::new(inner);
        Self {
            ptr: unsafe { NonNull::new_unchecked(Box::into_raw(data)) },
            _marker: PhantomData,
        }
    }
}

impl<T> Drop for Arc<T> {
    fn drop(&mut self) {
        let inner = unsafe { self.ptr.as_ref() };
        if inner.owner.fetch_sub(1, Ordering::Release) == 1 {
            std::sync::atomic::fence(Ordering::Acquire);
            unsafe { drop(Box::from_raw(self.ptr.as_ptr())) };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Arc;
    use std::sync::atomic::Ordering;

    #[test]
    fn test_arc() {
        use super::Arc;
        use std::thread;

        let five = Arc::new(5);
        let mut handles = vec![];

        for _ in 0..10 {
            let current = five.clone();
            handles.push(thread::spawn(move || {
                println!("{}", *current);
            }));
        }

        for h in handles {
            h.join().unwrap();
        }
    }

    #[test]
    fn drop_once() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        struct Counter<'a>(&'a AtomicUsize);
        impl<'a> Drop for Counter<'a> {
            fn drop(&mut self) {
                self.0.fetch_add(1, Ordering::SeqCst);
            }
        }

        let d = AtomicUsize::new(0);

        {
            let a = Arc::new(Counter(&d));
            let b = a.clone();
            let c = b.clone();
            drop(a);
            drop(b);
            drop(c);
        }

        assert_eq!(d.load(Ordering::SeqCst), 1, "Drop must happen exactly once");
    }

    #[test]
    fn clone_increments_count() {
        let a = Arc::new(10);
        let b = a.clone();
        let c = b.clone();

        let inner = unsafe { a.ptr.as_ref() };
        assert_eq!(inner.owner.load(Ordering::Relaxed), 3);
    }

    #[test]
    fn concurrent_clones_and_drops() {
        use std::thread;

        let a = Arc::new(123);
        let mut handles = vec![];

        for _ in 0..100 {
            let x = a.clone();
            handles.push(thread::spawn(move || {
                let _y = x.clone();
                let _z = x.clone();
            }));
        }

        for h in handles {
            h.join().unwrap();
        }

        // only the original Arc should remain
        let inner = unsafe { a.ptr.as_ref() };
        assert_eq!(inner.owner.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn thread_access_after_clones() {
        use std::thread;

        let a = Arc::new(99);
        let mut handles = vec![];

        for _ in 0..50 {
            let x = a.clone();
            handles.push(thread::spawn(move || {
                assert_eq!(*x, 99);
            }));
        }

        for h in handles {
            h.join().unwrap();
        }
    }

    #[test]
    fn multiple_readers() {
        let a = Arc::new(vec![1, 2, 3]);

        let b = a.clone();
        let c = b.clone();

        assert_eq!(a[1], 2);
        assert_eq!(b[1], 2);
        assert_eq!(c[1], 2);
    }
}
