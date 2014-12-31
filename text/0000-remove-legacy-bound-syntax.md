- Start Date: 2014-12-30
- RFC PR:
- Rust Issue:

# Summary

Rust has two ways of specifying bounds on generic types. In the legacy syntax,
the bounds are specified inline within the `<` `>` parameter lists:
```rust
struct Foo<'a, 'b: 'a, T: Clone> {
    t: &'a &'b T
}
```
In the new syntax, the bounds are specified after the parameter list in a where
clause:
```rust
struct Foo<'a, 'b, T> where 'b: 'a, T: Clone {
    t: &'a &'b T
}
```

This RFC proposes the removal of the legacy bound syntax.

# Motivation

The where clause syntax is more powerful than the legacy syntax and arguably
more readable, especially in more complex cases. Having two language features
that do the same thing is something to be avoided. It imposes a greater
maintenance burden on the language, and has the potential to confuse
programmers when they come across the rarely used legacy syntax.

# Detailed design

The parser will no longer accept bounds in `<` `>` parameter lists.

Where clauses do not currently work with tuple structs. The clause is expected
just after the parameter list:
```rust
struct Foo<T> where T: Clone (T);
```
This looks strange, and also unfortunately causes conflicts with the sugary
notation for the `Fn` traits:
```
test.rs:1:24: 1:33 error: parenthetical notation is only stable when used with the `Fn` family of traits
test.rs:1 struct Foo<T> where T: Clone (T);
                                 ^~~~~~~~~
test.rs:1:24: 1:33 help: add `#![feature(unboxed_closures)]` to the crate attributes to enable
test.rs:1 struct Foo<T> where T: Clone (T);
                                 ^~~~~~~~~
test.rs:1:24: 1:33 error: wrong number of type arguments: expected 0, found 2
test.rs:1 struct Foo<T> where T: Clone (T);
                                 ^~~~~~~~~
```

The where clause should instead be parsed after the tuple:
```rust
struct Foo<T>(T) where T: Clone;
```

# Drawbacks

It'll cause a lot of churn.

The legacy syntax is a bit more concise in the most simple cases:
```rust
impl<T> Foo for Bar<T> where T: Foo {}
// vs
impl<T: Foo> for Bar<T> {}
```

# Alternatives

Not doing this leaves an obscure non-orthogonal feature in the language past
1.0, where we have to maintain it indefinitely.

# Unresolved questions

None
