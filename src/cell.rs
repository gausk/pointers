use std::cell::UnsafeCell;

/// Cell<T> implements interior mutability by moving values in and out of the cell.
/// That is, an &mut T to the inner value can never be obtained, and the value itself
/// cannot be directly obtained without replacing it with something else.
/// Cell are also not Sync, so can't be shared across threads.
pub struct Cell<T> {
    value: UnsafeCell<T>,
}

impl<T> Cell<T> {
    pub fn new(value: T) -> Self {
        Self {
            value: UnsafeCell::new(value),
        }
    }

    pub fn set(&self, value: T) {
        // SAFETY: we know no-one else is concurrently mutating self.value (because !Sync)
        // SAFETY: we know we are not invalidating any references because we never gave any out.
        unsafe { *self.value.get() = value };
    }

    pub fn get(&self) -> T
    where
        T: Copy,
    {
        // SAFETY: we know no-one else is modifying this value, since only this thread can mutate as !Sync
        // and only get or set can be called at one time.
        unsafe { *self.value.get() }
    }
}

/// 1. Getting a raw *mut T from an &T does NOT remove Rust’s aliasing guarantees — the compiler still assumes the
/// data behind &T is immutable, so mutating it through a raw pointer is undefined behavior.
/// 2. UnsafeCell<T> is the only type that tells the compiler the data may be mutated through shared references,
/// preventing misoptimizations. All interior-mutability types must use it.

/// Implied by UnsafeCell as variable impl<T> !Sync for Cell<T> {}
/// ```compile_fail
///  use std::sync::Arc;
///  let cell1 = Arc::new(pointers::cell::Cell::new(0));
///  std::thread::spawn(|| { cell1.set(1); });
/// ```
struct ThreadUnsafeTest {}

#[cfg(test)]
mod tests {
    use super::Cell;
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn test_cell() {
        struct SomeStruct {
            regular_field: u8,
            special_field: Cell<u8>,
        }

        let my_struct = SomeStruct {
            regular_field: 0,
            special_field: Cell::new(1),
        };

        let new_value = 100;
        my_struct.special_field.set(new_value);
        assert_eq!(my_struct.special_field.get(), new_value);
    }

    #[test]
    fn test_unsafe_with_threads() {
        unsafe impl<T> Sync for Cell<T> {}

        let x = Arc::new(Cell::new(0));
        let x1 = Arc::clone(&x);
        let jh1 = thread::spawn(move || {
            for _ in 0..100000 {
                let val = x1.get();
                x1.set(val + 1);
            }
        });

        let x2 = Arc::clone(&x);
        let jh2 = thread::spawn(move || {
            for _ in 0..100000 {
                let val = x2.get();
                x2.set(val + 1);
            }
        });
        jh1.join().unwrap();
        jh2.join().unwrap();
        assert!(x.get() < 200000)
    }
}
