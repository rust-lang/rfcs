- Start Date: 2014-01-23
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Add an unsafe enum which is an enum without a discriminant.

# Motivation

When working with FFI, many C libraries will take advantage of unions. Unfortunately Rust has no way
to represent this sanely, so developers are forced to write duplicated struct definitions for each
union variant and do a lot of ugly transmuting. Unsafe enums are effectively unions, and allow FFI
that uses unions to be significantly easier. The syntax chosen here replicates existing syntax without adding new keywords and is thus backwards compatible with existing code.

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

Due to the lack of a discriminant, destructuring becomes irrefutable, but also unsafe. Therefore all destructuring needs to be in unsafe functions or blocks. Because destructuring is irrefutable you can directly destructure unsafe enums which isn't possible with safe enums:
```rust
unsafe { let Variant1(bar) = foo; }
```
When using `match` you can only have a single irrefutable pattern. However, patterns with values inside of them or conditionals are refutable and can therefore be combined.
```rust
// Valid
unsafe match foo {
    Variant2(x) => ...,
}
unsafe match foo {
    Variant1(5) => ...,
    Variant1(x) if x < -7 => ...,
    Variant2(x) => ...,
}
// Invalid
unsafe match foo {
    Variant1(x) => ...,
    Variant2(x) => ...,
}
unsafe match foo {
    Variant1(x) => ...,
    _ => ...,
}
```
`if let` and `while let` are irrefutable unless the pattern has a value inside. Because irrefutable `if let` and `while let` patterns are currently illegal for enums in Rust, they will continue to be illegal for unsafe enums.
```rust
// Illegal
if let Variant1(x) = foo {}
while let Variant2(y) = foo {}
// Legal
if let Variant1(5) = foo {}
while let Variant1(7) = foo {}
```

## Requirements on variants

Due to the lack of a discriminant there is no way for Rust to know which variant is currently
initialized, and thus all variants of an unsafe enum are required to be `Copy` or at the very least
not `Drop`.

# Drawbacks

Adding unsafe enums adds more complexity to the language through a separate kind of enum with its own restrictions and behavior.

# Alternatives

* Continue to not provide untagged unions and make life difficult for people doing FFI.
* Add an entirely separate type with a keyword such as `union`.

# Unresolved questions

* For `if let`, when the pattern is irrefutable should the else block be legal?
