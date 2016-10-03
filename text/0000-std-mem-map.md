- Feature Name: std-mem-map
- Start Date: 2016-10-03
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

This RFC provides a function `std::mem::map()` which can be used to replace a mutable borrow to a new value by consuming the current value.

# Motivation
[motivation]: #motivation

Currently it is impossible to move out of a borrowed value. This is due to the language not having a way of enforcing that the value is returned to a safe state before the reference is returned. To solve this problem we have a couple of functions to replace the value in a safe way. Currently these functions include `std::mem::replace()` and `std::mem::swap()`. However these functions are not sufficent when generating the new value requires consuming the old value.

This RFC adds a clean and simple way of dealing with this issue.

## Example Difficult Code

```rust
fn mutate_a(s: String) -> String;
fn mutate_b(s: String) -> String;

fn tick(s: &mut String, op: bool) {
	*s = if op { mutate_a(*s) } else { mutate_b(*s) }
 	                      ^^ cannot move out of borrowed content
	                                            ^^ cannot move out of borrowed content

}
```

## Example Work Around

```rust
fn tick(s: &mut String, op: bool) {
	let old = mem::replace(s, String::new());
	mem::replace(s, if op { mutate_a(old) } else { mutate_b(old) });
}
```

However this approach has a number of downsides. It requires unintutive calls to `std::mem::replace()` which isn't very intuitive for this use case. Also it requires the construction of a "dummy" instance of the type to be changed. This can be expensive or difficult for many types.

## Example With RFC

```rust
fn tick(s: &mut String, op: bool) {
	std::map(s, |s| if op { mutate_a(s) } else { mutate_b(s) });
}
```

This example has a more intuitive function and doesn't require the construction of a dummy instance.

# Detailed design
[design]: #detailed-design

This function adds a single function `std::mem::map()` which can perform this "in-place" update of a value.

```rust
fn map<T, F>(val: &mut T, f: F)
	where F: FnOnce(T) -> T
```

# Drawbacks
[drawbacks]: #drawbacks

This does not solve the overall issue of temporarialy moving a value out of a mutable borrow.

# Alternatives
[alternatives]: #alternatives

An alternative would be to extend the ownership model to provide a way to move ownership back into a value. This would be very complicated and it is unclear if this would be benificial overall.

# Unresolved questions

None

# Example Implementation

```rust
use std::mem;

fn map<T, F>(val: &mut T, f: F)
	where F: FnOnce(T) -> T
{
	unsafe {
		let mut old = mem::replace(val, mem::uninitialized());
		mem::forget(mem::replace(val, f(old)));
	}
}
```
