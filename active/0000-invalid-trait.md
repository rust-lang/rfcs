- Start Date: 2014-04-09
- RFC PR #: (leave this empty)
- Rust Issue #: (leave this empty)

# Summary

Add `Invalid` trait and use it for `Option<T>` space optimization, instead of hardcoding "special" types in the compiler.

# Motivation

Generally, Rust enum types contain a discriminator field in order to distinguish which of the variants is present. However, in some specific cases it is elided by the compiler.  One such example is `Option<*T>` type, which does not store discriminator because compiler *knows* that "pointers cannot be null", so it uses "all-zeroes" bit pattern to encode the `None` variant.

The current approach is not extensible.  There may be cases when the programmer knows that certain bit patterns will never be used, but there is no way to convey this information to the compiler.

This RFC proposes to introduce a new built-in trait, `Invalid`, which can be used to test whether a given bit pattern constitutes a valid value, and have Rust compiler rely on this trait instead of hard-coded logic.

A secondary motivation for this proposal is to allow data validation when writing e.g. serialization libraries.

# Detailed design

```rust
trait Invalid {
    // sets memory to the 'invalid' bit pattern
	unsafe fn set_invalid(p: *u8);
	// tests whether memory contains an 'invalid' bit pattern
	unsafe fn is_invalid(p: *u8) -> bool;
}
```
Naturally, the implementation must ensure that `set_invalid()` and `is_invalid()` are coherent.

Note: I've considered having `set_invalid`'s return valud type to be `Self`, however in this case we'd be actually creating 'invalid' values, which is a counter-intuitive, and potentially dangerous.

### Compiler changes

Rustc should perform the same space optimization it currently does for `Option<*T>` (or `~T`, or `&T`), but now for all types for which `Invalid` is implemented.
When creating the nullary enum variant it should use the `set_invalid()` method.  When checking which variant is present, the `is_invlaid()` method should be used.

### libstd implementations:
```rust
impl<T> Invalid for *T {
	unsafe fn set_invalid(p: *u8) {
		let x: &mut *T = transmute(p);
		*x = ptr::null();
	}
	unsafe fn is_invalid(p: *u8) -> bool {
		let x: &*T = transmute(p);
		*x as uint == 0
	}
}
impl<T> Invalid for ~T { /* same */ }
impl<T> Invalid for &T { /* same */ }
```

# Alternatives
The primary alternative to this proposal would be specifying invalid bit patterns via attributes (e.g. https://github.com/rust-lang/rfcs/pull/36).  I think this approach would be hard to extend to arbitrary types (currently it only works with pointers).

# Unresolved questions
* Is there a better alternative to writing directly into memory?  This might prevent LLVM from placing enums into a register.  Perhaps it would be better to return something like `[u8, ..sizeof(Self)]`? (well, except that Rust doesn't have sizeof() operator).
* Currently there is no way to invoke static methods of a trait in Rust (https://github.com/mozilla/rust/issues/8888).  It is anticipated that in v1.0 such syntax will exist.  For compiler-internal use this should not be a problem, though.
* Should libstd implement `Invalid` for f32 and f64 (using one of the NaN bit patterns for "invalid value")?  The downside is that some people might expect all of float values to round-trip via `Option<>`, even the NaNs.
* Should libstd implement it for `bool` using, say, `-1 as bool` for "invalid"?
