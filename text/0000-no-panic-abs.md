- Feature Name: no_panic_abs
- Start Date: 2016-07-14
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Add the non-panicking `checked_abs`, `overflowing_abs` and `wrapping_abs` 
functions to all signed integer types.

# Motivation
[motivation]: #motivation

Currently, calling `abs()` on one of the signed integer types might panic (in 
debug mode at least) because the absolute value of the largest negative value 
can not be represented in that signed type. Unlike all other integer 
operations, there is currently not a non-panicking version on this function.

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
pub fn checked_abs(self) -> Option<Self> {
	if self.is_negative() {
		self.checked_neg()
	} else {
		Some(self)
	}
}

pub fn overflowing_abs(self) -> (Self,bool) {
	if self.is_negative() {
		self.overflowing_neg()
	} else {
		(self,false)
	}
}

pub fn wrapping_abs(self) -> Self {
	if self.is_negative() {
		self.wrapping_neg()
	} else {
		self
	}
}
```

# Drawbacks
[drawbacks]: #drawbacks

Can't think of any.

# Alternatives
[alternatives]: #alternatives

* The absolute value of the largest negative value this number could be 
represented correctly when converting to an unsigned type of the same size.
One could make a function that returns the unsigned type directly. This RFC 
does not propose that because the author could not think of a good name for 
that function. In retrospect, this is probably what the `abs()` function should 
have done. With the proposed new set of functions, the user can then cast the 
return value of `wrapping_abs` to the appropriate unsigned type and use that 
absolute value.

* Do nothing, requiring people to implement the functionality manually when 
needed.

# Unresolved questions
[unresolved]: #unresolved-questions

Should there be a similar function on the `Wrapping` wrapper as well?
