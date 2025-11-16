use std::cell::UnsafeCell;
use crate::cell::Cell;

#[derive(Debug, Copy, Clone)]
pub enum RefState {
    Shared(usize),
    Exclusive,
    Unshared,
}

/// A mutable memory location with dynamically checked borrow rules.
/// RefCell<T> uses Rust’s lifetimes to implement “dynamic borrowing”,
/// a process whereby one can claim temporary, exclusive, mutable access to the inner value.
/// Borrows for RefCell<T>s are tracked at runtime, unlike Rust’s native reference types
/// which are entirely tracked statically, at compile time.
pub struct RefCell<T> {
    value: UnsafeCell<T>,
    state: Cell<RefState>,
}

impl<T> RefCell<T> {
    pub fn new(value: T) -> RefCell<T> {
        Self {
            value: UnsafeCell::new(value),
            state: Cell::new(RefState::Unshared),
        }
    }

    pub fn borrow(&self) -> Option<Ref<'_, T>> {
        match self.state.get() {
            RefState::Exclusive => {
                None
            }
            RefState::Shared(ref_count) => {
                // SAFETY: No exclusive reference given before.
                self.state.set(RefState::Shared(ref_count + 1));
                Some(Ref { cell: &self })
            }
            RefState::Unshared => {
                // SAFETY: No reference given before.
                self.state.set(RefState::Shared(1));
                Some(Ref { cell: &self })
            }

        }
    }

    pub fn borrow_mut(&self) -> Option<RefMut<'_, T>> {
        match self.state.get() {
            RefState::Exclusive | RefState::Shared(_) => None,
            RefState::Unshared => {
                // SAFETY: No other references given as state unshared
                self.state.set(RefState::Exclusive);
                Some(RefMut { cell: &self})
            }
        }
    }
}

struct Ref<'refcell, T> {
    cell: &'refcell RefCell<T>,
}

impl<'a, T> Drop for Ref<'a, T> {
    fn drop(&mut self) {
        match self.cell.state.get() {
            RefState::Exclusive | RefState::Unshared => unreachable!(),
            RefState::Shared(ref_count) if ref_count == 1 => {
                self.cell.state.set(RefState::Unshared);
            }
            RefState::Shared(ref_count) => {
                self.cell.state.set(RefState::Shared(ref_count - 1));
            }
        }
    }
}

struct RefMut<'refcell, T> {
    cell: &'refcell RefCell<T>,
}

impl<'a, T> Drop for RefMut<'a, T> {
    fn drop(&mut self) {
        self.cell.state.set(RefState::Unshared);
    }
}

impl<T> std::ops::Deref for Ref<'_, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.cell.value.get() }
    }
}

impl<T> std::ops::Deref for RefMut<'_, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.cell.value.get() }
    }
}

impl<T> std::ops::DerefMut for RefMut<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.cell.value.get() }
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_refcell_multiple_borrow() {
        let c = RefCell::new(5);
        let b1 = c.borrow().unwrap();
        assert_eq!(*b1, 5);
        let b2 = c.borrow().unwrap();
        assert_eq!(*b2, 5);
    }

    #[test]
    fn test_refcell_borrow_mut() {
        let c = RefCell::new(5);
        let b1 = c.borrow().unwrap();
        assert_eq!(*b1, 5);
        assert!(c.borrow_mut().is_none());
        drop(b1);
        let mut b_mut = c.borrow_mut().unwrap();
        assert_eq!(*b_mut, 5);
        assert!(c.borrow_mut().is_none());
        *b_mut = 2;
        drop(b_mut);
        assert_eq!(*c.borrow().unwrap(), 2);
    }
}

