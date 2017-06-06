- Start Date: 2014-6-1
- RFC PR #: (leave this empty)
- Rust Issue #: (leave this empty)

# Summary

A new form of reference, `&uninit`, is added that is write-only and points to possibly uninitialized
data.

# Motivation

Many functions that would be perfectly safe to write in other languages, like `mem::swap`, `vec::swap`,
or `mem::replace` are unsafe when written in Rust. This lack of ability to safely move things around
efficiently means that one often has to resort to using unsafe code, as shown in `PriorityQueue`'s `sift`
methods.

While `PriorityQueue` does this for performance reasons, there are certain idioms that are not in general
possible without unsafe code. For example, a `fn(T) -> T` cannot be used to mutate a value behind a `&mut`
pointer, as we cannot even temporarily move out of the pointer.

# Detailed design

A new reference type, `&uninit` is added, with the following properties:

- It cannot alias with other pointers in the same sense as `&mut`.
- It cannot be read from.
- It must be written to at least once in its lifetime. This makes it a linear type, unlike most of Rust's
  types, which are affine.
- Once written to, it implicitly becomes an `&mut` pointer.
- When encountered while unwinding, it zeroes the value it points to.

There are two ways to create an `&uninit` pointer:

- Moving out of an `&mut` pointer. The pointer implicitly becomes an `&uninit` pointer.
- Borrowing an uninitialized variable. This is equivalent to writing a dummy value, taking an `&mut` borrow,
  moving out of the `&mut`, and dropping the dummy value.

Note that one subtlety in the mechanism of switching between `&mut` and `&uninit` is that the state which the
pointer is in must be defined at all times. This disallows, for example, the following:

```rust
if condition {
	drop(*ptr);
} // ERROR: ptr is left possibly uninitialized and possibly initialized
```

This solves all of the cases mentioned in the motivation section. For example:

```rust
pub fn swap<T>(x: &mut T, y: &mut T) {
	let temp = *x; // x is now an &uninit pointer.
	*x = *y; // x was written to, so is now an &mut pointer, but y was moved from, and so is an &uninit pointer.
	*y = temp; // temp is moved out of, and y returns to being an &mut pointer.
} // There are no remaining &uninit pointers in scope, so we are allowed to exit.

pub fn apply_trans(p: &mut Box<uint>) {
	fn trans(in: Box<uint>) -> Box<uint> {
		let val = *in + 1;
		box val
	}
	let temp = *p; // p is now an &uninit pointer.
	*p = trans(temp); // p is written to, so becomes an &mut pointer.
} // There are no remaining &uninit pointers in scope, so we are allowed to exit.
```

Additionally, it truly is safe, even in the face of failures:

```rust
fn deallocate_dead() {
	let vec = vec!(box 0, box 1, box 2, box 3);
	let ptr = vec.get_mut(2);
	drop(*ptr);

	// This drops the whole vector, but the pointer to 2 is not freed twice because it was zeroed when
	// ptr was encountered.
	fail!();

	*ptr = box 2; // Fill in the whole
}

struct List {
	next: Option<Box<List>>
}

// We cannot use these holes to create cycles in ownership that would never be cleaned up
fn ownership_cycle() {
	let mut list = box List { next: None }
	let ptr = &mut list.next;
	drop(*ptr);
	*ptr = list; // ERROR: list is borrowed
}

fn read_uninitialized() -> Box<uint> {
	let mut val;
	let ptr = &uninit val;
	let ret = val // ERROR: val is borrowed

	*ptr = box 1; // Fill the pointer

	ret
}

fn uninitialized_mut() {
	fn kill(ptr: &mut Box<uint>) {
		drop(*ptr);
	} // ERROR: ptr, an &uninit pointer, is left unfilled

	let mut val = box 1;
	kill(&mut val); // This deallocates the box, making the next line crash
	println!("{}", *val);
}
```

# Drawbacks

- This adds a dependency on the drop flag mechanism, which can bloat data structures and cause inefficiencies
  and was possibly slated for removal. See mozilla/rust#5016 for more details.
- This reintroduces the idea of type state, which can sometimes be difficult to reason about.
- This adds yet another pointer type.

# Alternatives

- Don't do this at all. This obviously is workable, but it leaves multiple places with unnecessary unsafe code.
  Additionally, it seems odd for common idioms like swap in other languages to not only not work directly but be
  impossible to write safely.
- Make the drop flag explicit, meaning that every type has a "null" value that won't be cleaned up. Then moves out
  of `&mut` are simply replacing with this "null". This seems like a very bad idea - there is a reason that Rust opted to
  use `Option` instead of null pointers.
- Try to add more functions to the standard library that can handle most conceivable movement patterns. For example, the
  `PriorityQueue` code could be replaced with a function that takes an iterator of `&mut` pointers and rotates the values
  contained within. While this certainly is useful, it is unclear that this idea will scale. While rotation is a useful
  transformation, other permutations could be useful in other situations, and it would be very difficult to deal with all of
  them without making the standard library huge. 

# Unresolved questions

- Is there a way to preserve failure safety without requiring the drop flag?
- Given `&uninit (A, B)`, it should be possible to get `(&uninit A, &uninit B)`. How? What is the syntax?
- What should the relevant error messages be?
