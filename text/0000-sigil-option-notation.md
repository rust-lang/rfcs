- Feature Name: `sigil_option_notation`
- Start Date: 2020-04-29
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

The ability to write `Option<Type>` as `?Type`. The implementation could be either as a `lang-item` syntax sugar or by subsuming `Option` into the language entirely.

# Motivation
[motivation]: #motivation

`Option<>` is used very often, both in user code and in the standard library. It's also a common point of refactoring, as the user reasons about the nullability of the value, and whenever the `Option` needs to be added or removed the angle brackets need to be either fished out or inserted in the correct position, which can be annoying for deeply compounded types.

Rust has traditionally opted to have terse syntax (`fn` instead of `func`, `use` instead of `using`, `Rc<_>` instead of `RefCount<_>` etc), so that there is more space for expressive user-space naming. `Option<>` is used in virtually every Rust program, to the point of being included in `std::prelude`, so it could be argued that it deserves a shorter notation.


There is also the precedent of adopting syntax from other modern languages.

In Swift, Optionals are built-in to the language and are expressed as `Type?`. This RFC proposes the `?Type` syntax, instead, for the following reasons:

-   Semantic consistency with the optional trait notation (`?Trait`), which means "the thing after the `?` might or might not be there".
-   Composability with reference and pointer sigils (`&`, `&mut`, `*` etc). 
    -   If we were to use `Type?`, the type `&Type?` could be interpreted as either `Option<&Type>` or `&Option<Type>`, which would require the user to remember the precedence.
    -   With `?Type`, there is no such ambiguity.
        -   `&?Type` = `&Option<Type>`
        -   `?&Type` = `Option<&Type>`
    - Also, having to find the end of a type inside of a deep compound type to insert the `?` would somewhat defeat the purpose.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

You might use `Option<Type>` very frequently. To save space, you may instead choose to use the notation `?T`.

Here are some examples:

|Generic notation|Sigil notation|
|---|---|
|`Option<String>`|`?String`|
|`Option<&str>`|`?&str`|
|`&Option<&str>`|`&?&str`|
|`(usize,Option<Rc<RefCell<u8>>>)`|`(usize, ?Rc<RefCell<u8>>)`|
|`Option<&mut i32>`|`?&mut i32`|
|`&mut Option<String>`|`&mut ?String`|
|`Option<Foo<T>>`|`?Foo<T>`|

It can also be used in function signatures, and wherever else `Option<_>` can be used.

```rust

fn some_function<Vec<?Foo>>(req_arg: String, opt_arg: ?i32) -> ?String {
    ...
}
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The feature should be clear. It would involve adding `?` to the type grammar in a similar rule as `&` and `*`.

It should not conflict with the optional trait syntax (`?Trait`), since it would resolve into a type and not into a bound.

# Drawbacks
[drawbacks]: #drawbacks

-   As `Option` is so commonly used, it would somewhat change the "face" of Rust.
-   If this becomes the preferred notation, all documentation of interfaces that use `Option` would have to change.
-   It could be argued that having both `?Type` and `?Trait` would be confusing.
-   Implementing it as syntax sugar would add to the issues with `lang-item` special cases.
-   Baking it into the language would make `Option<T>` a special case, as it would have to be supported for backwards compatibility.
-   In either case, it would make `Option` magic.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

This change would be mostly for aesthetic and convenience reasons, and so could be simply not added.

# Prior art
[prior-art]: #prior-art

As mentioned, this feature is very similar to a core part of the Swift syntax. It would also add a nice symmetry to Swift's `expr!` syntax (which is similar to Rust's `expr?` syntax).

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- The implementation strategy must be decided (`lang-item` or built-in).
- The official documentation notation must be chosen.

# Future possibilities
[future-possibilities]: #future-possibilities

This feature might spark interest into a generalization of the `?` notation to allow for custom "nullable" types. It must be decided if this interest is desirable.