- Feature Name: `assoc_math_consts`
- Start Date: 2023-04-17
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Add the constants in std::f32::consts and std::f64::consts as associated
constants to the f32 and f64 types (e.g. f32::PI).

# Motivation
[motivation]: #motivation

Currently mathematical constants such as π live in
std::{f32, f64}::consts. This is difficult to type and read if written out in full everywhere.
Consider
```rust
assert_eq!(std::f32::consts::FRAC_PI_4.sin(), std::f32::consts::FRAC_1_SQRT_2);
```
vs
```rust
assert_eq!(f32::FRAC_PI_4.sin(), f32::FRAC_1_SQRT_2);
```
While it is possible to `use std::f32::consts as f32c;` or similar, it could be cumbersome to do that in every file
in a project which uses mathematical constants heavily.

Also new users of Rust might expect mathematical constants to be there.
Currently
NAN, INFINITY, etc. are associated constants, and it might
be confusing for f32::NAN to exist, but not f32::PI.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

When writing code which uses mathematical constants, the associated constants in f32 and f64 can be used. For example,
the function f(*x*) = (π/*e*)<sup>*x*</sup> can be written as:

```rust
fn f(x: f32) -> f32 {
	(f32::PI / f32::E).powf(x)
}
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The following constants will be added with their values taken from
the respective constants in std::f32::consts and std::f64::consts.

- f32::{PI, TAU, FRAC\_PI\_2, FRAC\_PI\_3, FRAC\_PI\_4, FRAC\_PI\_6, FRAC\_PI\_8, FRAC\_1\_PI,
FRAC\_2\_PI, FRAC\_2\_SQRT\_PI, SQRT\_2, FRAC\_1\_SQRT\_2, E, LOG2\_E, LOG2\_10, LOG10\_E, LOG10\_2,
LN\_2, LN\_10}
- f64::{PI, TAU, FRAC\_PI\_2, FRAC\_PI\_3, FRAC\_PI\_4, FRAC\_PI\_6, FRAC\_PI\_8, FRAC\_1\_PI,
FRAC\_2\_PI, FRAC\_2\_SQRT\_PI, SQRT\_2, FRAC\_1\_SQRT\_2, E, LOG2\_E, LOG2\_10, LOG10\_E, LOG10\_2,
LN\_2, LN\_10}

# Drawbacks
[drawbacks]: #drawbacks

Currently it is not possible to `use` associated constants, so in order to `use` mathematical constants,
it will still be necessary to refer to them by their paths in std.

Even if `use`-ing associated constants does become possible in a future version of Rust,
it will likely never be possible or a good idea to do `use f32::*;` whereas
`use std::f32::consts::*;` is perhaps more reasonable (although, it's worth mentioning
that more mathematical constants could be added in future versions of Rust, so using `*`
is probably not a good idea in general).

This proposal would add many more items to the `f32` and `f64` types, which might not be desirable.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

- The mathematical constants in std could be (planned to be) deprecated.
  The drawback as mentioned above is that f32::PI, etc. are not `use`-able.
  The benefit is that there would be a clear "preferred" path for mathematical constants.
- We could create a trait similar to [num::traits::FloatConst](https://docs.rs/num/0.4.0/num/traits/trait.FloatConst.html)
  which houses these constants. The benefits are that the constants can be used in a generic
  context and we wouldn't be crowding the `f32` and `f64` types.
  The downside would be that adding a new constant would be a breaking change
  for any user types implementing the trait (unless it was made externally unimplementable
  via the usual "hack" of a pub trait in a private module).
- Creating a type for each constant, e.g. `struct Pi;` then implementing
  `From<f32>`, `From<f64>` for them (and perhaps also `Add<f32>`, etc.).
  The benefits are that these could be used in generic contexts,
  wouldn't be crowding the `f32`/`f64` types, and
  non-std types could add `From<T>` implementations
  for the constants. The downsides are that `f32::from(Pi)` (which you
  might end up needing to use in a lot of cases to get type inference to work)
  is a bit less "ergonomic" than `f32::PI`, the names would need to change
  from `CONSTANT_CASE` to `TypeCase`, and it might be less intuitive
  (especially for beginners) that mathematical constants are structs and not
  constants.


# Prior art
[prior-art]: #prior-art

- The crates `half` and `fixed` have these as associated constants on their types.
- RFC 2700 originally proposed to do this, but since it was "met with mild resistance",
  it was decided that it would be left for a later RFC since the focus of 2700
  was on adding integral associated constants (e.g. u32::MAX).

# Unresolved questions
[unresolved-questions]: #unresolved-questions

# Future possibilities
[future-possibilities]: #future-possibilities

- In the future, especially if associated constants become `use`-able,
  we could deprecate std::{f32, f64}::consts.
