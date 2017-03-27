- Feature Name: immovable_types
- Start Date: 2017-01-09
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

This add an new built-in trait `Move` which all existing types will implement. Types which do not implement it cannot move after they have been borrowed.

# Motivation
[motivation]: #motivation

Interacting with C/C++ code may require data that cannot change its location in memory. To work around this we allocate such data on the heap. For example the standard library `Mutex` type allocates a platform specific mutex on the heap. This prevents the use of `Mutex` in global variables. If we add immovable types, we can have an alternative immovable mutex type `StaticMutex` which we could store in global variables. If the lifetime of the mutex is limited to a lexical scope, we could also have a `StaticMutex` in the stack frame and avoid the allocation.

The key motivation for this proposal is to allow generators to have "stack frames" which do not move in memory. The ability to take references to local variables rely on those variable being static in memory. If a generator is moved, the local variables contained inside also move, which invalidates references to them. So references to local variables stored inside the generator cannot be allowed.

Since generators can only move during suspend points we can require that references to local variables do not live across suspend points and so they would not get invalidated. This is still quite restrictive compared to normal functions and will result in code containing unnecessary allocations. If generators are immovable, no such restrictions apply, and references would work like in normal functions. It does however place a burden on the user of those generators to not move them. This isn't a problem for use cases such as awaiting on a future in asynchronous code or iterating over a structure, since the generator would be stored in the stack frame (which is immovable).

# Detailed design
[design]: #detailed-design

A new unsafe auto trait `Move` is introduced in `core::marker`. Auto traits are implemented for all primitive types and for composite types where the elements also implement the trait. Users can opt-out to this for custom types. References, pointers, `core::ptr::Unique` and `core::ptr::Shared` explicitly implement this trait, since pointers are movable even if they point to immovable types.

All type parameters (including `Self` for traits), trait objects and associated types have a `Move` bound by default.

If you want to allow types which may not implement `Move`, you would use the `?Move` trait bound which means that the type may or may not implement `Move`.

You can freely move values which are known to implement `Move` after they are borrowed, however you cannot move types which aren't known to implement `Move` after they have been borrowed. Once we borrow an immovable type, we'd know its address and code should be able to rely on the address not changing. This is sound since the only way to observe the address of a value is to borrow it. Before the first borrow nothing can observe the address and the value can be moved around.

Static variables allow types which do not implement `Move`.

## Borrowing immovable types

Borrowing values of types which do not implement `Move` is only allowed if the borrows lasts for the entire lifetime of the values, including the drop of the value, since `drop` takes a reference to it. Reborrows of such borrows follow existing rules.

This means that the following borrow would not be allowed:
```rust
let mutex = StaticMutex::new(0);
{
	*mutex.lock() += 1;
}
let moved = mutex;
*moved.lock() += 1;
```
Here `lock` borrows the `mutex` variable. The borrow only last until the end of the same statement. That means we'd be allowed to move `mutex` into a new variable `moved` and call `lock` on it, this time with an different address for `&self`!

We rely on the fact that borrows prevent moves. We cannot change the lifetime of the borrow to encompass the moving statement using the current borrow checker. This can be changed once we get non-lexical lifetime and we'd get an error on the move instead.

Conceptually we can think of borrows of `?Move` values as introducing 2 borrows:
- one borrow with as short lifetime as possible with normal restrictions
- one borrow which must match the lifetime of the borrowed value. The only restriction placed is that the value must not be moved out of.

This RFC suggests an approach where we use only the shorter borrow and require it to match the lifetime of the value. This is less flexible, but results in minimal implementation changes. A more flexible solution can be introduced with non-lexical lifetimes.

We can easily work around issues with this in code by using a single borrow of the immovable value and just reborrow.

Illegal:
```rust
let mutex = StaticMutex::new(0);
*mutex.lock() += 1;
*mutex.lock() += 1;
```
Workaround using reborrows:
```rust
let mutex = &StaticMutex::new(0);
*mutex.lock() += 1;
*mutex.lock() += 1;
```

A borrow such as `&var.field` where `field` is immovable will last as long as the lifetime of `var` to ensure it matches the lifetime of the field.

We need to prevent assignment to immovable types once they have been borrowed. This is because assignment actually moves the l-value before calling `Drop::drop`. If there are any restrictions on the l-value or if the l-value has a dereference operation, assignment to immovable types is not allowed.

Types which implement `Copy`, but not `Move` are allowed. You can still copy them around, but borrows follows the restrictions of `?Move` types.

## Immovable types contained in movable types

To allow immovable types to be contained in movable types, we introduce a `core::cell::MobileCell` wrapper which itself implements `Move`. It works similarly to `Cell` in that it disallows references to the value inside.
```rust
#[lang = "mobile_cell"]
pub struct MobileCell<T: ?Move> {
	value: T,
}

unsafe impl<T: ?Move> Move for MobileCell<T> {}

impl<T: ?Move> MobileCell<T> {
	pub const fn new(value: T) -> Movable<T> {
		Movable {
			value: value,
		}
	}

	pub fn into_inner(self) -> T {
		self.value
	}

	pub fn replace(&mut self, new_value: T) -> T {
		let mut result = MobileCell::new(new_value);
		core::mem::replace(self, &mut result);
		result.into_inner()
	}
}
```

## Implications for language traits

In order to allow functions to take immovable types and arguments and return them, we need to change `FnOnce`, `FnMut` and `Fn`. A `?Move` bound should be added for the `Args` type parameter to these traits. We also need a `?Move` bound on `FnOnce::Output`, which is backwards incompatible. `FnOnce::Output` was stabilized in 1.12, so hopefully there aren't any code relying on it yet.

Having a `?Move` bound on `Deref::Target` would be nice. It would allow us to use the dereference operator on `Box`, `Rc`, and `Arc` containing immovable types.

A `?Move` bound on `IntoIterator::IntoIter` and `Iterator::Self` would also be useful, since you could then use immovable iterators in for-loops.

I suggest we do a `crater` run to investigate if these breakages are feasible.

Changing these associated types will be insta-stable. You would be unable to write stable code which would conflict with this proposal. `?Move` bounds would also show up in documentation, although we would be able to filter those out if desired.

## Allowing immovable types in container types

`std::boxed::Box`, `std::rc::Rc`, `std::rc::Weak`, `std::sync::Arc`, `std::sync::Weak` will be changed to allow immovable types inside, but will themselves be movable. These can be used to overcome the limitations of immovable types at the cost of an allocation.

For `Rc` and `Arc` , the function `try_unwrap` would only be allowed on movable types.

In general, we can allow immovable types in an movable container if we either:
- disallow all methods of accessing the address of the contained immovable types, including references (possible for `Vec`, `HashMap`)
- prevent the type from actually moving once it's inside (the method suitable for `Box`, `Rc`, `Arc`)


# How We Teach This
[how-we-teach-this]: #how-we-teach-this

Rust already has the concept of immovable values when a value is borrowed. This adds types where borrows always last until the value is dropped.

The concept of immovable types is likely familiar to users of C, C++ and C FFIs.

# Drawbacks
[drawbacks]: #drawbacks

This adds a new builtin trait and more logic to the borrow checker. It also requires `?Move` bounds. It may also break existing programs.

# Alternatives
[alternatives]: #alternatives

- Instead of having a `Move` trait, we can add final reference types `&final` `&final mut`. Borrowing with these would correspond to borrows of `?Move` types in this RFC. This would require much move invasive changes to the language and may rule out the possiblity of self borrowing types with a `'self` lifetime. 

- Do nothing, but not having this makes generators interact rather poorly with references.

# Unresolved questions
[unresolved]: #unresolved-questions

Which associated types can we change in a backwards incompatible way?