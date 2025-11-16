mod cell;
mod rc;
mod refcell;

/*
# Rc
## Multiple Ownership:
Allows multiple parts of the program to own the same value using reference counting.

## Immutable Only:
Does not allow mutation of the inner value unless combined with interior mutability types (e.g., Rc<RefCell<T>>).

# Cell

## Copy-by-Value Interior Mutability:
Allows mutation by replacing or copying the entire value (get, set), but only works well for Copy or small-by-value types.

## No References Given Out:
Cannot borrow &T or &mut T from a Cell<T>—you only read/write the whole value.

# RefCell

## Interior Mutability via Runtime Borrow Checking:
Allows mutable or immutable borrows even through &T, with borrow rules enforced at runtime.

## Works for Complex / Non-Copy Types:
Supports borrowing references into the inner value (e.g., RefMut), enabling mutation of complex data structures like Vec<T>, HashMap, etc.

*/
