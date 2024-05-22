- Feature Name: `on_unimplemented_trait_use`
- Start Date: 2024-05-22
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Add `[diagnostic::on_unimplemented_trait_use]` in `#[diagnostic]` on structs that will influence error messages emitted by unsatisfied traits bounds.

# Motivation
[motivation]: #motivation

The idea came out when I was trying to print out a PathBuf, there's a custom message said: 
>in format strings you may be able to use `{:?}` (or {:#?} for pretty-print) instead  
call `.display()` or `.to_string_lossy()` to safely print paths, as they may contain non-Unicode data

And found out its hardcoded in trait `Display`
```rust
#[rustc_on_unimplemented(
    on(
        any(_Self = "std::path::Path", _Self = "std::path::PathBuf"),
        label = "`{Self}` cannot be formatted with the default formatter; call `.display()` on it",
        note = "call `.display()` or `.to_string_lossy()` to safely print paths, \
                as they may contain non-Unicode data"
    ),
    message = "`{Self}` doesn't implement `{Display}`",
    label = "`{Self}` cannot be formatted with the default formatter",
    note = "in format strings you may be able to use `{{:?}}` (or {{:#?}} for pretty-print) instead"
)]
pub trait Display {...}
```
It would be nice if such messages are implemented on the struct itself, so crate mantainer can add a detail explanation when there're some special structs which don't impl some "usually expected" traits, like `Display`, `Eq`, `PartialEq` or `Hash`.

For example:
```rust
#[diagnostic::on_unimplemented_trait_use(
    trait = Display,
    message = "`{Self}` doesn't implement `{Display}`",
    label = "`{Self}` cannot be formatted with the default formatter; call `.display()` on it",
    note = "call `.display()` or `.to_string_lossy()` to safely print paths, \
                as they may contain non-Unicode data"
)]
struct PathBuf;
````

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

if the struct impl both this attribute and the trait, it should gives an error like the one when you're trying to impl the same trait multiple times. E.g.
> conflicting implementations of trait `Debug` for type `MyStruct`

