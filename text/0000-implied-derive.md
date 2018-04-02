- Feature Name: `implied_derive`
- Start Date: 2018-04-03
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

When deriving standard library such as `Copy`,
the transitive closure of all super traits will also be implicitly derived.

# Motivation
[motivation]: #motivation

## Fewer surprises for beginners

For a beginner who tries to derive `Copy` only to get an error message:

```rust
error[E0277]: the trait bound `{Type}: std::clone::Clone` is not satisfied
```

it can seem incomprehensible why the deriving system can't just derive `Clone`
as well. Removing this need will aid in lowering the barrier to entry a bit.

## Ergonomics

Consider a type such as `Option<T>` defined as:

```rust
#[derive(Clone, Copy, PartialEq, PartialOrd, Eq, Ord, Debug, Hash)]
pub enum Option<T> {
    None,
    Some(T),
}
```

Across the ecosystem, it is quite common to derive a large number of
standard library traits such as done in the case of `Option<T>`.

There is however a great deal of redundance here.
With this RFC, we can get rid of needless mention of super traits,
and instead write:

```rust
#[derive(Copy, Ord, Debug, Hash)]
pub enum Option<T> {
    None,
    Some(T),
}
```

This definition is significantly more terse and thus more ergonomic when
rapidly prototyping. As a benefit to improved ergonomics, newtypes become
more encouraged as you have to explicitly derive fewer traits.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

## Vocabulary and definitions

- The **cartesian product** `S × T` of the sets `S` and `T`
  is defined as `S × T = {(s, t) | s ∈ S ∧ t ∈ T}`.

- A **binary relation** `R` on sets `S` and `T` is a subset of `S × T`.

- A **transitive relation** `R ⊆ A × A` is a binary relation such that:
  `∀ x, y, z ∈ A. (x, y) ∈ R ∧ (y, z) ∈ R => (x, z) ∈ R`.
  That is: for all elements `x`, `y`, and `z` in the set `A`,
  if `x` and `y` are related, and `y` and `z` are related,
  then `x` and `z` must also be related.

- The **transitive closure** `R+` of a transitive relation `R` is defined as:
  ```
  R_0       = R
  R_{i + 1} = R_i ∪ {(s, u) | ∃ t. (s, t) ∈ R_i ∧ (t, u) ∈ R_i}
  ```
  From the perspective of an element `x` related to some `y` via `R`,
  i.e: `x R y`, the transitive closure can be seen as all the `y`s you can
  reach from `x` in one or more steps.

- **super trait** - For a trait `Copy`, defined as `trait Copy : Clone {}`,
  the trait `Clone` is a super trait of `Clone`. We also say that if `T: Copy`,
  for some type `T`, then `T: Clone`, or in other words:
  `Copy` implies `Clone` (denoted `Copy => Clone`).

## New concepts

- An **implicitly derived** trait is a trait `T` which gets implemented for
  a type because the type has `#[derive(S)]`, where `T` is in the transitive
  super-trait closure of `T`.

## Practical implications

Previously, it was not enough to `#[derive(Copy)]` to also implement `Clone`
for the type being derived. Instead, you had to `#[derive(Copy, Clone)]`.
With this RFC, you can `#[derive(Copy)]`, and then `#[derive(Clone)]` is implied.
This change also applies to `Ord` and all other, now or future,
derivable standard library traits.

In other words, we have that:

+ `#[derive(Copy)]` => `#[derive(Clone)]`
+ `#[derive(Ord)]` => `#[derive(Eq, PartialEq, PartialOrd)]`
+ `#[derive(PartialOrd)]` => `#[derive(PartialEq)]`
+ `#[derive(Eq)]` => `#[derive(PartialEq)]`

### Lints

+ If you explicitly derive a trait that can be implicitly derived,
  then a warning named along the lines of `explicitly_derived_super_trait`
  will be issued recommending that you remove the implictly derived trait.
  You are encouraged as a user to heed this warning.

### Errors

+ `#[derive(Clone, Clone)]` will no longer issue an error about conflicting
  implementations.

### Custom derive

+ This RFC does **not** affect the behavior of `#[derive(..)]` for traits
  which are custom derived. However, for a more consistent experience across
  the ecosystem, custom derive macro authors are encouraged to implement
  super-traits to the extent possible.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## Changes

The transitive super-trait closures of all derivable standard library traits are:

```
Copy => {Clone}
Clone => {}
Ord => {Eq, PartialEq, PartialOrd}
PartialOrd => {PartialEq}
Eq => {PartialEq}
PartialEq => {}
Hash => {}
Default => {}
Debug => {}
```

For each of these traits `T` on the LHS, and all future derivable traits,
`#[derive(T)]` will now imply `#[derive(C)]` for all `C` in the transitive
super-trait closure of `T`.

### Union, not concatenation

When the compiler is dealing with the union of all explicitly derived and
implicitly derived traits, the union of those traits will be derived instead of
the concatenation. This is particularly important for two reasons:

1. To minimize the breakage in transitioning to edition 2018.
   If you in the future do `#[derive(Copy, Clone)]`,
   then the deriving system will see this as `#[derive(Copy, Clone)]`
   and not `#[derive(Copy, Clone, Clone)]`.

2. `Eq` and `PartialOrd` both imply `PartialEq`, and so if the concatenation is
   used rather than the union, then the compiler will derive `PartialEq` twice
   which will result in overlapping impls.

### `#[structural_match]`

[RFC 1445]: https://github.com/rust-lang/rfcs/pull/1445

This RFC supercedes [RFC 1445] in that `#[derive(Eq)]`, where transitively
applied to the transitive closure of all field types of a type, implies
`#[structural_match]`. In other words, the requirement of transitively having
`#[derive(PartialEq, Eq)]` is reduced to `#[derive(Eq)]`.

### Interaction with `#[derive_no_bound]` et al.

[RFC 2353]: https://github.com/rust-lang/rfcs/pull/2353

To ensure consistency, `#[derive_no_bound(Trait)]` as described in [RFC 2353]
should also imply `#[derive_no_bound(Super)]` where `Super` is the
transitive super-trait closure of `Trait`. Conversely,
`#[derive_field_bound(Trait)]` will should imply `#[derive_field_bound(Super)]`.

# Drawbacks
[drawbacks]: #drawbacks

There are two main drawbacks of this proposal.

## Breakage

This RFC will break some code. Specifically, if a user has already manually
given an `impl` for a trait in the transitive closure of super-traits for a
particular trait, then an error will be raised. An example of this situation is:

```rust
#[derive(Copy)]
struct Foo;

impl Clone for Foo {
    fn clone(&self) -> Self { Self {} }
}
```

which, in the future, would be equivalent to:

```rust
#[derive(Copy, Clone)]
struct Foo;

impl Clone for Foo {
    fn clone(&self) -> Self { Self {} }
}
```

and thus resulting in the error:

```rust
error[E0119]: conflicting implementations of trait `std::clone::Clone` for type `Foo`:
 --> src/main.rs:2:20
  |
2 |     #[derive(Copy, Clone)]
  |                    ^^^^^ conflicting implementation for `Foo`
...
5 |     impl Clone for Foo {
  |     ------------------ first implementation here
```

### Mitigating factors

1. We can do this breakage as part of edition 2018.

2. We can give good error messages and help users to migrate with `rustfix`.

3. It is expected that the breakage will be relatively small because situations
   where `Copy` is derived but `Clone` is implemented is rare.
   Furthermore, it `Ord` it could be downright risky to derive `Ord` but
   manually implement `PartialEq`.

## Readability

Arguably, this RFC optimizes for writing ergonomics instead of reading.
With this RFC implemented, it will be less clear that some traits are derived
or even implemented from the source code alone since some derived traits are
only implicitly derived.

### Mitigating factors

However, with the future work outlined in the subsection [rustdoc improvements],
this drawback can be mitigated.

## Surprising for Haskell developers

As outlined in the section [prior-art], this RFC moves away from the behavior
of deriving in Haskell, and therefore, the behavior could be surprising for
Haskell developers.

### Mitigating factors

It is the RFC author's personal experience that Haskell developers in general
are early adopters and therefore used to change and are unrigid in their
expectations of languages.

# Rationale and alternatives
[alternatives]: #alternatives

The only seemingly viable alternative to this RFC is to not do this.
At this time, there does not seem to be any other alternative design.

# Prior art
[prior-art]: #prior-art

Since deriving was a feature inspired by Haskell,
we take a look at how Haskell deals with deriving and super traits.

The `Ord` type class (equivalent to a trait in Rust) is defined like so:

```haskell
class Eq a => Ord a where
  compare :: a -> a -> Ordering
  (<) :: a -> a -> Bool
  (<=) :: a -> a -> Bool
  (>) :: a -> a -> Bool
  (>=) :: a -> a -> Bool
  max :: a -> a -> a
  min :: a -> a -> a
  {-# MINIMAL compare | (<=) #-}
```

As in Rust, `Ord` has `Eq` in its transitive super-class closure in Haskell.

We write the following into GHCi, the REPL of GHC, the Glasgow Haskell Compiler:

```haskell
ghci> data Foo = Bar deriving Ord
```

and we get back:

```haskell
<interactive>:1:25: error:
    * No instance for (Eq Foo)
        arising from the 'deriving' clause of a data type declaration
      Possible fix:
        use a standalone 'deriving instance' declaration,
          so you can specify the instance context yourself
    * When deriving the instance for (Ord Foo)
```

The error here is equivalent in nature to the error:

```rust
error[E0277]: the trait bound `main::Foo: std::clone::Clone` is not satisfied
 --> src/main.rs:2:14
  |
2 |     #[derive(Copy)]
  |              ^^^^ the trait `std::clone::Clone` is not implemented for `main::Foo`
```

raised by the following snippet:

```rust
fn main() {
    #[derive(Copy)]
    struct Foo;
}
```

The conclusion is therefore that the prior art is in favor of the status quo of
deriving in Rust and that we would depart from that with this RFC.

# Unresolved questions
[unresolved]: #unresolved-questions

This section outlines some unresolved questions which should be resolved prior
to merging this RFC.

## `#[derive(only(Eq))]`

To regain the ability to derive a subtrait but manually implement a supertrait,
the compiler could allow the modifier `only` on impls. One advantage this has
is that expressive power is mostly retained (the difference is negligible).
Another advantage that `rustfix` could migrate derives from edition 2015 to 2018
by simply prefixing every derived trait in `#[derive(..)]` with `only`. This
should be contrasted with `rustfix` telling the user that they should insert
a, in some cases, quite large, blob of code instead.

### Syntactic bikeshed

As with most proposals, the lexical syntax of `only` is up for bikeshedding.
In particular, there are currently three possible notations:

- `#[derive(only(Eq))]`

This notation has the problem that if we ever allowed the notation `F(A) -> B`
for any trait `F` of the form:

```rust
trait F<A> {
    type Output;
}
```

as shorthand for `F<A, Output = B>` as is the case with the `Fn` trait,
and `F(A)` as shorthand for `F<A, Output = ()>`, then `only(Eq)` could be
interpreted as `only<Eq, Output = ()>`. Having such a trait of the form

```rust
trait only<trait T> {
    type Output;
}
```

seems however quite unlikely. Furthermore, the trait name and the derive macro
name need not coincide, even if it is a strong recommendation.
Therefore, this problem might not be a problem in practice.

- `#[derive(only Eq)]`

This syntax reads quite lightly since it has fewer parenthesis involved.
The main drawback here is that we must change the grammar of attributes
to accept `$ident $ident` as a valid form.

Another drawback is that `only` could be an effect if we ever adopt some
more explicit effect system which would allow things such as
`#[derive(async Foo)]` and `#[derive(const Foo)]`. It seems quite unlikely
however that there should be an effect called `only` even in the event that
we do gain a more elaborate effect system.

- `#[derive_only(Eq)]`

This particular syntax does not have any conflicts with anything else.
However, it does mean that you have to separate `only` and non-`only` derived
traits. This drawback is not particularly good.

# Future work

This section outlines some possible future work.

## `#[proc_macro_derive(Sub, implies(Super))]`

To further enable a consistent experience from built-in derive
macros for standard library traits as well as to custom-derive macros,
a futher development of the custom derive API could be to allow authors
to specify an implied list of traits like so:

```rust
#[proc_macro_derive(Subtrait, implies(SupertraitA, SupertraitB))]
pub fn derive_subtrait(input: TokenStream) -> TokenStream { ... }
```

This would insert `#[derive(SupertraitA, SupertraitB)]` on the type before
expanding of the derive macros and would be visible to all other custom
derive macros.

This would be opt-in, which would mean that `implies(..)` would be optional to
specify. The key `implies` here is of course up for bikeshedding.

## `rustdoc` improvements
[rustdoc improvements]: #rustdoc-improvements

["Auto Trait implementations"]: https://doc.rust-lang.org/nightly/std/option/enum.Option.html#synthetic-implementations

As done with the section ["Auto Trait implementations"],
the documentation generated by `rustdoc` could show implicitly derived traits
in a section named "Implicitly Derived Trait implementations" below a section
which is named "Derived Trait implementations".