- Feature Name: rand crate redesign
- Start Date: 2017-08-01
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Evaluate options for the future of `rand` regarding both cryptographic and
non-cryptographic uses.

There is a strawman revision which implements or demonstrates many of the changes
suggested in this document, but is not entirely in-sync with the suggestions
here.

See also:

* [Crate evaluation thread]
* [Strawman revision PR]
* [Strawman revision code]
* [Strawman revision doc]

# Introduction

## Motivation

The [crate evaluation thread] brought up the issue of stabilisation of the `rand`
crate, however there appears to be significant dissatisfaction with the current
design. This RFC looks at a number of ways that this crate can be improved.

The most fundamental question still to answer is whether a one-size-fits-all
approach to random number generation is sufficient (*good enough*) or whether
splitting the crate into two is the better option: one focussed on cryptography,
the other on numerical applications.

At the finer level, there are many questions, e.g. which members the `Rng`
trait needs, what the `Rand` trait should look like, which PRNG algorithms
should be included, what `thread_rng` should do, and so on.

Once some progress has been made to answering these questions, I will update
the strawman revision accordingly, and if necessary split the corresponding PR
into reviewable chunks. Each chunk can have its own PR discussion as necessary.
Once this is done, we can talk about stabilising parts of `rand` in a separate
PR or RFC.

## Background

A *Pseudo-Random Number Generator*, abbreviated PRNG or simply RNG, is a
deterministic algorithm for generating *random-like* numbers. Such algorithms
typically have a fixed-size state space, a *seed* function to produce an initial
state from some value, an *advance* function to step from one state to the next,
and a *generate* function to yield a value in the output domain (type). This
implies the following properties:

*   Generated numbers are reproducible, when the algorithm and seed (or initial
    state) is known
*   All PRNGs have a finite period: eventually the *advance* function will
    reproduce a prior state (not necessarily the initial one), and the sequence
    of numbers will repeat

Given a fixed state-space of `k` bits, at most `2^k` states are possible. A
good PRNG should have a large period, possibly close to `2^k`; this period
may depend on the seed value, but good PRNGs should ensure a large period for
all seeds.

To be useful, a PRNGs should usually also have the following properties:

*   Require little memory and have fast computation
*   Generated values are uniformly distributed across the output domain
*   The distribution of each value is indepedent of previously generated
    values(1)
*   For cryptographic applications, it is not possible to predict knowledge of
    prior values does not aid in predicting the next value(1)

Note (1): obviously once a PRNG has completed its initial period and started
repeating itself, the above properties are no longer possible. It is therefore
required that the period is very long.

Further, a PRNG may offer a *jump-ahead* function to quickly calculate the
state after a large arbitrary number of steps `v`. This allows a random number
stream to be deterministically partitioned into a number of sub-streams.

L'Ecuyer provides some more background on PRNGs in
[Random Number Generation](https://scholar.google.com/citations?view_op=view_citation&hl=en&user=gDxovowAAAAJ&citation_for_view=gDxovowAAAAJ:d1gkVwhDpl0C) (Springer, 2012).

## Guide-level explanation

Since this concerns one (or more) crates outside the standard library, it is
assumed that these crates should be self-documenting. Much of this documentation
still needs writing, but must wait on design decisions.

In my opinion, [the extensive examples](https://docs.rs/rand/0.3.16/rand/#examples)
in the crate API documentation should be moved to a book or example project.
They are well written, but do not really belong in API documentation.

## Note on type parameters

Very often we make use of type parameters with a restriction like
`R: Rng + ?Sized`. This is *almost* the same as just `R: Rng`, except for one
thing: `R: Rng` doesn't work for dynamic dispatch (where the parameter is a
trait object like `let mut rng: &mut Rng = &mut thread_rng();`).

# Reference-level explanation

## Generation API

This section concerns the `Rng` trait and extension traits, but not
specifically implementations or generation of values of other types.

The `Rng` trait covers *generation* of values. It is not specific to
[deterministic] PRNGs; instead there is an extension trait `SeedableRng: Rng`
covering deterministic seeding.

It has been proposed to rename `Rng` to `Generator`; this proposal has not
seen a lot of support.

### `Rng` trait

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

It has been suggested that `next_u8`, `next_u16` and (where supported)
`next_u128` functions should be added. However, provided that default
implementations are included, these can be added to the trait in the future if
desired. It seems unlikely that PRNGs would offer faster generation of `u8` and
`u16` values than simply casting (`next_u32() as u8`), and the only curent
alternative, OS-based-generation, has high overhead; therefore there appears
little use for `next_u8` and `next_u16` functions. `next_u128` on the other
hand may have some utility: [native 128-bit generators already
exist](http://www.math.sci.hiroshima-u.ac.jp/~m-mat/MT/SFMT/). This may provide
some performance benefit to applications requiring many pairs of 64-bit values,
but I suggest not adding `next_u128` for now unless specifically requested.

```
trait Rng {
    fn next_u32(&mut self) -> u32
    fn next_u64(&mut self) -> u64
    // and possibly:
    fn next_u128(&mut self) -> u128
    
    fn fill_bytes(&mut self, dest: &mut [u8])
}
```

For crypto purposes, it has been suggested that `fill_bytes` alone would be
sufficient. For non-crypto purposes, at least `next_u32` and `next_u64`
are desirable for performance, since many RNGs natively
produce these values.

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
always initialise with a fixed seed if no custom seed is provided. PRNGs could
cycle, but usually have a very long period and no practical way of detecting
cycles (note that returning the same value twice does not imply a cycle, and
that a cycle may return to a state later than the initial state). There is
therefore little use in non-crypto PRNGs returning a `Result`; doing so
would also require extra error handling in user code or `unwrap` within the
library, as well as some performance overhead. All distributions would need to
be adapted.

There is therefore a conflict of design here; [Brian Smith suggests separate
crypto and non-crypto APIs](https://internals.rust-lang.org/t/crate-evaluation-for-2017-07-25-rand/5505/59?u=dhardy)
(and presumably crates). This would allow a minimal crypto trait with a single
`fill_bytes(..) -> Result<..>` method, without impacting performance or
correctness (`unwrap`) of non-crypto code, while PRNGs have simple methods like
`next_u32(&mut self) -> u32`.

In support of this split, there is a strong argument that cryptographic
applications should [relying on OS
generators](https://internals.rust-lang.org/t/crate-evaluation-for-2017-07-25-rand/5505/37)
where possible. Further, keeping cryptography-related crates small may be
useful for security reviews.

**Question:**
If `rand`/`Rng` is split into multiple parts, several approaches are possible;
what to do here is very much undecided.

First, "strong RNGs" designed for cryptographic usage could:

1.  Use a dedicated `CryptoRng` trait, indepedant of `Rng`, with methods
    returning a `Result` on failure
2.  As (1), but add a wrapper struct implementing `Rng` for a `CryptoRng` which
    panics on errors
3.  Use the standard `Rng` trait and be designed such that the only time
    failure is possible is during creation (`fn new() -> Result<Self>`)
4.  Do not include any generators which may fail in `rand` (leave to dedicated
    crypto libraries, if at all) [practically, this is equivalent to 3 aside
    from choice of generators to include]

Second, the `rand` crate could:

1.  Remain a single crate
2.  Have a sub-crate `os_rand` covering only the `OsRng` functionality (via
    simple functions), and have `rand` depend on `os_rand` for initialisation
3.  Have a `rand_core` sub-crate including `OsRng` functionality, the `Rng`
    and/or `CryptoRng` trait(s), and a crypto-approved PRNG; `rand` would
    depend on `rand_core`
4.  Keep all generators in `rand` and move all distributions (including
    `random()`) to a `distributions` crate (or same split but with `rng` and
    `rand` crate names)
5.  Split into even more crates...

*Personally,* I feel that we should stick to a single `Rng` trait as above. I
am not against splitting the implementation into multiple crates, but see
little benefit (a split *might* facilitate review of cryptographic code).

### Debug

The strawman revision now specifies `pub trait Rng: Debug {...}`, i.e. requires
all implementations to implement `Debug`. In many cases this requires only
that implementing types are prefixed with `#[derive(Debug)]`, although in some
cases it adds requirements on internal values.

Is this useful?

### Extension traits

The `SeedableRng` trait should remain as is:

```rust
pub trait SeedableRng<Seed>: Rng {
    fn reseed(&mut self, Seed);
    fn from_seed(seed: Seed) -> Self;
}
```

Another trait could be added to allow jump-ahead:

```rust
pub trait JumpableRng: Rng {
    /// Return a copy of self, mutated as if `next_u32` had been called `steps`
    /// times
    fn jump_u32(&self, steps: usize) -> Self;
    
    // Also jump_u64() ? For most 64-bit generators they should be the same;
    // for 32-bit generators, jump_u64 should jump twice as fast.
}
```

And a trait could allow entropy injection, however I don't believe this belongs
in `rand`. See [suggested trait](https://github.com/rust-lang/rfcs/pull/2106#issuecomment-322414869)
and [my thoughts](https://github.com/rust-lang/rfcs/pull/2106#issuecomment-322705482).

These traits can be added in the future without breaking compatibility, however
they may be worth discussing now.

### Creation of RNGs

The `Rng` trait does not cover creation of new RNG objects. It is recommended
(but not required) that each RNG implement:

*   `pub fn new() -> Self`, taking a seed from [`OsRng`], see below
*   `pub fn from_rng<R: Rng+?Sized>(rng: &mut R) -> Self`
*   `SeedableRng<Seed>` for some type `Seed`

Note that the above won't be applicable to all `Rng` implementations; e.g.
`ReadRng` can only be constructed with a readable object.

`from_rng` is a bit special: in some cases a naive implementation used to seed
a PRNG from another of the same type could effectively make a clone.
Implementations should be careful to prevent this from happening by sampling
extra values and/or mutating the state. It should also be clear that this method
does not add entropy, therefore should not be used for cryptography.

Other constructors should be discouraged; e.g. all current generators have a
`new_unseeded` function; realistically no one should use this except certain
tests, where `SeedableRng::from_seed(seed) -> Self` could be used instead.

Alternatively, `new` could seed from `thread_rng` or similar, or not exist
forcing users to use `from_rng` or the `SeedableRng` trait. However, I believe
`new` should exist and seed from `OsRng`, since this makes the easiest way to
create an RNG secure and well seeded.

## Generators

This section concerns implementations of `Rng`;
[the API is discussed above](#generation-api).

In no particular order, this section attempts to cover:

*   which generators should be provided by this library
*   the story for generators in other crates
*   requirements of generators included in this library
*   benchmarks, testing and documentation on included generators
*   convenience new types and functions

### PRNG algorithms

Rand currently provides three PRNG algorithms, and a wrapper (`OsRng`) to
OS-provided numbers. It also provides a couple of simple wrappers, and two
traits, [`Rng` and `SeedableRng`](#generation-api). The included PRNGs are:

*   [`IsaacRng`] (32-bit) and [`Isaac64Rng`]; a very fast cryptographic
    generator, but with potential attacks on weak states
*   [`ChaChaRng`]; a cryptographic generator used by Linux and by Google for TLS,
    among other uses
*   [`XorShiftRng`]; a very fast generator, generally inferior to Xoroshiro

**Question:** should any of the above be removed? What other PRNGs should we
consider adding — should we keep the crate minimal or add several good
generators? (Note that the questions of whether `rand` should be split into
multiple crates and whether a separate crypto trait should be added affects
this question.) [bhickey has some thoughts on this.](https://github.com/rust-lang-nursery/rand/pull/161#issuecomment-320483055).

*   Likely [`XorShiftRng`] should be removed, since `Xoroshiro` is generally superior.
*   Should we add [`Xoroshiro128+`] as a replacement for XorShift?
    ([Wikipedia article](https://en.wikipedia.org/wiki/Xoroshiro128%2B))
*   Should we add implementations for other promising crypto-PRNGs, e.g.
    Fortuna/Yarrow (apparently used by *BSD, OSX, and iOS)?
*   Should we add an implementation of [`RDRAND`]? This is supposed to be
    secure and fast, but not everyone trusts closed-source hardware, and it may
    not be the fastest. If we do, how should we handle platforms without this
    feature?
*   Wikipedia [mentions an improved `ISAAC+` variant](https://en.wikipedia.org/wiki/ISAAC_(cipher)#Cryptanalysis) of ISAAC; what is this?
*   The [eSTREAM project](http://www.ecrypt.eu.org/stream/)
    ([Wiki article](https://en.wikipedia.org/wiki/ESTREAM)) selected several
    new stream cipher algorithms; these should all be usable as crypto PRNGs.
*   If the rand crate is split somehow or a "rand_extra" crate added, should
    this accept good quality implementations of any known PRNG?

There are a couple of wrapper/renamed implementations:

*   [`IsaacWordRng`] is [`Isaac64Rng`] on 64-bit pointer machines, [`IsaacRng`] on
    32-bit pointer machines [other name suggestions welcome]
*   [`StdRng`] is currently a wrapper around [`IsaacWordRng`], with a `new()`
    method that seeds from [`OsRng`]. Ideally this `new` behaviour should be
    moved to the PRNG itself (see [creation-of-rngs]); in this case `SndRng`
    could just be a new type name, not a wrapper

**Question:** should we rename `StdRng` to `FastRng` or `CryptoRng`, or perhaps
have `CryptoRng` for the ChaCha generator and `FastRng` for either the
Xoroshift or ISAAC generators? My understanding is that ISAAC is faster than ChaCha
and mostly secure, but has some weak states, and is therefore effectively a
compromise between a speedy generator like `XorShift` and a strong cryptographic
generator. Several people have suggested that even simulations not requiring
cryptographic generators should be using them for their stronger statistical
indepedence guarantees. This does not imply that non-cryptographic PRNGs have
no good uses (e.g. games usually do not require good quality generators).

### Special `Rng` implementations

[`ConstRng`] is a special implementation yielding a given constant repeatedly.
It is sometimes useful for testing.

**Question:** are there any good reasons *not* to include [`ConstRng`]? Or
perhaps `rand` should also include a looping-sequence generator or a counting
generator, or a generator based on an iterator?

[`ReadRng`] is an adaptor implementing `Rng` by returning data from any source
supporting `Read`, e.g. files. It is used by `OsRng` to read from
`/dev/urandom`.

[`ReseedingRng`] is a wrapper which counts the number of bytes returned, and
after some threshold reseeds the enclosed generator. Its only real use would be
to periodically adjust a random number stream to recover entropy after loss
(e.g. a process fork) or where there was insufficient entropy to begin with
(seeding from the OS generator too soon after boot — but probably all OSs
deal with this problem anyway; e.g. Linux saves a "seed file", and Intel's
`RDRAND` instruction can be used as an extra source of entropy, even if not
trusted).

Note that if an "entropy injection" trait can be added, we should use
that instead of reseeding from scratch.

### OS provision

`OsRng` currently implements `Rng` by directly sampling from whatever OS
functionality is available.
The `OsRng` struct is useful in that it separates initialisation from generation
and that it stores any state required; this is not however as important as it
might appear.

Initialisation is trivial on most platforms; the exceptions are:

*   Linux: implementation tests whether the `getrandom()` system call is
    available by calling it, once, using synchronisation primitives;
    on failure the implementation tries to construct a reader on `/dev/urandom`;
    the latter can in theory fail but is present since Linux 1.3.30.
*   Redox constructs a reader on `rand:`; in theory this can fail
*   NaCl queries an interface, then either explicitly returns an `Err` or, on
    success, asserts that it has a function pointer then succeeds

After initialisation, panics are possible if:

*   (Linux) getrandom returns an unexpected error
*   (Linux via urandom, Redox): file read fails
*   (IOS, FreeBSD, OpenBSD, Fuchsia, Windows, NaCl): system call returns an
    error

It may be worth investigating which of these panics could conceivably happen,
and add appropriate testing during initialisation.

On the other hand, since the primary use of `OsRng` is to seed another RNG
(single use), and since all possible platforms can in theory cause an error
after initialisation, it might be preferable to replace `OsRng` with a simple
`try_fill_bytes` function. This would entail doing all synchronisation on first
use (via a simple synchronisation primitive or thread-local memory), and
adapting each `Rng`'s `fn new() -> Self` function.

Contrary to the above, an implementation of `Rng` using only the OS may be
exactly what some users want, since this can be used just like any other `Rng`,
aside from the lower performance; **probably [`OsRng`] will stay as-is**.

### Convenience functions

`thread_rng` is a function returning a reference to an automatically
initialised thread-local generator.

In the current `rand` crate, [`thread_rng`](https://docs.rs/rand/0.3.16/rand/fn.thread_rng.html) constructs a reference-counted
periodically-reseeding [`StdRng`] (ISAAC) per thread on first use. This is
"reasonably fast" and "reasonably secure", which can be viewed either as a good
compromise or as combining the worst aspects of both options — is this a good
default generator?

[@zackw points out that the default random number generator should be secure](https://internals.rust-lang.org/t/crate-evaluation-for-2017-07-25-rand/5505/68):

> I think it’s vital that the *default* primitive random number generator, the
> one you get if you don’t do anything special, is an automatically-seeded
> CSPRNG. This is because people will reach for the equivalent of C’s
> [rand(3)](http://man7.org/linux/man-pages/man3/rand.3.html) in situations
> where they *need* a CSPRNG but don’t realize it, such as generation of HTTP
> session cookies. Yes, there are *better* ways to generate HTTP session
> cookies, but if the rand-equivalent is an auto-seeded CSPRNG, the low bar
> goes from “catastrophically insecure” to “nontrivial to exploit,” and that’s
> valuable.

One school of thought is that the default generator should be [`OsRng`], thus
aiming for maximal security and letting users deal with performance if and only
if that is a problem. In light of this, the strawman revision uses [`OsRng`] as
the default, but allowing the generator used by [`thread_rng`] to be replaced
on a per-thread basis:

*   [`thread_rng`] returns a reference to a boxed `Rng` (using dynamic dispatch)
*   [`set_thread_rng`] replaces the `Rng` used by the current thread
*   [`set_new_thread_rng`] changes how new thread-RNGs are created

Note that the ability to override the generator used by `thread_rng` has certain
uses in testing, limited uses in improving performance (for ultimate
performance and for reproducibility, and may be preferable to avoid using
`thread_rng` at all), and has security implications, in that libraries cannot
rely on `thread_rng` being secure due to binaries and other libraries having
the option to replace the generator. It may therefore be better not to allow
override and possibly to use a "fast/secure compromise" PRNG like the current
`rand` crate.

Another school of thought is that [`thread_rng` and `random` etc. should not be
included at all](https://internals.rust-lang.org/t/crate-evaluation-for-2017-07-25-rand/5505/82).

In the current `rand` crate, a second convenience generator is available:
[`weak_rng`] constructs a new `XorShiftRng` seeded via `OsRng` each
time it is called. (The `SomeRng::new()` syntax can replace the need for
this type of function; see [creation-of-rngs].) The strawman revision simply
removes `weak_rng`.

Two functions using [`thread_rng`] are included:

*   [`random`], generating random values via the default distribution
*   [`random_with`], generating random values via a specified distribution

### Testing generators

Since most of the generators discussed are *deterministic*, this determinism
should be tested. Each generator should be required to have at least one test
setting a specific seed, then reproducing a sequence of values obtained either
from a specification of the PRNG algorithm or generated with a reference
implementation.

The ChaCha and ISAAC generators already have such tests
(`test_rng*_true_values`), however the ISAAC variant does not document the
source. The XorShift generator currently has no such test.

### Benchmarking generators

It would be nice if the crate included the following information on each
generator in documentation and/or benchmarks:

*   state size in bytes
*   initialisation time
*   amortised time to generate 1 word of randomness (where "word" means the
    native size of the generator, not of the machine)

Currently there are benchmarks of generation time, but this *might* not truely
represent the amortised time due to infrequent update cycles.

### Notes

Should we worry about generator algorithms not *exactly* matching the domain
(`u32` or `u64`)? For example, Xoroshiro apparently never generates zero.

## Random values

This section concerns creating random values of various types and with various
distributions given a generator (`Rng`).

Most of this functionality is contained in the [`distributions`] module.
(For a time this module was renamed `dist` for brevity, but renamed back to
avoid confusion. `distr` might be another possibility.)

The strawman revision showcases two traits for generating random values of the
current type, the [`Rand`] trait and [`SimpleRand`]. It is the intention to only
keep one of these, and name whichever remains `Rand`. The first, (currently
named) [`Rand`], supports parameterisation by a distribution, thus giving
explicit control over how values are generated. The second, [`SimpleRand`] lacks this
parameterisation, making simple usage simpler but requiring usage of
distributions directly for other cases.

Both "Rand" traits work in concert with the [`Distribution`] trait; more on that
below. For these examples we'll use two implementations: the "best-for-the-type"
[`Default`] distribution and the [`Range`] distribution.

Now to some [`Rand`] examples:

```rust
use rand::distributions::{Rand, Default, Range};
let mut rng = rand::thread_rng();

// Type annotation needed; two options:
let byte: u8 = Rand::rand(&mut rng, Default);
let byte = u8::rand(&mut rng, Default);

// For ranges, the generated type is the same as the parameter type:
let ranged = Rand::rand(&mut rng, Range::new(-99, 100));
```

And some [`SimpleRand`] examples:

```rust
use rand::distributions::{SimpleRand, Distribution, Range};
let mut rng = rand::thread_rng();

// Again, type annotation is needed; two options:
let byte: u8 = SimpleRand::simple_rand(&mut rng);
let byte = u8::simple_rand(&mut rng);

// SimpleRand does not support other distributions, so we have to use the
// distribution directly:
let ranged = Range::new(-99, 100).sample(&mut rng);
```

Note that the `Default` distribution also supports direct sampling, so we don't
need *either* version of `Rand`:

```
use rand::distributions::{Distribution, Default};
let mut rng = rand::thread_rng();

let byte: u8 = Default.sample(&mut rng);
```

#### Pass by copy?

Currently [`Rand::rand`] takes the distribution parameter by value. This is the
best option for zero-size distribution types like [`Default`] and [`Open01`], since
it allows call syntax like `Rand::rand(&mut rng, Default)` (second parameter
does not need to be referenced).

Most distribution types are fairly small, e.g. `Range` is two or three values
of the parameterised type, so for the most part pass-by-value is reasonable,
although for example `Gamma` is 40 bytes. Can the compiler optimise this?

On the other hand, `Distribution::sample` takes itself by reference. This is
required for the special `Weighted` distribution, which does not support `Copy`.
Does this add overhead? Note that currently `rand` is implemented using
`sample`, which in some ways is the worst of both worlds. Should distributions
be required to support `Copy` or, at least, should `sample` take `self` by
value?

### Distributions

The [`Distribution`] trait replaces `rand`'s current [`IndependentSample`]
trait. The `Sample` trait is removed; I believe it was originally intended for use
in random processes like random walks; these are discrete-time (stochastic)
models, thus `advance_state()` and `get_state()` functions are more applicable
than `sample()`; in any case this is beyond the scope of `rand`.

The surviving trait is quite simple:

```rust
/// Types (distributions) that can be used to create a random instance of `T`.
pub trait Distribution<T> {
    /// Generate a random value of `T`, using `rng` as the
    /// source of randomness.
    fn sample<R: Rng+?Sized>(&self, rng: &mut R) -> T;
}
```

This could be extended with other functions such as
`fn map<F: Fn(T) -> T>(&self, f: F) -> MappedDistribution<T, F>`, but I do not
see a good rationale.

Any implementation, such as [`Default`], supports usage via `sample`:
`Default.sample(&mut rng)`. (Note that `struct Default;` is valueless; Rust
allows objects to be created without any extra syntax: `let x = Default;`.)

Several zero-size structs implementing [`Distribution`] specify simple distributions:

*   [`Uniform`] specifies uniform distribution over the entire range available, and
    is implemented for all integer types and `bool`
*   [`Uniform01`] specifies uniform distribution over the half-open range `[0, 1)`,
    and is implemented for `f32` and `f64`
*   [`Closed01`] is like [`Uniform01`] but for `[0, 1]` (thus including 1.0)
*   [`Open01`] is like [`Uniform01`] but for `(0, 1)` (thus excluding 0.0)
*   [`Default`] uses [`Uniform`] or [`Uniform01`] depending on type (and can be
    extended for other types)

[`Default`] has roughly the same capabilities as the
[old `Rand`](https://docs.rs/rand/0.3.15/rand/trait.Rand.html); currently it doesn't
support arrays, tuples, `Option`, etc., but it could conceivably, and probably
also `derive(Rand<Default>)` or something similar.

It should be noted that there is no agreement on using the name `Default`. In
particular, there is a naming conflict with `std::default::Default`, which can
lead to surprising compiler messages if the user forgets to
`use rand::Default;`. Similarly, `Uniform` and `Uniform01` are open to
adjustment. All three could be replaced with a single `Uniform`; this just
leaves two semantic issues: the range differs by type, and some possible
type-dependent implementations (such as for `Option`) cannot practically have
uniform distribution.

#### Range

There is one further uniform distribution:

*   [`Range`] specifies uniform distribution over a range `[a, b)` and supports
    integer and floating-point types

This [`Range`] is minimally changed from the current `rand`, and supports
extension to user-defined types by exposing its internal fields. An alternative
implementation, [`range2`], has been written in an attempt to improve extension
to other types and avoid the need for an unused `zone` field with float types,
but has some drawbacks, perhaps most notably that `Range` is parameterised so
that `Range::new(low, high)` must be replaced with `new_range(low, high)` or
`Range::<T>::new(low, high)`.

Possibly the current `range` function should be removed, then `new_range` from
[`range2`] and an equivalent for [`Range`] could be named `range`.

#### Non-uniform distributions

Finally, there are several [`distributions`]
unchanged from the current `rand`:

*   `Exp`
*   `Normal`, `LogNormal`
*   `Gamma`, `ChiSquared`, `FisherF`, `StudentT`

Currently these are only implemented for `f64`. They could be extended to `f32`
but this might require adding some more lookup tables to the crate.

Internally, `Exp(1)` and `N(0, 1)` (standard normal) fixed distributions are
used; these could be exposed via new zero-size distribution structs.
This might be slightly faster for some uses (avoid a multiplication and extra
data access).

Most distributions are implemented in public sub-modules, then *also* imported
into `distributions` via `pub use`. Possibly the sub-modules should be hidden.

#### Conversion to floating point

Currently this is implemented via `impl Distribution<f32> for Uniform01` and
the `f64` equivalent in the strawman revision, and within the `Rng` trait in
the current `rand`. It has been suggested that this should be implemented in
a simple function (used by `Rand` / `Uniform01`) so that users only wanting to
use a small subset of the library for cryptography do not need to use the
distributions code. This is only really useful if the `rand` crate is split
into a minimal crypto sub-set and the rest building on that.

The following article points out that the common method of generating floats in the
range `[0, 1)` or `(0, 1)` is wrong. It is worth pointing out that our existing
code *does not use this method*, however it may still be worth reading the
article: [Generating Pseudo-random Floating-Point
Values](https://readings.owlfolio.org/2007/generating-pseudorandom-floating-point-values/).

[More on the topic here](http://xoroshiro.di.unimi.it/), under the heading
"Generating uniform doubles ...".

#### Testing distributions

Distributions should test exact output with several specified inputs, via usage
of [`ConstRng`] or similar. (TODO: implement such tests.)

### `Rand` vs `Distribution`

As suggested above, both `Rand` traits are basically wrappers around
`Distribution`:

```rust
impl<T, D: Distribution<T>> Rand<D> for T {
    fn rand<R: Rng+?Sized>(rng: &mut R, distr: D) -> Self {
        distr.sample(rng)
    }
}

impl<T> SimpleRand for T where Default: Distribution<T> {
    fn simple_rand<R: Rng+?Sized>(rng: &mut R) -> Self {
        Default.sample(rng)
    }
}
```

This implies that the `Rand` traits could be removed altogether without any
loss of functionality. Alternatively, we could remove the `Distribution` trait,
keep the distributions (`Default`, `Range`, etc.), and implement `Rand`
directly:

```rust
impl<u32> Rand<Uniform> for u32 {
    fn rand<R: Rng+?Sized>(rng: &mut R, _distr: Uniform) -> Self {
        rng.next_u32()
    }
}
```

For the user, this leaves a choice between:

```rust
// simple Rand (SimpleRand in this document):
use rand::Rand;
let x = i64::rand(rng);

// complex Rand:
use rand::Rand;
let y = i64::rand(rng, Default);

// no Rand:
use rand::Distribution;
let z: i64 = Default.sample(rng);

// in all cases, we can still have:
let a: i64 = rand::random();
```

### Convenience functions and more distributions

The above examples all get randomness from [`thread_rng`]. For this case, two
convenience functions are available:

*   [`random`], essentially `fn random() { Default.sample(&mut thread_rng()) }`
*   [`random_with`], essentially
    `fn random_with<D: Distribution>(distr: D) { distr.sample(&mut thread_rng()) }`

These do not require a [`Rand`] trait. Since calling [`thread_rng`] has a little
overhead, these functions are slightly inefficient when called multiple times.

Additionally, within the `distributions` module, some more convenience functions
are available:

*   `uniform(rng) -> T`, equivalent to `Rand::rand(rng, Uniform)`
*   `range(low, high, rng) -> T`, equivalent to `Rand::rand(rng, Range::new(low, high))`

It is debatable whether these are worth keeping and possibly extending to include
`uniform01(rng) -> T` etc. They are convenient when used with iterators (see below).

A couple more distributions are available using functions of the same form,
but (currently) without a `Distribution` implemention representing them:

*   [`codepoint`] `(rng) -> char` generating values uniformly distributed over all valid
    Unicode codepoints, even though many are unassigned. This may be useless
    but is the most obvious implementation of `Distribution<char>` for
    [`Uniform`] and [`Default`].
*   [`ascii_word_char`] `(rng) -> char` uniformly selects from `A-Z`, `a-z` and
    `0-9`, thus is convenient for producing basic random "words" (see usage
    with iterators below).

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

### `rand_derive`

The current `rand` has a sub-crate, [`rand_derive`]
([source code](https://github.com/rust-lang-nursery/rand/tree/master/rand-derive)).
Probably something similar could be designed for [`Default`] or `Rand<Default>`,
but due to the current author's lack of interest this has not been investigated.

### Testing distributions

Each distribution should have a test feeding in a known sequence of "generated"
numbers, then test the exact output for each input, where the output may be
generated by the library itself but must be tested to be at least approximately
correct via external calculation.

A generator returning a sequence of evenly spaced `u64` values should be
sufficient; output should include zero, `u64::MAX`, and
several values in between.

### Benchmarking distributions

Most distributions currently only apply to `f64`; for this type a baseline
sampling from `[0, 1)` and each distribution available should be tested,
each generating the same number of values.

Benchmarks for other types could be added.

## `no_std` support

This was requested, and has been implemented in the refactor by hiding many
parts of rand behind `#[cfg(feature="std")]`. Without `std` quite a few features
are removed, including `thread_rng()`, `random()`, and the entire `os` module
which normally provides entropy for initialisation.

API doc can be generated via `cargo doc --no-default-features`, but is not
currently hosted (TODO), and `no_std` support is not automatically tested
(TODO; use `cargo build --no-default-features` for now).

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

@Lokathor proposed an alternative design for distributions: make each a trait:

```rust
pub trait Rand: Sized {
    fn rand<R: Rng+?Sized>(rng: &mut R) -> Self;
}

pub trait RangedRand: Rand + PartialOrd {
    fn rand_range<R: Rng+?Sized>(rng: &mut R, low: Self, high: Self) -> Self;
}
```

This however lacks a way of separating parameterisation of the distribution
from sampling; for example `Range` does some calculations in `Range::new`,
creates a simple object with two or three values, then does minimal work during
sampling (`my_range.sample(&mut rng)`).

# Unresolved questions
[unresolved]: #unresolved-questions

## Summary of above

There are many questions above; perhaps the biggest ones are as follows.

API:

*   Remove `next_f32` and `next_f64` from `Rng`?
*   Add `next_u128` to `Rng`?
*   Add a `CryptoRng` trait or otherwise split out cryptographic generators?
*   Split the `rand` crate?
*   Add `JumpableRng` trait?
*   Add `InjectableRng` trait?
*   Which constructors should `Rng` impls have?

Generators:

*   Which PRNGs `rand` contain?
*   Which testing `Rng` impls, if any, should `rand` contain?
*   Replace `OsRng` with an `os::try_fill_bytes` function?
*   Allow generator behind `thread_rng` to be switched?
*   Include `StdRng` and `weak_rng` wrappers?

Distributions:

*   Rename the `distributions` module?
*   Rename `Uniform`, `Default`, etc.?
*   Keep the parameterised `Rand<Distr>`, replace with the simple `Rand` or
    remove both?
*   Replace [`range`] with [`range2`] ?

## Derive rand

The `derive_rand` sub-crate of the current `rand` provides another method to
generate random values of the current type. This could probably be adjusted to
derive `Rand<Default>` or maybe even support custom distributions. In the
strawman revision I simply deleted this sub-crate since I have no personal
interest in creating random values this way.

-------------------------------------------------------------------------------

[Crate evaluation thread]: https://internals.rust-lang.org/t/crate-evaluation-for-2017-07-25-rand/5505
[Strawman revision PR]: https://github.com/rust-lang-nursery/rand/pull/161
[Strawman revision code]: https://github.com/dhardy/rand
[Strawman revision doc]: https://dhardy.github.io/rand/rand/index.html
[`Rand`]: https://dhardy.github.io/rand/rand/distributions/trait.Rand.html
[`SimpleRand`]: https://dhardy.github.io/rand/rand/distributions/trait.SimpleRand.html
[`Distribution`]: https://dhardy.github.io/rand/rand/distributions/trait.Distribution.html
[`Default`]: https://dhardy.github.io/rand/rand/struct.Default.html
[`Uniform`]: https://dhardy.github.io/rand/rand/distributions/struct.Uniform.html
[`Uniform01`]: https://dhardy.github.io/rand/rand/distributions/struct.Uniform01.html
[`Open01`]: https://dhardy.github.io/rand/rand/distributions/struct.Open01.html
[`Closed01`]: https://dhardy.github.io/rand/rand/distributions/struct.Closed01.html
[`Range`]: https://dhardy.github.io/rand/rand/distributions/range/struct.Range.html
[`Rand::rand`]: https://dhardy.github.io/rand/rand/distributions/trait.Rand.html#tymethod.rand
[`random`]: https://dhardy.github.io/rand/rand/fn.random.html
[`random_with`]: https://dhardy.github.io/rand/rand/fn.random_with.html
[`IndependentSample`]: https://docs.rs/rand/0.3.16/rand/distributions/trait.IndependentSample.html
[`rand_derive`]: https://docs.rs/rand_derive/0.3.0/rand_derive/
[`codepoint`]: https://dhardy.github.io/rand/rand/distributions/fn.codepoint.html
[`ascii_word_char`]: https://dhardy.github.io/rand/rand/distributions/fn.ascii_word_char.html
[`range`]: https://dhardy.github.io/rand/rand/distributions/range/index.html
[`range2`]: https://dhardy.github.io/rand/rand/distributions/range2/index.html
[`distributions`]: (https://dhardy.github.io/rand/rand/distributions/index.html)
[`ReadRng`]: https://dhardy.github.io/rand/rand/struct.ReadRng.html
[`ReseedingRng`]: https://dhardy.github.io/rand/rand/reseeding/struct.ReseedingRng.html
[`IsaacRng`]: https://dhardy.github.io/rand/rand/prng/struct.IsaacRng.html
[`Isaac64Rng`]: https://dhardy.github.io/rand/rand/prng/struct.Isaac64Rng.html
[`IsaacWordRng`]: https://dhardy.github.io/rand/rand/prng/struct.IsaacWordRng.html
[`ChaChaRng`]: https://dhardy.github.io/rand/rand/prng/struct.ChaChaRng.html
[`XorShiftRng`]: https://dhardy.github.io/rand/rand/prng/struct.XorShiftRng.html
[`Xoroshiro128+`]: http://xoroshiro.di.unimi.it/
[`StdRng`]: https://dhardy.github.io/rand/rand/struct.StdRng.html
[`RDRAND`]: https://en.wikipedia.org/wiki/RdRand
[`ConstRng`]: https://dhardy.github.io/rand/rand/struct.ConstRng.html
[`thread_rng`]: https://dhardy.github.io/rand/rand/fn.thread_rng.html
[`set_thread_rng`]: https://dhardy.github.io/rand/rand/fn.set_thread_rng.html
[`set_new_thread_rng`]: https://dhardy.github.io/rand/rand/fn.set_new_thread_rng.html
[`OsRng`]: https://dhardy.github.io/rand/rand/struct.OsRng.html
