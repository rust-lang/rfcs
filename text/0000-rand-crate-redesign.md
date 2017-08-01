- Feature Name: rand crate redesign
- Start Date: 2017-08-01
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Evaluate options for the future of `rand` regarding both cryptographic and
non-cryptographic uses.

See also:

* [Crate evaluation thread]
* [Strawman design PR]
* [Strawman design doc]

# Motivation
[motivation]: #motivation

The [crate evaluation thread] brought up the issue of stabilisation of the `rand`
crate, however there appears to be widespread dissatisfaction with the current
design. This RFC looks at a number of ways that this crate can be improved.

The most fundamental question still to answer is whether a one-size-fits-all
approach to random number generation is sufficient (*good enough*) or whether
splitting the crate into two is the better option: one focussed on cryptography,
the other on numerical applications.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Since this concerns one (or more) crates outside the standard library, it is
assumed that these crates should be self-documenting. Much of this documentation
still needs writing, but must wait on design decisions.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## Generation API

This section concerns the `Rng` trait, but not specifically implementations or
generation of values of other types.

Aside: one proposal is to rename `Rng` to `Generator`.

The `Rng` trait governs what types can be generated directly, and there appears
to be consensus on removing all convenience functions not concerned with
value generation. Doing so would leave:

```
trait Rng {
    fn next_u32(&mut self) -> u32
    fn next_u64(&mut self) -> u64
    
    fn next_f32(&mut self) -> f32
    fn next_f64(&mut self) -> f64
    
    fn fill_bytes(&mut self, dest: &mut [u8])
}
```

Further, although [direct generation of floating-point random values is
possible](http://www.math.sci.hiroshima-u.ac.jp/~m-mat/MT/SFMT/#dSFMT), it is
proposed to remove `next_f32` and `next_f64`. This simplifies the API and removes
non-trivial integer to float conversions from the trait, but is not truely
necessary.

Next, it has been suggested that `next_u8`, `next_u16` and (where supported)
`next_u128` should be added. That gives:

```
trait Rng {
    fn next_u8(&mut self) -> u8
    fn next_u16(&mut self) -> u16
    fn next_u32(&mut self) -> u32
    fn next_u64(&mut self) -> u64
    fn next_u128(&mut self) -> u128
    
    fn fill_bytes(&mut self, dest: &mut [u8])
}
```

For crypto purposes, it has been suggested that `fill_bytes` alone would be
sufficient. For non-crypto purposes, the other methods (at least 32-bit and
64-bit variants) are desirable for performance, since many RNGs natively
produce `u32` or `u64` values.

Further to this, there is discussion on whether these methods should all return
a `Result`. Apparently, some crypto RNGs can estimate available entropy and
detect cycles. A properly seeded cryptographic generator should be able to
produce a very long sequence of strong cryptographic numbers, but without
sufficient entropy for initialisation the generated numbers could be guessable.
Further, on many platforms, `OsRng` could fail (presumably due to the same
initialisation problem); the current implementation can panic.
Some people therefore advocate changing the above
functions to each return a `Result`.

From the point of view of non-cryptographic numeric random number generation,
RNGs are usually fast, deterministic functions which have no means to detect
errors. Some may be able to detect lack of initialisation, but some implementations
always initialise with a fixed seed if no custom seed is provided. These may cycle,
likely will be unable to detect cycles (note that returning the same value twice
does not imply a cycle). Thus for non-crypto usage, returning a `Result` is
unnecessary, would require extra error handling code or `unwrap`s, and ahs some
performance penalty. (Note that all distributions would probably need to return
a `Result` too.)

There is therefore a conflict of design here; [Brian Smith suggests separate
crypto and non-crypto APIs](https://internals.rust-lang.org/t/crate-evaluation-for-2017-07-25-rand/5505/59?u=dhardy)
(and presumably crates). This would allow a minimal crypto trait with a single
`fill_bytes(..) -> Result<..>` method, without impacting performance or
correctness (`unwrap`) of non-crypto code.

My personal feeling is that
[relying on the OS](https://internals.rust-lang.org/t/crate-evaluation-for-2017-07-25-rand/5505/37)
where there are strong crypto requirements is the best choice, and that where a
user-space crypto RNG is required, the best design would be something like as
follows (but I have little experience with cryptography, and this does not allow
error on cycle detection):

```
let mut seed_buf = [u8; LEN];
rand::try_fill_from_os(&mut buf)?;  // this may fail
let mut rng = SomeRng::from_seed(seed_buf);
// after creation, rng can be assumed not to fail
```

Besides the issue of `next_u32` etc. and `fill_bytes` potentially failing,
another advantage of separate crypto and numeric `rand` crates would be absolute
simplicity of the crypto API and crate. Presumably in this case the numeric
crate would still depend on the crypto crate for correct initialisation.

Further, should the `Rng` trait allow entropy injection and estimation of
available entropy? Obviously many RNGs won't be able to do the latter.
Entropy injection might be a viable alternative to periodic reseeding.

## Generators

This section concerns implementations of `Rng`.

`OsRng` currently implements `Rng` by directly sampling from whatever OS
functionality is available. It might be preferable to implement a
platform-specific `try_fill_from_os(buf: &mut [u8]) -> Result<()>` function,
and (possibly) implement `OsRng` as a wrapper around this.
This approach might be slightly less performant when pulling random numbers
directly from the OS, but the overhead is probably insignificant relative to
the system call, and may often be zero.

Three user-space RNGs are currently provided. Should this change? And should the
aim be to build a selection of high quality generators or keep the list short?
Are there any other RNGs worth adding now?

* `IsaacRng` (32-bit) and `Isaac64Rng`
* `ChaChaRng`
* `XorShiftRng`

The appropriate 32 or 64 variant of Isaac is exposed as `IsaacWordRng`. While
the concept is good, the name is weird.

`StdRng` is currently a wrapper around `IsaacWordRng`, with a `new()` method
that seeds from `OsRng`. Possibly this should be replaced with two wrapper structs
or simply re-bound names: `FastRng` and `CryptoRng`.

`thread_rng()` current constructs a reference-counted periodically-reseeding
`StdRng` per thread on first use. TODO: allow user override via dynamic dispatch?
Rename to `crypto_rng()`?

`weak_rng()` currently constructs a new `XorShiftRng` seeded via `OsRng` each
time it is called. Rename to `fast_rng()` and make it use a `FastRng` type?
What about `random()`, should for example the documentation point out that
creating a `weak_rng()` may be useful for performance where crypto-strength
generation is not needed?

### Generator adaptors

`ReseedingRng` is a wrapper which periodically reseeds the enclosed RNG.

Should a similar wrapper to periodically inject entropy from the OS be added?
Of course this shouldn't be necessary normally, but it might help when (a) the
initial OS-provided seed had little entropy and (b) cycles might otherwise occur.

The `SeedableRng` trait is an optional extra allowing reseeding:

```
pub trait SeedableRng<Seed>: Rng {
    fn reseed(&mut self, _: Seed);
    fn from_seed(seed: Seed) -> Self;
}
```

## Random values

This section concerns creating random values of various types and with various
distributions given a generator (`Rng`).

This part of the design already has a fairly good story in the strawman design,
namely the [`Rand` trait] and associated
[`Distribution` trait and impl of Rand](https://github.com/dhardy/rand/blob/master/src/distributions/mod.rs#L58), the available distributions, and the
[`random`](https://dhardy.github.io/rand/rand/fn.random.html) and
[`random_with`](https://dhardy.github.io/rand/rand/fn.random_with.html)
convenience functions.

The `Rand::rand(rng, distribution)` function takes the `distribution` parameter
by value; this might cause extra copying in some cases. But most distributions
are small or zero-size; the `Gamma`-derived ones are the only ones larger than
three `f64`s, and the copy can likely be optimised out(?).

### Distributions

The [`Distribution`](https://dhardy.github.io/rand/rand/distributions/trait.Distribution.html)
trait replaces `rand`'s current
[`IndependentSample`](https://docs.rs/rand/0.3.15/rand/distributions/trait.IndependentSample.html)
trait. The `Sample` trait is removed; I believe it was originally intended for use
in random processes like random walks; these are discrete-time (stochastic)
models, thus `advance_state()` and `get_state()` functions are more applicable
than `sample()`; in any case this is beyond the scope of `rand`.

Several zero-size structs implementing `Distribution` specify simple distributions:

*   `Uniform` specifies uniform distribution over the entire range available, and
    is implemented for all integer types and `bool`
*   `Uniform01` specifies uniform distribution over the half-open range `[0, 1)`,
    and is implemented for `f32` and `f64`
*   `Closed01` is like `Uniform01` but for `[0, 1]` (thus including 1.0)
*   `Open01` is like `Uniform01` but for `(0, 1)` (thus excluding 0.0)
*   `Default` chooses `Uniform` or `Uniform01` depending on type, and could
    be extended to other types.

`Rand<Default>` is roughly the same as the
[old `Rand`](https://docs.rs/rand/0.3.15/rand/trait.Rand.html); currently it doesn't
support arrays, tuples, `Option`, etc., but it could conceivably, and probably
also `derive(Rand)`. The others are new.

There is one further uniform distribution:

*   `Range` specifies uniform distribution over a range `[a, b)` and supports
    integer and floating-point types

This `Range` is unchanged from the current `rand`, and cannot be extended to
user-defined types despite the presence of a backing trait, `SampleRange`.
Possibly this should be adapted, although it should be noted that the internal
details are designed to support a specific set of types, and in any case a
user may create a new `MyRange` type implementing `Distribution`.

Finally, there are several non-uniform distributions, unchanged from the
current `rand`:

*   `Exp`
*   `Normal`, `LogNormal`
*   `Gamma`, `ChiSquared`, `FisherF`, `StudentT`

Currently these are only implemented for `f64`. Probably they could be extended
to `f32` quite easily.

Internally, `Exp(1)` and `N(0, 1)` (standard normal) fixed distributions are
used; these could be exposed via new zero-size distribution structs.
This might be slightly faster for some uses (avoid a multiplication and extra
data access).

Most distributions are implemented in public sub-modules, then *also* imported
into `distributions` via `pub use`. Possibly the sub-modules should be hidden.

### Convenience functions and more distributions

At the top-level of the crate, two convenience functions are available; the
first is roughly equivalent to that in the current `rand` while the second is
new:

*   `random() -> T` using the thread-local `Rng` and generating any value
    supporting `Rand<Default>`
*   `random_with(distribution) -> T` instead generating values using the given
    `distribution` (which can be of any type supporting `Distribution`)

Additionally, within the `distributions` module, some more convenience functions
are available:

*   `uniform(rng) -> T`, equivalent to `Rand::rand(rng, Uniform)`
*   `range(low, high, rng) -> T`, equivalent to `Rand::rand(rng, Range::new(low, high))`

It is debatable whether these are worth keeping and possibly extending to include
`uniform01(rng) -> T` etc. They are convenient when used with iterators (see below).

A couple more distributions are available using functions of the same form,
but (currently) without a `Distribution` implementor representing them:

*   `codepoint(rng) -> char` generating values uniformly distributed over all valid
    Unicode codepoints, even though many are unassigned. (This may be useless?)
*   `ascii_word_char(rng) -> char` uniformly selects from `A-Z`, `a-z` and `0-9`

### Iteration

Iterators are available as wrappers are an `Rng`. These don't support `next`
for compatibility with the borrow checker, but [do support `map` and `flat_map`
as well as `take`](https://dhardy.github.io/rand/rand/iter/struct.Iter.html).
The objects returned by `map` and `flat_map` are
[standard iterators](https://doc.rust-lang.org/std/iter/trait.Iterator.html).

These can be used to generate collections and strings:

```
use rand::{thread_rng, Rng, iter};
use rand::distributions::{uniform, ascii_word_char};

let mut rng = thread_rng();
let x: Vec<u32> = iter(&mut rng).take(10).map(|rng| uniform(rng)).collect();
println!("{:?}", x);
 
let w: String = iter(&mut rng).take(6).map(|rng| ascii_word_char(rng)).collect();
println!("{}", w);
```

This is considerably changed from the current `rand`, which instead has
[`Rng::gen_iter()`](https://docs.rs/rand/0.3.15/rand/trait.Rng.html#method.gen_iter)
using `Rng::gen()` and
[`Rng::gen_ascii_chars()`](https://docs.rs/rand/0.3.15/rand/trait.Rng.html#method.gen_ascii_chars)
for generating random letters (equivalent to `ascii_word_char()`).

### Other stuff

Function [`weighted_bool(n, rng) -> bool`](https://dhardy.github.io/rand/rand/distributions/fn.weighted_bool.html)
is a simple probability function.

The [`sequences` module](https://dhardy.github.io/rand/rand/sequences/index.html)
supports sampling one element from a sequence (`Choose`), sampling several
(`sample`), and shuffling (`Shuffle`).

[`WeightedChoice`](https://dhardy.github.io/rand/rand/sequences/struct.WeightedChoice.html)
is a support type allowing weighted sampling from a sequence.

# Drawbacks
[drawbacks]: #drawbacks

This attempt to redesign `rand` assumes there are no major issues with API
breakage. Several parts of the old API should uremain compatible, but little
attention has been paid to this.

If the crate is split into two crates (crypto and non-crypto), some uses (randomised
algorithms) will not clearly fit into either one, likely many distributions and
support functions will not be available from the crypto API, but possibly a
decent compromise could be reached.

# Rationale and Alternatives
[alternatives]: #alternatives

We *could* leave `rand` as is for now. We could even stabilise the current design.
But I haven't seen anyone advocate this option.

# Unresolved questions
[unresolved]: #unresolved-questions

Lots and lots; see above.

The `derive_rand` sub-crate of the current `rand` provides another method to
generate random values of the current type. This could probably be adjusted to
derive `Rand<Default>` or maybe even support custom distributions. In the
strawman design I simply deleted this sub-crate since I have no interest in
creating random values this way.

[Crate evaluation thread]: https://internals.rust-lang.org/t/crate-evaluation-for-2017-07-25-rand/5505
[Strawman design PR]: https://github.com/rust-lang-nursery/rand/pull/161
[Strawman design doc]: https://dhardy.github.io/rand/rand/index.html
[`Rand` trait]: https://dhardy.github.io/rand/rand/distributions/trait.Rand.html
