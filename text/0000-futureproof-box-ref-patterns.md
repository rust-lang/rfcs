- Start Date: 2014-11-11
- RFC PR #: (leave this empty)
- Rust Issue #: (leave this empty)

# Summary

Futureproof `box` patterns by renaming them to `deref`.

In an effort to consolidate `box` and `&` patterns, change the latter to use the `deref` syntax as well, in recognition of them being semantically equivalent to `box` patterns.

Make the newly introduced `deref` keyword a non-strict keyword.

# Motivation

The main motivation for this RFC is to futureproof `box` patterns, which currently only work with the `Box` type. Post the 1.0 release it is likely that the Rust community decides to extend `box` patterns to support any type that implements the `Deref` family of traits to make pattern matching accessible to other user-defined types such as `Rc`. However, whilst there is an overlap between types that are `box` allocated and types that implement the `Deref` family of traits, conceptually they're distinct categories. Therefore, the `box` keyword does not seem to well reflect the meaning of the pattern and in recognition of this the RFC proposes to rename it to `deref`.

In addition, the RFC recognises that `box` and `&` patterns are functionally identical and proposes to rename the latter to `deref` as well.

# Detailed design

1. Rename the `box` keyword appearing in patterns to `deref`.

    The following code:
    ```rust
        match ty::get(function_type).sty {
            ty::ty_closure(box ty::ClosureTy {
                store: ty::RegionTraitStore(..),
                ..
            }) => {
                // ...
            }
            _ => {
                // ...
            }
        }
    ```

    would, after this change, read:
    ```rust
        match ty::get(function_type).sty {
            ty::ty_closure(deref ty::ClosureTy {
                store: ty::RegionTraitStore(..),
                ..
            }) => {
                // ...
            }
            _ => {
                // ...
            }
        }
    ```

    It is proposed that the `deref` keyword be made a non-strict keyword so that user code can continue to use `deref` as the name of an identifier or an item.

2. Change the `&` patterns to use the `deref` syntax as well.

    The following code:
    ```rust
    match node {
        &PatWild(_) => {
            // ...
        }
        &PatEnum(_, ref args) => {
            // ...
        }
        _ => {
            // ...
        }
    ```

    would, after this change, read:
    ```rust
    match node {
        deref PatWild(_) => {
            // ...
        }
        deref PatEnum(_, ref args) => {
            // ...
        }
        _ => {
            // ...
        }
    ```

It is important to note that extending the proposed `deref` patterns to support types implementing the `Deref` family of traits is outside of this RFC's scope and can be proposed and introduced backwards-compatibly in the future.

An important implication of this change is that it renders `ref` and `deref` patterns *symmetrical* in their functional meaning.

Specifically, a pattern of the form `deref [pat]` denotes that the pattern will successfully match a value if dereferencing said value arrives at a value that matches the pattern [pat].

Similarly, a pattern of the form `ref [pat]` denotes that the pattern will successfully match a value if taking the address of the value will produce a value that matches the pattern [pat]. In this particular case, [pat] is restricted to always be an identifier pattern and indeed a `ref` pattern binds said identifier to a reference of the destructured value.

# Drawbacks

* Renaming `&` to `deref` is undoubtedly an ergonomic regression.
* Renaming `&` to `deref` will impact a substantial number of Rust code currently in existence.

# Alternatives

1. Restrict the RFC to only propose the first change, namely renaming `box` patterns to `deref` patterns whilst leaving `&` patterns intact. While this is still an improvement and addresses the main issue in question, it leaves the language with two constructs that are semantically equivalent.

2. Change `box` patterns to use the `&` syntax instead. This has been considered before drafting this RFC, however, it is believed that allowing `&` in patterns destructuring non-reference types that implement one of the `Deref` traits could be a major point of confusion in the learning process.

# Unresolved questions

None.
