- Start Date: 2014-01-23
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Add an unsafe enum which is an enum without a discriminant.

# Motivation

When working with FFI, many C libraries will take advantage of unions. Unfortunately Rust has no way
to represent this sanely, so developers are forced to write duplicated struct definitions for each
union variant and do a lot of ugly transmuting. Unsafe enums are effectively unions, and allow FFI
that uses unions to be significantly easier.

# Detailed design

An unsafe enum is equivalent to a safe enum except that it does not have a discriminant.

## Declaring an unsafe enum

```rust
unsafe enum MyEnum {
    Variant1(c_int),
    Variant2(*mut c_char),
    Variant3 {
        x: f32,
        y: f32,
    },
}
```

## Instantiating unsafe enums

```rust
let foo = Variant1(5);
```

## Destructuring

Due to the lack of a discriminant, `match` cannot be used to destructure an unsafe enum.
Additionally `if let` and `while let` do not make sense to support because there is no discriminant
to test. Therefore there is only one way to destructure an unsafe enum, and it is unsafe.
```rust
unsafe { let Variant1(bar) = foo; }
```

## Requirements on variants

Due to the lack of a discriminant there is no way for Rust to know which variant is currently
initialized, and thus all variants of an unsafe enum are required to be `Copy` or at the very least
not `Drop`.

# Drawbacks

Adding unsafe enums adds more complexity to the language through a separate kind of enum which is
unusable in many of the ways that normal enums are.

# Alternatives

* Continue to not provide untagged unions and make life difficult for people doing FFI.
* Add a keyword such as `union`.

# Unresolved questions

* ???
