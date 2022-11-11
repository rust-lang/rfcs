- Feature Name: `at-least-one-feature`
- Start Date: 2022-11-11
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Allow packages to require that dependencies on them must specify at least one feature (the `default` feature counts).
This avoids backwards compatibility problems with `default-features = false`.

# Motivation
[motivation]: #motivation

A major use-case of Cargo features to take previously mandatory functionality and make it optional.
This is usual done in order to make the code more portable than it was previously, while not breaking existing consumers of the library.
Consider this example which shoes both work works and what doesn't:

1. Library has no features.

2. A `foo` feature is added, gating functionality that already existed.
   It is on by default.
   `no-default-features = true` can be used in which some dependencies only needed for `foo` can be avoided.
   Yay!

3. A `bar` feature is added, gating functionality that already existed.

   - Suppose it is off by default.
     Oh no!
     All Existing use-cases break because the functionality that depends on `bar` goes away.

   - Suppose it is on by default, or depended-upon by `bar`.
     Oh no!
     Existing `no-default-features = true` are now broken!
     They want a feature set of `{bar}` which would correspond to the old `{}`, but there is no way to arrange it.

In step two, we could "ret-con" the empty feature set with the `default` feature.
But this is a trick that can only be pulled once.
The second time around, we already have a default feature; we are out of luck.

The previous attempts attempted to make new default features, or migrate the requested feature sets.
But that is complex.
There is exactly a simpler solution: simply require that *some* feature always be depended-upon.

To see why this works, it helps to first see that step 2 was *already* broken.
Here's the thing, even though there previously were not any features, that *doesn't* mean there were not any `no-default-features = true` users!
Sure, it wouldn't do anything for crate with new features, but one can still use it.
Then when just `bar` is added, we already have a problem, because the `default` feature will no "catch" all the existing users ---
the mischievous users that were already using `no-default-features = true` will have their code broken!

This brings us to the heart of the problem.
So long as users are depending on "something", we can be careful to make sure those features keep their meaning.
But when users are depending on nothing at all with `no-default-features = true` and an empty feature set, we have nothing to "hook into".
The simple solution is just to rule out that problem entirely!

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Packages with
```toml
[package]
at-least-one-feature = true
```
are easier to maintain!
You don't need to worry about the `no-default-features = true, features = []` case anymore.
You can be sure that all consumers must dependent on either `default` or a regular named feature.

Whenever you want to make existing functionality more conditional, simply extend the set of features the code relies on with a new feature, and ensure existing features depend on that feature.

For example:
```rust
fn my_fun_that_allocates() { .. }

#[cfg(all(feature = "foo", feature = "bar"))]
fn my_weird_fun_that_allocates() { .. }
```
becomes:
```rust
#[cfg(feature = "baz")]
fn my_fun_that_allocates() { .. }

#[cfg(all(feature = "foo", feature = "bar", feature = "baz"))]
fn my_weird_fun_that_allocates() { .. }
```

And the corresponding `Cargo.toml`:
```toml
[features]
default = ["foo"]
bar = ["foo"]
```
becomes:
```toml
[features]
default = ["foo", "baz"]
foo = ["baz"]
bar = ["foo"] # no need to add "baz" because "bar" picks it up
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

Depending on a
```toml
[package]
at-least-one-feature = true
```
crate with an empty feature set is disallowed and invalidates the solution.
The `default` feature counts as a member of that set when `default-features = true`.
 
The solver shall avoid such solutions (so as not break old versions of libraries without `at-least-one-feature` being "discoverable").

## Provisional Theory

We can begin to formalize library compatibility with something like
[this](https://q.uiver.app/?q=WzAsNCxbMCwwLCJcXG1hdGhjYWx7UF/PiX0oXFxtYXRocm17RmVhdHVyZXN9X3tcXG1hdGhybXtPbGR9fSkiXSxbMSwwLCJcXG1hdGhjYWx7UF/PiX0oXFxtYXRocm17RmVhdHVyZXN9X3tcXG1hdGhybXtOZXd9fSkiXSxbMCwxLCJcXG1hdGhybXtSdXN0fSJdLFsxLDEsIlxcbWF0aHJte1J1c3R9Il0sWzAsMSwiXFxtYXRocm17c2FtZVxcIG5hbWVzfSIsMCx7InN0eWxlIjp7InRhaWwiOnsibmFtZSI6Imhvb2siLCJzaWRlIjoiYm90dG9tIn19fV0sWzAsMiwiXFwjW1xcbWF0aHR0e2NmZ30oLi4uKV1fXFxtYXRocm17T2xkfSIsMl0sWzEsMywiXFwjW1xcbWF0aHR0e2NmZ30oLi4uKV1fXFxtYXRocm17TmV3fSJdLFszLDIsIlxcbWF0aHJte2BgdXBjYXN0XCJcXCBsaWJyYXJ5XFwgaW50ZXJmYWNlc30iXV0=)
[commutative diagram](https://en.wikipedia.org/wiki/Commutative_diagram).

- The old and new features form a partial order

- P_ω takes those partial orders to the partial order of their downsets.
  (That is, sets of features with the implied features from feature dependencies "filled in", ordered by inclusion.)

- "same names" maps the old downsets to the new downsets, filling in any newly implied features as needed.

- `#[cfg(...)]` is the mapping of feature sets to exposed library interfaces

- "'upcast' library interfaces" forgets whatever unrelated new stuff was added in the new library version

The idea is that going from the old features directly to the old interfaces, or going the "long way" from old features to new features to new interfaces to old interfaces should yield the same result.

For the more part, features can be "interspersed" anywhere the old feature partial order to make the new feature partial order.
However, this is an exception!
The old empty downset becomes the new empty downset, which means nothing can be added below it.
This is the `default-features = false` gotcha!

When we disallow the empty feature set, we are replacing P_ω with the "free join-semilattice" construction.
We are enriching features with the ∨ binary operator but no ⊥ identity element.
There is no empty downset becomes empty downset constraint, and thus we are free to add new features below all the others all we want.

# Drawbacks
[drawbacks]: #drawbacks

I can't really think of a reason, it's much simpler than the prior attempts!

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

An alternative is not to ban the empty feature set, but ensure it always translates to the empty library.
I.e. to require that *all* items must be dependent upon *some* feature; everything needs a `cfg`.

Returning to our half-worked-out theory, instead of banning a notion of a ⊥ empty feature set that must be preserved from the old library to the new (removing a requirement), we are adding a *new* requirement that the ⊥ feature set must map to the ⊥ Rust interface.
We still have the restriction that new features can be added below, but this restriction is no longer a problem:
there is no point of adding such a new minimal feature because there is nothing left to `cfg`-out!

This solution is more mathematically elegant, but it seems harder to implement.
It is unclear how Cargo could require the Rust code to obey this property without new infra like the portability lint.

# Prior art
[prior-art]: #prior-art

This is a well-known problem.
See just-rejected [#3283](https://github.com/rust-lang/rfcs/pull/3283), 
and my previous retracted [#3146](https://github.com/rust-lang/rfcs/pull/3146).

I think this is much simpler than the other two.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

It would be nice to completely work out the theory.

# Future possibilities
[future-possibilities]: #future-possibilities

The `default-features = false` syntax is clunky.
A new edition could say that an explicit feature list always means `default-features = []`, but that `default` can be used in feature lists.
With this change, "must depend on one feature, including possibly the default feature" becomes easier to explain:

- `[]` disallowed
- `["default"]` allowed
- `["foo"]` allowed
