- Feature Name: wrapping_abs
- Start Date: 2016-07-14
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Add a `wrapping_abs` function to all signed integer types.

# Motivation
[motivation]: #motivation

Currently, calling `abs()` on one of the signed integer types might panic (in 
debug mode at least) because the absolute value of the largest negative value 
can not be represented in that signed type. However, this number could be 
represented correctly when converting to an unsigned type of the same size.

# Detailed design
[design]: #detailed-design

This is the current implementation of `abs()`:

```rust
pub fn abs(self) -> Self {
	if self.is_negative() {
		-self
	} else {
		self
	}
}
```

This RFC proposes to add the following:

```rust
pub fn wrapping_abs(self) -> Self {
	if self.is_negative() {
		self.wrapping_neg()
	} else {
		self
	}
}
```

The user can then cast the return value to the appropriate unsigned type and 
use that absolute value.

# Drawbacks
[drawbacks]: #drawbacks

Can't think of any.

# Alternatives
[alternatives]: #alternatives

* One could make a function that returns the unsigned type directly. This RFC 
does not propose that because the author could not think of a good name for 
that function. In retrospect, this is probably what the `abs()` function should 
have done.
* Do nothing, requiring people to implement the functionality manually when 
needed.

# Unresolved questions
[unresolved]: #unresolved-questions

Should there be a similar function on the `Wrapping` wrapper as well?
