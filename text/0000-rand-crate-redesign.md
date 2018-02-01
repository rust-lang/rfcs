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

* [RFC comments](https://github.com/rust-lang/rfcs/pull/2106)
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

What we should provide is clear guidance (with the usual disclaimers) on choice
of RNG for at least the following cases:

1.  Cryptography should normally be handled by a crypto-library, but where
    entropy is needed, users should prefer to use the OS generator directly
    where possible. Where a user-space RNG is needed, ChaCha may be the best
    choice.
2.  Where fast random numbers are needed which should still be cryptographically
    secure, but where speed is more important than absolute security (e.g. to
    initialise hashes in the std library), a generator like ISAAC+ (?) should be
    used; this is exposed via `thread_rng()`.
3.  Where a fast and uniform generator is wanted for numeric applications, a
    generator like PCG (?) should be preferred.
4.  Where determistic operation / reproducibility is desired, any PRNG algorithm
    may be used, but the implementation should be selected by specific name and
    not a generic name (`StdRng` or `IsaacWordRng`) and be seeded manually.

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

### The `Rng` trait

The [current `Rng` trait](https://docs.rs/rand/0.3.16/rand/trait.Rng.html) is
quite large and complex:

```rust
pub trait Rng {
    fn next_u32(&mut self) -> u32;

    fn next_u64(&mut self) -> u64 { ... }
    fn next_f32(&mut self) -> f32 { ... }
    fn next_f64(&mut self) -> f64 { ... }
    fn fill_bytes(&mut self, dest: &mut [u8]) { ... }
    
    fn gen<T: Rand>(&mut self) -> T
        where Self: Sized { ... }
    fn gen_iter<'a, T: Rand>(&'a mut self) -> Generator<'a, T, Self>
        where Self: Sized { ... }
    fn gen_range<T: PartialOrd + SampleRange>(&mut self, low: T, high: T) -> T
        where Self: Sized { ... }
    fn gen_weighted_bool(&mut self, n: u32) -> bool
        where Self: Sized { ... }
    fn gen_ascii_chars<'a>(&'a mut self) -> AsciiGenerator<'a, Self>
        where Self: Sized { ... }
    fn choose<'a, T>(&mut self, values: &'a [T]) -> Option<&'a T>
        where Self: Sized { ... }
    fn choose_mut<'a, T>(&mut self, values: &'a mut [T]) -> Option<&'a mut T>
        where Self: Sized { ... }
    fn shuffle<T>(&mut self, values: &mut [T])
        where Self: Sized { ... }
}
```

#### Desired properties

The above trait is **long and complex**, and does not cleanly separate core
functionality (`next_*` / `fill_bytes`) from derived functionality (`gen`,
`choose`, etc.). This design pattern is successfully used elsewhere, e.g. by
[`Iterator`](https://doc.rust-lang.org/std/iter/trait.Iterator.html), but is not
ideal, especially for cryptographic applications where clear, easily-reviewable
code is of huge importance.

On the other hand, **ease of use** is also important. Simplifying the `Rng`
trait does not, however, imply that usage must be impaired; see the [`Sample`]
trait in the next section for more on this topic.

**Determinism** is important for many use-cases, including scientific simulations
where it enables third parties to reproduce results, games wishing to reproduce
random creations from a given seed, and cryptography. To quote Joshua
Liebow-Feeser
[@joshlf](https://github.com/rust-lang/rfcs/pull/2106#issuecomment-323546147):

> CSPRNGs are also used deterministically in cryptographic applications. For
> example, stream ciphers use CSPRNGs to allow multiple parties to all compute
> the same pseudorandom stream based on a secret shared seed. Determinism is
> very important for a CSPRNG even if it isn't used in all applications.

**Performance** can be quite important for many uses of RNGs. This is not
given top priority, but some applications require *many* random numbers, and
performance is in many cases the main reason to use a user-space RNG instead of
system calls to access OS-provided randomness. The design therefore considers
performance an important goal for most functionality, although system calls to
access OS randomness are assumed to be relatively slow regardless.

Algorithmic random number generators tend to be infallible, but external
sources of random numbers may not be, for example some operating-system
generators will fail early in the boot process, and hardware generators can
fail (e.g. if the hardware device is removed). These **failures** can only be
handled via `panic` with the current `Rng` trait, but an interface exposing this
possibility of error may be desirable (on the other hand, wrapping all return
values with `Result` is undesirable both for performance and style reasons).

#### Proposed `Rng` trait

```
// (Ignore CryptoRng base trait for now; see next section.)
trait Rng: CryptoRng {
    fn next_u32(&mut self) -> u32;
    fn next_u64(&mut self) -> u64;
    #[cfg(feature = "i128_support")]
    fn next_u128(&mut self) -> u128;
    
    fn fill_bytes(&mut self, dest: &mut [u8]);
}
```

All derived (convenience) methods have been removed from this trait; this
functionality is moved to the [`Sample`] trait or elsewhere.

Further, the `next_f32` and `next_f64` functions have been removed.
Although [direct generation of floating-point random values is
possible](http://www.math.sci.hiroshima-u.ac.jp/~m-mat/MT/SFMT/#dSFMT), I do
not believe this is often done in practice (such a generator would not be able
to directly produce random values of integer types efficiently, and would likely
have little performance advantage).

Finally, `next_u128` has been added. [Native 128-bit generators already
exist](http://www.math.sci.hiroshima-u.ac.jp/~m-mat/MT/SFMT/). This may provide
some performance benefit to applications requiring many pairs of 64-bit values.

It has been suggested that `next_u8`, `next_u16` also be added. However, most
fast generators operate on 32-bit or 64-bit integers natively, and attempts to
extract less bits usually only impair performance. This RFC therefore does not
add these methods, but notes that they could be added in the future without any
breakage (using default implementations on `next_u32`).

In order to assure cross-platform reproducibility of results from deterministic
generators, it is important that *endianness* is specified when converting between
numbers of different sizes, especially to/from `u8` types. This RFC specifies
that all conversions should be *little endian*, including `u32` → `u64` (least
significant first) and `u64` → `u32` (use least significant part).

Default implementations of most of these functions in terms of `next_u32` and
`next_u64` could be added; in my personal opinion there should be no default
methods since it is not uncommon (at least within the `rand` crate) to write
wrapper types around a generator re-implementing the `Rng` trait; if default
implementations exist then a wrapper implementation may miss some functions
and "work", but behave incorrectly (different random number stream, worse
performance). Instead, example code can be shown for `next_*` impls (e.g.
`self.next_u64() as u32` and
`(self.next_u32() as u64) | ((self.next_u32() as u64) << 32)`), and a function
to implement `fill_bytes`:
`fn fill_bytes_from_u64<R: Rng>(rng: &mut R, dest: &mut [u8])`.

#### Proposed `CryptoRng` trait

For cryptographic purposes, it has been suggested that `fill_bytes` alone would be
sufficient. Further, the function should return a `Result`, to properly
accomodate external randomness sources which can fail, such as direct OS system
calls or file reads.

The following trait is proposed, intended to be as simple as possible. It is
likely that details of any failure will not be useful, hence the empty
`CryptoError` struct (arguments to the contrary welcome).

```rust
use std::result::Result;

pub struct CryptoError;

pub trait CryptoRng {
    fn try_fill(&mut self, dest: &mut [u8]) -> Result<(), CryptoError>;
}
```

Note: methods like `fn try_next_u32(&mut self) -> Result<u32, CryptoError>` can
be added if desired, but probably it's preferable to keep this trait minimal.
Function names must differ from those in the `Rng` trait (reason a little later).

Since the trait `Rng` extends `CryptoRng`, we implicitly implement `CryptoRng`
for every `Rng` with the following code (which means that `Rng` implementations
do not have to worry about `CryptoRng`, and every `Rng` implementation is
automatically available as a `CryptoRng` implementation too):

```rust
impl<R: Rng+?Sized> CryptoRng for R {
    fn try_fill(&mut self, dest: &mut [u8]) -> Result<(), CryptoError> {
        Ok(self.fill_bytes(dest))
    }
}
```

Since users may wish to use `CryptoRng` generators in code expecting an `Rng`
trait implementation, we also provide two wrapper functions to create an `Rng`
from a `CryptoRng`:

```rust
/// Consume any CryptoRng, returning a type implementing `Rng`:
fn as_rng<CR: CryptoRng>(rng: CR) -> AsRng<CR>;
/// Given any `&mut crng` where `crng: CryptoRng`, return a type implementing `Rng`:
fn as_rng_ref<'a, CR: 'a+CryptoRng+?Sized>(rng: &'a mut CR) -> AsRngRef<'a, CR>;
```

(Hopefully there will be little need to use the above `as_rng*` functions in practice.
There are three potential issues: (1) cryptographic generators may not be
uniform, (2) cryptographic generators may be relatively slow, (3) there is an
unlikely possibility of a panic. Because of this, code expecting to be used with
a `CryptoRng`, such as an RNG constructor seeding from another RNG, should use
randomness from a `CryptoRng` not an `Rng`.)

#### Alternative `Rng` / `CryptoRng` designs

Several alternative designs for `Rng` and `CryptoRng` are possible.

We could have a single trait. In such a case, there is a strong desire to keep
`next_u32` and `next_u64` methods returning an integer directly: any `Result`
would for practical reasons probably get `unwrap()`-ed since e.g. propegating
`Result` errors through all random-distribution code when this code would
normally be used with infallible generators anyway makes no sense. `fill_bytes`
could be modified to return a `Result`, but the result would be a hybrid monster
where fallible generators would have to implement `next_u32` etc. methods with
`unwrap` / `panic`. Further, there has been a call to have a cryptographic
generator trait with minimal complexity, as [for example in the `ring` crate](https://briansmith.org/rustdoc/ring/rand/trait.SecureRandom.html).

Using two traits, `Rng` and `CryptoRng`, they could have a different
relationship. We could have `CryptoRng: Rng` but in practice this makes no
sense (any `CryptoRng` must implement all `Rng` functions using `unwrap`, and
this doesn't meet the desire for a minimal-complexity `CryptoRng`). We could
use two unrelated traits (no trait extension), but this doesn't seem to offer
any advantages.

We could even use a [base `RawRng<E>` trait](https://github.com/dhardy/rand_design/blob/master/traits/raw_rng.rs) where `E` is either
`CryptoError` or `!` and all functions return `Result<T, E>`; this has been
demonstrated [not to reduce performance](https://github.com/rust-lang/rfcs/pull/2106#issuecomment-323769203) and adds a "super trait", but since
`RawRng` is not object-safe I see little advantage. There is a long discussion
around this design [starting here](https://github.com/rust-lang/rfcs/pull/2106#issuecomment-323388931).

For an overview of various two-trait solutions, [see this repository](https://github.com/dhardy/rand_design/tree/master/traits).
[Note: these use functions `next_u32` and `try_next_u32`; since we don't need
`try_next_u32` imagine the examples use `fill_bytes` and `try_fill` instead.]

One point of note is [@Lokathor's suggestion](https://github.com/rust-lang/rfcs/pull/2106#issuecomment-325604852)
to use type-safety to prevent insecure generators being used where a secure
generator is required. *Personally* I do not see `CryptoRng` as a
*promise of secure random numbers*, but as a way of handling
potentially-fallible generators; for example `OsRng` and the `RDRAND`
instruction are potentially fallible but whether they are secure is another
question (OS design issues and limitations of embedded systems may make
`OsRng` less secure and most OSes have decided not to trust `RDRAND` fully but
instead use it only as an extra source of entropy). Of course, there is still
something to the type-safety idea: it *might* prevent use of very insecure
generators in some situations where security matters.

Of all the above, the most reasonable three options are in my opinion:

1.  Use the design above where `Rng` extends `CryptoRng`
2.  Use two unrelated traits, requiring explicit wrapping to convert objects of
    either type to the other ([like this](https://github.com/dhardy/rand_design/blob/master/traits/separate_explicit_Rng.rs))
3.  Use a single trait and accept that fallible generators may panic

#### Bikeshedding names

The names `Rng` and `CryptoRng` could be adjusted. Both types could uses the
same name in different namespaces. But, in my personal opinion, the guidance in
[RFC #356](https://github.com/rust-lang/rfcs/pull/356) should be taken with a
pinch of salt: names of items commonly used or discussed together should be
distinct to avoid the need for path prefixes. Brian Smith advocates using the
name `Generator` since the "R" in "RNG" is redundant with the crate name and
the "N" may be inappropriate (not all results are numbers), however there are a
couple of reasons to use RNG even so: (1) *RNG* is a well known and easy term to
search for, (2) "generator" is long, especially for use in suffixes like `OsRng`
or `OsGenerator`, (3) *generator* is highly inspecific.

The functions `as_rng` and `as_rng_ref`, and their return types, could be
renamed. (Note that two separate functions and return types are needed for
technical reasons unless an `impl Rng for &mut CryptoRng` rule is added;
compare [extends_CryptoRng2](https://github.com/dhardy/rand_design/blob/master/traits/extends_CryptoRng2.rs) and [extends_CryptoRng3](https://github.com/dhardy/rand_design/blob/master/traits/extends_CryptoRng3.rs).)

The function names `fill_bytes` and `try_fill` are inconsistent; we could instead
use `try_fill_bytes`.

### Rand crates

Currently, `rand` exists as a single crate. There has been a call to expose
`CryptoRng` and `OsRng` in separate crates. It has also been suggested that
no PRNGs remain in `rand` and separate crates be used for things like
`thread_rng`. Following this, the following is suggested:

*   `rand_crypto` contains `CryptoRng` and `CryptoError` but nothing else and
    has no dependencies on other crates
*   `rand_os` contains `OsRng` implemented via `CryptoRng`, depends on
    `rand_cypto` but nothing else
*   `rand` depends on `rand_crypto` and provides `Rng`, adaptors, distributions,
    etc.
*   `rand_chacha`, `rand_isaac`, `rand_xorshift`, `rand_pcg` etc. contain
    families of generators
*   `rand_thread` provides `thread_rng` via one of the above crates

In this case, crates implementing an RNG would depend on `rand_crypto` or
`rand`. Numeric users would depend on `rand` and `rand_thread` or some other
RNG crate.

(TODO: should RNG crates depend on `rand_os` for secure initialisation, or not
handle this internally? If not, should users import both crates and do something
like `MyRng::from(OsRng::new())`, or maybe something like
`OsRng::new_seeded::<MyRng>()`?)

#### Alternatives

**Names:**
we could break with tradition and name crates with `rng` instead of `rand`.
We could use different crate name patterns/prefixes for cryptographic RNGs vs
numeric RNGs, or for RNG implementations vs traits & derived functionality.
(Note: owner of `rng` crate has offered its use.)

**Crates:**
we could use fewer crates, or even more crates (e.g. by putting the `Rng` trait
and its impl rules in a separate crate).

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

Further, serialisation could be allowed by conditionally supporting [Serde].
This should be an optional feature to avoid depending on Serde where not
needed. Serialisation has been requested
[in this PR](https://github.com/rust-lang-nursery/rand/pull/119).
For now this has not been implemented since [`serde_derive` does not support
`Wrapping`](https://github.com/serde-rs/serde/issues/1020) (and I'm too lazy to
write a workaround).

Alternatively, serialisation could be enabled via a simple custom trait instead
of depending on serde.

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
create an RNG secure and well seeded. *However*, if `OsRng` is in a separate
`rand_os` crate,  should RNG crates depend on this just to provide a `new`
function? See the *Rand crates* sub-section above.

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

### Traits governing generation

The current `rand` crate has a [`Rand`](https://docs.rs/rand/0.3.16/rand/trait.Rand.html)
trait implementable by any type which can
be "randomly constructed", with quite a few (but not exhaustive) implementations
for various Rust types, one for a standard library type (`Option`), and even
implementations for `Rng` types.

The strawman revision showcases two variations on `Rand`: a [`Rand`] trait
parameterisable by distributions and a [`SimpleRand`] trait, similar to the
current `Rand`. The intention was to keep only one of these, however my current
preference is to remove both (more on this in a moment).

Current `rand` has two traits for generating values from distributions,
[`Sample`](https://docs.rs/rand/0.3.16/rand/distributions/trait.Sample.html) and
[`IndependentSample`]. I believe the purpose of `Sample` was to allow
implementation for random processes (e.g. random walks and Lévy flight);
however such processes are usually better interacted with via `advance_state()`
and `get_state()` functions, and are in any case beyond the scope of the `rand`
crate. The strawman revision therefore removes `Sample` and renames
[`IndependentSample`] to [`Distribution`], better reflecting the trait's
purpose.

The strawman revision adds several new distributions, namely `Uniform`,
`Uniform01`, and `Default` (see below). These distributions allow creation of
random values for arbitrary types, replacing the need for a `Rand` trait. The
[`Rand`] and [`SimpleRand`] traits in the strawman revision are simply
wrappers around distributions, specifically `Default`.

Additionally, the strawman revision adds a trait called [`Sample`]. This is
simply an extension to [`Rng`], adding some convenience functions to access
functionality from the `Default` and `Range` distributions, as well as
iterators.

So the strawman revision currently has *three* convenience wrappers around
distributions: [`Rand`], [`SimpleRand`] and [`Sample`]. My feeling is that the
most convenient of these is [`Sample`] and the other two serve no real purpose
(note that implementing random value creation for user-defined types is
now handled via `impl Distribution<T> for Default`).

#### Examples

Some [`Rand`] examples:

```rust
use rand::distributions::{Rand, Default, Range};
let mut rng = rand::thread_rng();

// Type annotation needed; two options:
let byte: u8 = Rand::rand(&mut rng, Default);
let byte = u8::rand(&mut rng, Default);

// For ranges, the generated type is the same as the parameter type:
let ranged = Rand::rand(&mut rng, Range::new(-99, 100));
```

Some [`SimpleRand`] examples:

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

Some [`Sample`] examples:

```rust
use rand::distributions::{Sample, Rand, Default, Range};
let mut rng = rand::thread_rng();

// Type annotation needed:
let byte: u8 = rng.gen();

// For ranges, the generated type is the same as the parameter type:
let ranged = rng.gen_range(-99, 100);
```

Equivalent code without using any of the wrappers:

```
use rand::distributions::{Distribution, Default, Range};
let mut rng = rand::thread_rng();

let byte: u8 = Default.sample(&mut rng);

let ranged = Range::new(-99, 100).sample(&mut rng);
```

#### Pass by copy?

Currently [`Rand::rand`] and [`Sample::sample`] take the distribution parameter
by value. This is the best option for zero-size distribution types like
[`Default`] and [`Open01`], since it allows call syntax like
`Rand::rand(&mut rng, Default)` (second parameter
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

### Distributions trait

The [`Distribution`] trait replaces `rand`'s current [`IndependentSample`]
trait. It is quite simple:

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
`Default.sample(&mut rng)`. (Note that `struct Default;` is a unit type (like `()`); Rust
allows objects to be created without any extra syntax: `let x = Default;`.)

#### Simple distributions

Several zero-size structs implementing [`Distribution`] specify simple distributions:

*   [`Uniform`] specifies uniform distribution over the entire range available, and
    is implemented for all integer types and `bool`
*   [`Uniform01`] specifies uniform distribution over the half-open range `[0, 1)`,
    and is implemented for `f32` and `f64`
*   [`Closed01`] is like [`Uniform01`] but for `[0, 1]` (thus including 1.0)
*   [`Open01`] is like [`Uniform01`] but for `(0, 1)` (thus excluding 0.0)
*   [`Default`] uses [`Uniform`] or [`Uniform01`] depending on type (and can be
    extended for other types)
*  [`AsciiWordChar`] samples uniformly from the ASCII characters 0-9, A-Z and a-z

[`Default`] has roughly the same capabilities as the the current `rand` crate's
[`Rand`](https://docs.rs/rand/0.3.15/rand/trait.Rand.html); currently it doesn't
support arrays, tuples, `Option`, etc., but support for thes could conceivably
be added, and probably also an equivalent to `derive_rand`.

It should be noted that there is no agreement on using the name `Default`. In
particular, there is a naming conflict with `std::default::Default`, which can
lead to surprising compiler messages if the user forgets to
`use rand::Default;`.
Potentially `Default` could be renamed to `Rand`; my personal feeling is that
the name `Default` works well.

Similarly, `Uniform` and `Uniform01` are open to
adjustment. All three (including `Default`) could be replaced with a single
`Uniform`; but using three names does solve
two semantic issues: (1) the range of sampled values differs by type, especially
between integer and floating-point types, and (2) some possible
type-dependent implementations (such as for `Option`) cannot practically have
a uniform distribution.

[`AsciiWordChar`] is currently an oddity, used in many tests but with no hard
requirements on form or function. This could be renamed and augmented in line
with [Regex character classes](https://en.wikipedia.org/wiki/Regular_expression#Character_classes).

#### Range

There is one further uniform distribution:

*   [`Range`] specifies uniform distribution over a range `[a, b)` and supports
    integer and floating-point types

This [`Range`] is minimally changed from the current `rand`, and supports
extension to user-defined types by exposing its internal fields.

An alternative
implementation, [`range2`], has been written in an attempt to improve extension
to other types and avoid the need for an unused `zone` field with float types,
but has some drawbacks, perhaps most notably that `Range` is parameterised so
that `Range::new(low, high)` must be replaced with `range(low, high)` or
`Range::<T>::new(low, high)`.

**Question:** which `range` implementation should we choose?

Unfortunately `range2` exposes more public types like `RangeInt<X>` and
`RangeFloat<X>`, and must retain the `SampleRange` trait but *only* to identify
types for which a `Range` implementation is available. (Suggestions to improve
this code welcome.)

Note that while `Range` is open to implementation for user-defined types, its
API with a "low" and "high" may not be appropriate for many types; e.g. a
complex type may want "low_real", "high_real", "low_complex" and "high_complex"
parameters. For such uses, it is suggested the user create a new distribution
(e.g. `ComplexRange`) and not try to extend `Range`.

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
[`Sample`]: https://dhardy.github.io/rand/rand/trait.Sample.html
[`Sample::sample`]: https://dhardy.github.io/rand/rand/trait.Sample.html#tymethod.sample
[`AsciiWordChar`]: https://dhardy.github.io/rand/rand/distributions/struct.AsciiWordChar.html
[Serde]: https://serde.rs/
