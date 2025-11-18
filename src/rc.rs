use crate::cell::Cell;
use std::marker::PhantomData;
use std::ptr::NonNull;

/// Single-threaded reference-counting pointers. ‘Rc’ stands for ‘Reference Counted’.
/// The type Rc<T> provides shared ownership of a value of type T, allocated in the heap.
/// Invoking clone on Rc produces a new pointer to the same allocation in the heap.
/// When the last Rc pointer to a given allocation is destroyed, the value stored
/// in that allocation (often referred to as “inner value”) is also dropped.
pub struct Rc<T> {
    inner: NonNull<RcInner<T>>,
    _marker: PhantomData<RcInner<T>>,
}

pub struct RcInner<T> {
    value: T,
    owner_count: Cell<usize>,
}

impl<T> Clone for Rc<T> {
    fn clone(&self) -> Self {
        let inner = unsafe { self.inner.as_ref() };
        inner.owner_count.set(inner.owner_count.get() + 1);
        Rc {
            inner: self.inner,
            _marker: PhantomData,
        }
    }
}

impl<T> std::ops::Deref for Rc<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &unsafe { self.inner.as_ref() }.value
    }
}

impl<T> Rc<T> {
    pub fn new(value: T) -> Self {
        let inner = Box::new(RcInner {
            value,
            owner_count: Cell::new(1),
        });

        Self {
            inner: unsafe { NonNull::new_unchecked(Box::into_raw(inner)) },
            _marker: PhantomData,
        }
    }
}

impl<T> Drop for Rc<T> {
    fn drop(&mut self) {
        let inner = unsafe { self.inner.as_ref() };

        let c = inner.owner_count.get() - 1;
        inner.owner_count.set(c);

        if c == 0 {
            let _ = inner;
            // take ownership and drop RcInner + T
            drop(unsafe { Box::from_raw(self.inner.as_ptr()) });
        }
    }
}

/*
# Strong (Rc<T>)

## Owns the data:
Increases the strong reference count and keeps the value alive.

## Controls deallocation:
The value is dropped only when the strong count becomes zero.

# Weak (Weak<T>)

## Does NOT own the data:
Does not keep the value alive; increases only the weak count.

## Must be upgraded:
Access requires upgrade() → Option<Rc<T>>, which is None if all strong pointers are gone.
*/

// Why PhantomData is required?
// For the T type drop check to happen.

/*
pub struct Foo<'a, T> where T: Default {
    inner: &'a mut T,
}

impl<T: Default> Drop for Foo<'_, T> {
    fn drop(&mut self) {
        std::mem::replace(self.inner, T::default());
    }
}

fn main() {
    // This won't compile as drop happen like stack, first defined last dropped.
    let (foo, mut t);
    // Below would work
    // let (mut t, foo);
    t = String::from("hello");
    foo = Foo { inner: &mut t};
}
*/

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rc() {
        let a = Rc::new("hello".to_string());
        let b = Rc::clone(&a);
        assert_eq!(*a, "hello");
        assert_eq!(*b, "hello");
        drop(b);
        assert_eq!(*a, "hello");
        drop(a);
    }
}
