- Feature Name: unsigned-abs
- Start Date: 2015-03-20
- RFC PR:
- Rust Issue:

# Summary

Change the function `abs()` to return unsigned integers, instead of signed integers as it does today.

# Motivation

The documentation for `std::num::SignedIntStable::abs()` states, `Int::min_value()` will be returned
if the number is `Int::min_value()`. This subtlety can lead to bugs, because a developer may count
on `abs()` to return a positive value. And forcing every caller of `abs()` to know about this corner
case and handle it, is not a very nice design.

If `abs()` is changed to return an unsigned integer of the same width as its input, there is no
special case to handle. Code calling `abs()` is cleaner because it does not have to handle
`Int::min_value()`, and will lead to fewer bugs.

Because (for example) `-128i8` happens to have the same binary representation as `128u8`, this is
basically a free operation.

Consider how code that calls the current version of `abs()` can ensure its result is positive:

1. Proof by reasoning about your code that the input of `abs()` will never be `Int::min_value()`.
2. Do a runtime check before calling `abs()`, and somehow handle it or return an error.
3. Cast the input to a wider signed integer (if available) before calling `abs()`.
4. Cast the result of `abs()` to an unsigned integer of the same size.

In all cases this requires extra work for the code calling abs. With this proposal option 4 will be
the default, while option 1 to 3 remain possible. The default behaviour will lead to correct
(positive) results for all inputs.

# Drawbacks

It is not possible anymore to do:
```rust
let mut a: i8 = -15;
a = abs(a);
```

This becomes:
```rust
let mut a: i8 = -15;
a = abs(a) as i8; // may overflow
```
This may actually be an advantage, as now the user code is the cause of the overflow, and not `abs()`.

# Alternatives

Keep the status quo, which can lead to subtle bugs or code that is less clean.

# Unresolved questions

None.
