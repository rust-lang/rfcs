- Feature Name: prefix_type_param
- Start Date: 2015-07-08
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Let specified generic type parameter lists be abbreviated, that is,
`foo::<u8, _>` and `foo::<u8>` are equivalent.

# Motivation

There are times when generic functions need to have their type
parameters specified, requiring the use of the `::<...>`
syntax. Before this RFC, doing this would require every type parameter
to have something written, even if it was just `_`.

```rust
fn foo<T, U, V>() -> T {
    // ...
}

// equivalent:
let x: u8 = foo();
let y = foo::<u8, _, _>();
```

It is very possible to have APIs that only require a subset of the
type parameters to be specified in the common case (the rest can be
correctly deduced from other information). It is syntactically nice to
allow not writing `_` when it seems "obviously" unnecessary. The above
example could be written:

```rust
let z = foo::<u8>();
let w = foo::<u8, _>();
```

One concrete motivation for this is a new design for the
`rand::random` to allow more flexibility. It would have signature:

```rust
fn random<T: Rand, D: IntoRand<T>>(d: D) -> T
```

Generally, the type `D` can be deduced from the call itself, but the
return type requires more information. Either an external type hint
like `let x: u8 = random(..);`, or an explicit function annotation
`random::<u8, _>(..)`. Since it is easily deducable, the second
parameter can usually be left as `_`. It would hence be nicer to be
able to write `random::<u8>`.

This also makes it easier to add type parameters without downstream
code having to change.

# Detailed design

When the compiler handles a type-specified function, instead of
emitting `error: too few type parameters provided: expected N
parameter(s) , found M parameter(s) [E0089]`, it mechanically fills in
the remaining `N - M` parameters with `_`.

This RFC only applies to expressions. It is not proposing that it be
legal to abbreviate `let x: Foo<T, _>` as `let x: Foo<T>`.

# Drawbacks

- It is not possible to specify that all type parameters are
  specified, so that it is an error and requires adjustment if more
  are added upstream. It isn't so clear to the author which default is
  right (syntax for "closing" a specification list could be added if
  the version in this RFC is decided to be the right default but being
  definite is still desirable).

# Alternatives

- Type ascription may solve much of the motivation for doing this,
  since it becomes easier to type-hint things. E.g. the `random`
  example can becomes `random(..): u8`.
- Require noting that parameters have been left to inference,
  e.g. `random::<u8, ...>`. This defeats much of the syntactic point,
  and also means it is less helpful for backwards-compatibility
  tricks.

# Unresolved questions

None right now.
