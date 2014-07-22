- Start Date: 2014-07-22
- RFC PR #: (leave this empty)
- Rust Issue #: (leave this empty)

# Summary

This adds the ability to have macros in type signatures.

# Motivation

## Use procedural macros to generate values at the type level.
For natural number we could define something like this:
```rust
struct Zero;
struct Succ<T>;
```
Now we can make numbers like 0 (`Zero`),  1 (`Succ<Zero>`), 2 (`Succ<Succ<Zero>>`), etc. on the type level.
However these are inconvenient to write out, so we can make an procedural macro which generates these for us.
`PackInt!(3)` could expand to `Succ<Succ<Succ<Zero>>>`. To turn this back into a value, we can use a trait.
```rust
trait IntVal {
    fn get(&self) -> uint;
}

impl IntVal for Zero {
    fn get(&self) -> uint {
        0
    }
}

impl<T: IntVal> IntVal for Succ<T> {
    fn get(&self) -> uint {
        unsafe {
            1 + unpack_int<T>()
        }
    }
}

fn unpack_int<T: IntVal>() -> uint {
    unsafe {
        mem::uninitialized<T>().get()
    }
}
```

An more advanced example would be `PackStr!("hello")` which could use cons cells to encode a string literal as a type.

## Completeness
It's currently surprising that macros won't work in type signatures.

# Detailed design

Anywhere a type is allowed, an type macro is also allowed using the existing macro infrastructure.

# Drawbacks

None.

# Alternatives

None.

# Unresolved questions

None.