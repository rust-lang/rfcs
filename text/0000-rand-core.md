- Feature Name: rand-core crate
- Start Date: 2017-09-07
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Publish a new `rand-core` crate, containing:

*   the `Rng` trait,
*   possibly a `CryptoRng` trait,
*   extension traits `SeedFromRng` and `SeedableRng`
*   helper functions and/or default implementations for `Rng` functions,

*For now*, re-export all the above in the `rand` crate. Also add two things to
`rand`:

*   the `NewSeeded` trait and its implementation for `SeedFromRng`
*   a `Sample` trait

## Links

* [Sample implementation](https://github.com/dhardy/rand/tree/master/rand_core)
* [Sample rustdoc](https://docs.rs/rand_core)
* [RFC comments (TODO)]()
* [Rand crate revision RFC] (parent RFC)

## Terminology

For clarity, this document defines the following terms:

*   **RNG**: Random Number Generator, a source of "randomness"
*   **PRNG**: Pseudo-Random Number Generator (thus algorithmic and usually reproducible)
*   **CSPRNG**: Cryptographically Secure PRNG

## What this RFC is, and what it is not

The [Rand crate revision RFC] has yielded a lot of useful suggestions, tweaks
and refactors, but has proved to cover too much material to cover in a single
thread. Despite many suggestions, it has also failed to reach consensus on a
design for the core traits.

This RFC covers:

*   What core traits `rand` should expose
*   How they should be published (new `rand_core` crate)
*   How PRNG implementations should reference (crate dependencies) and implement
    these traits
*   Construction & seeding of PRNGs

This RFC does not cover very much of the functionality in `rand`. In fact, the
only aspects of this RFC which should affect most users (excluding RNG
implementors and a few crypto users) is construction of PRNGs and the `Sample`
extension trait (a new one; see below). Specifically, this RFC does not cover:

*   How end-users should use `rand` (for now, the assumption is that all
    necessary items will be re-exported through `rand` to minimise breakage;
    this may change but will be the subject of a follow-up RFC, not this one)
*   How RNG implementations will be published (we must allow independent RNG
    crates; for now at least some RNGs may remain in the `rand` crate)
*   How values of any type other than those directly output by `Rng`/`CryptoRng`
    functions are generated
*   Thread-local or default generators (`thread_rng`, `weak_rng`)

There will be follow-up RFCs covering most of the above, just not yet
(the [Rand crate revision RFC] gives a rough overview).

## Motivation and background

Please see [the parent RFC's introduction](https://github.com/dhardy/rfcs/blob/rand/text/0000-rand-crate-redesign.md#introduction) for general background and motivation.

Motivation for this sub-RFC is

1.  To focus on the core traits
2.  To answer the question: *how should RNGs published independently be
    implemented, and what `rand` crate dependencies should they have?*
3.  To revise how PRNGs should be created
4.  To address the relevant part of the question of
    how `rand` should be split into multiple crates

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

For now, most end users should continue using the `rand` crate, and should not
need to depend on `rand_core` directly. On the other,
some of the traits concerned, such as `Rng` and `SeedableRng`, will be of
interest to end users and will be documented appropriately.

It is intended that crates publishing RNG implementations will depend on the
`rand_core` crate directly, and not on the `rand` crate.

Crates mainly using integral (especially `u32` and `u64`) or byte sequence
(`[u8]`) random values (i.e. cryptographic code) may choose to depend on
`rand_core` directly.
This may be viable for cryptography-focussed crates, but lacks several useful
features of `rand`: `thread_rng`, the `ReseedingRng` wrapper, conversion to
floating point, `Range` distribution, statistical distributions like `Normal`.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## Core traits

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

Of these functions, only `next_*` and `fill_bytes` are supposed to be
implemented directly; there is no reason an implementation should override any
of the other functions. Further, `next_f*` are only of interest to floating
point generators; there are few pseudo-random algorithms directly producing
floating-point numbers and I don't believe any of these are in common usage.

There is one other function we may wish to add: `next_u128`. More later.

But before we revise `Rng`, lets go over our goals.

### Desired properties

[Note: many of these paragraphs are simply copied from the previous version of
this section in the parent RFC; several more paragraphs have been added.]

The above trait is **long and complex**, and does not cleanly separate core
functionality (`next_*` / `fill_bytes`) from derived functionality (`gen`,
`choose`, etc.). This design pattern is successfully used elsewhere, e.g. by
[`Iterator`](https://doc.rust-lang.org/std/iter/trait.Iterator.html), but
smaller, modular traits may provide a cleaner design.

On the other hand, **ease of use** is also important. Simplifying the `Rng`
trait does not, however, imply that usage must be impaired; e.g.
[this `Sample` trait](https://dhardy.github.io/rand/rand/trait.Sample.html)
shows that simplifying the `Rng` trait need not impair usability.

**Determinism** is important for many use-cases, including scientific simulations
where it enables third parties to reproduce results, games wishing to reproduce
random creations from a given seed, and cryptography. To quote Joshua
Liebow-Feeser
[@joshlf](https://github.com/rust-lang/rfcs/pull/2106#issuecomment-323546147):

> CSPRNGs are also used deterministically in cryptographic applications. For
> example, stream ciphers use CSPRNGs to allow multiple parties to all compute
> the same pseudorandom stream based on a secret shared seed. Determinism is
> very important for a CSPRNG even if it isn't used in all applications.

**Performance** can be quite important for many uses of RNGs.
Some applications require *many* random numbers, and
performance is in many cases the main reason to use a user-space RNG instead of
system calls to access OS-provided randomness. The design therefore considers
performance an important goal for most functionality, although system calls to
access OS randomness are assumed to be relatively slow regardless.

**Performance** is also the reason there are several generator functions
(`next_*` and `fill_bytes`). Optimal implementation for each one depends on
how the generator works, thus there is no implementation of any of these using
only the other functions which would be optimal for all generators.

**Error handling:**
algorithmic random number generators tend to be infallible, but external
sources of random numbers may not be, for example some operating-system
generators will fail early in the boot process, and hardware generators can
fail (e.g. if the hardware device is removed). These failures can only be
handled via `panic` with the current `Rng` trait, but an interface exposing this
possibility of error may be desirable (on the other hand, wrapping all return
values with `Result` is undesirable both for performance and style reasons).
This is most applicable to `fill_bytes` since external generators commonly
provide byte streams (`[u8]`).

Some readers have also expressed a desire for a `CryptoRng` trait signifying
**the intention that implementations are secure**.

Finally, we wish to be able to provide a rule like
`impl<'a, R: Rng + ?Sized> Rng for &'a mut Rng` (for `CryptoRng` also). This allows
functions to take an `Rng` reference with code like `fn foo<R: Rng>(rng: R)`.
Some other `impl` rules would conflict with this thus can't be used, e.g.
`impl<R: CryptoRng+?Sized> Rng for R`.

#### Implications

Some direct implications of the above requirements:

*   There must be at least two core traits (`Rng` and `CryptoRng`) to signal intentions
*   There should *probably* not be any more (simplicity of design)
*   `Rng` cannot derive `CryptoRng`, nor can an impl rule provide the latter implicitly
*   `Rng` must have `next_u32` and `next_u64` (performance)
*   `CryptoRng` should likely have a `try_fill(&mut self, dest: &mut [u8]) -> Result<(), E>` function

That said, there are many less clear things:

*   Should `CryptoRng` extend `Rng`?
*   If so, should `CryptoRng` have any extra functions (e.g. `try_fill`) or
    should all functions be in the base trait?
*   Does `Rng` need a byte-sequence function (`fill_bytes`)?
*   Should we have an infallible `fill_bytes` *and* `try_fill`?
*   Do we need `try_next_u32` and similar functions? Hopefully not, for simplicity.
*   Can we rely on `panic` for error handling and not use `Result`?
*   If we do use `Result`, what should be the error type?

### Design questions

There are several as-yet unanswered questions:

#### Do we need to return a `Result` for error handling?

Relying on being able to catch an unwinding "panic" is not a typical design
pattern in Rust; although it appears to work, there is no user-prompting of
unhandled unwind paths (as there is for `Result`), this is a more advanced
part of Rust which will not be familiar to many Rust programmers, and it will
not work at all for binaries configured to abort on panic.

[Benchmarks](https://github.com/rust-lang/rfcs/pull/2106#issuecomment-328161354)
show no overhead of using `Result` on the `fill_bytes` function at least,
although it does complicate code a little (on the other hand for the `next_u*`
functions, there may be a tiny overhead, and code becomes significantly more
messy).

I think the best answer may be a compromise returing a `Result<(), Error>` from
`fill_bytes` (renamed to `try_fill`), while leaving the `next_u*` functions
returning simple numbers (and panicking on error). On the other hand, it is
arguable that `fill_bytes` should not return a `Result` since most generators
should be very nearly infallible anyway (only things like external hardware
generators or seekable PRNGs with short cycles where the user jumps close to
the cycle end are likely to fail without necessarily killing the whole program).

#### What should the error type be?

Assuming we do use a `Result` (and it's useful outside of `Rng` too, e.g. for
constructing an RNG), what should the error type be?

We wish to make the library `no_std` compatible, so the type cannot depend on
`std::io::Error`, or `Box` (in the future `Box` should be
available to some `no_std` environments, but depend on having an allocator).

Some `no_std` compatible options are `&'static str`, a fixed-length ASCII or
UTF-8 buffer, a pointer to a statically allocated buffer (possibly), a
numeric error code, or something like
`enum Error { Static(&'static str), Dynamic(*mut str) }` where `Error::Dynamic`
requires the handler to free memory with whichever allocator is in use.
It is [also possible](https://github.com/rust-lang/rfcs/pull/2106#issuecomment-327442573)
to cast a dynamically-allocated `Box<String>` to a `&'static str` and leak the
memory (no free) on recovery — at least, this works when tested.

If different types are used depending on whether `std` is available, we could
perhaps use an error type like the following; this may be the best option since
`Error::Msg` is always present and both variants may implement
`core::fmt::Debug` allowing simple "print and stop/continue" handlers to avoid
`cfg`-dependent code.

```rust
enum Error {
    // this variant always present:
    Msg(&'static str),
    // this variant only present when std is available:
    ChainedIo(&'static str, ::std::io::Error),
}

impl ::core::fmt::Debug for Error {
    // ...
}
```

#### Given that at least one function returns a `Result`, do we also need equivalent functions not returning a `Result`?

Regarding the `next_u*` functions, my opinion is that there should be infallible
versions of these functions (returning simple numbers) since this is what many
uses (e.g. the `rand::distributions`) module expect. I have not seen any real
demand for a version of these functions returning a `Result`; hence in my
opinion we do not need two versions of these functions.

Regarding `fill_bytes` / `try_fill`, the little benchmarking done shows no
performance impact of returning a `Result`. Handling a `Result` involves some
additional code complexity, but since this function is mostly of interest to
cryptographic code wanting a sequence of bytes, and these users are the ones
requesting error handling, this extra code seems reasonable. It is slightly
unfortunate that any code using `try_fill` on an infallible PRNG must still do
error handling or use `unwrap`, but this is probably not a big deal. Therefore
I believe a `try_fill(..) -> Result<(), Error>` function is sufficient and an
infallible `fill_bytes` is not needed.

#### What is the purpose of `CryptoRng`?

Apparently `CryptoRng` should allow the compiler to enforce safe use of RNGs
[through type safety](https://github.com/rust-lang/rfcs/pull/2106#issuecomment-326013067).
Unfortunately this leaves the question of which RNGs should be a `CryptoRng`
with only a vague answer.

It has been [suggested](https://github.com/rust-lang/rfcs/pull/2106#issuecomment-328666650)
that a mathematical definition of CSPRNGs be used to define which RNGs should
implement `CryptoRng`; unfortunately this assumes a world of perfect provability;
in reality "cryptographic RNGs", as with public-key cryptography, tend to rely
on unproven mathematics. Further, this definition is not very useful for
external generators where the implementation used may be platform or hardware
dependent, and is likely outside of the implementor's control (e.g. `OsRng`).

We could instead ask ourselves questions like *how well is the theory behind
the generator understood?*, *what are the minimum and average cycle lengths?*,
*are there any bad states?, and *how trusted is the implementation*? It is not
possible to give any exact criterion using this approach, and if implementors
must answer these questions themselves quality will vary, but an external body
could perhaps give somewhat consistent answers. Unfortunately this would be
expensive or require a lot of volunteered time from well-qualified individuals,
and in any case it would not stop mis-use of `CryptoRng` in private crates.

As an approximation to the above, we could instead choose to only implement
`CryptoRng` for RNGs already well regarded and in wide use, and where the
implementations are well tested (if possible by reproducing output from a
reference implementation). This penalises any
new or obscure algorithms, though perhaps this is acceptable.

The only remaining suggestion I have is to set a fairly low bar of *is the
generator at least minimally secure*, in that

*   there is no known algorithm for predicting future output given some past
    output, within reasonable computational time,
*   there are no obvious flaws,
*   there are no short cycles
*   there is no significant bias

Even this criterion is not trivial to answer; fortunately publications of RNGs
do usually try to answer these questions. If we do go with this relatively easy
criterion, possibly we should use a different name like `MinimalCryptoRng`
or `NonTrivialRng` (we *could* even have a trait for this as well as
`CryptoRng` if we really wanted).

Before choosing an answer, we should also look at where such a trait could be
useful:

*   To select an RNG in a cryptography-focussed library. But since authors of
    such a library should have a good background in cryptography, they should
    be in a better position to determine which RNGs are suitable than we are.
*   To select an RNG in code using a library which requires cryptographic
    numbers but does not source them itself. In this case a `CryptoRng` trait
    would allow the library some control over the quality of the RNG used
    without actually specifying an RNG. But is this a real use-case — if the
    library requires secure random numbers should it really let the user
    provide them? Possibly; I don't know.
*   To require that future numbers are at least not trivially predictable. This
    has a couple of use-cases: any time knowledge of the RNG state might allow
    remote DoS attacks (e.g. use of a hash-map where the implementation and
    hash-randomisation is known), then non-trivially-predictable RNGs should be
    used, but there is usually little point requiring any more than this (since
    the attacks are usually not easy even when the RNG state is known, the
    potential damage is low, and in any case "not-trivially-predictable" usually
    ends up meaning *impossible to predict* in practice. The second use-case is
    operations seeding a PRNG, where in some cases use of a weak RNG may
    result in an accidental clone or bad state.

### `Rng` and `CryptoRng` design

Here we come to some actual designs. My current favourite is the first one
below.

For those designs where `CryptoRng` does not extend or automatically implement
`Rng`, we could add a wrapper type allowing any `CryptoRng` to be used as an
`Rng`; but this is likely not necessary.

For all the following, we could add a wrapper type going the other way, named
something like `FakeCryptoRng`, allowing any `Rng` to be used as a `CryptoRng`.
I'm not sure if there's much need for this.

#### Design 1: `CryptoRng` extends `Rng` (marker only)

Provide two related traits:

```rust
pub trait Rng {
    fn next_u32(&mut self) -> u32;
    fn next_u64(&mut self) -> u64;
    
    fn try_fill(&mut self, dest: &mut [u8]) -> Result<(), Error>;
}

pub trait CryptoRng: Rng {}
```

Advantages of this design:

*   A `CryptoRng` is *exactly* an `Rng` with the note that *it intends to be secure*

#### Design 2: `CryptoRng` extends `Rng`

Provide two related traits:

```rust
pub trait Rng {
    fn next_u32(&mut self) -> u32;
    fn next_u64(&mut self) -> u64;
}

pub trait CryptoRng: Rng {
    fn try_fill(&mut self, dest: &mut [u8]) -> Result<(), Error>;
}
```

As above, we *might* wish to provide a wrapper to convert an `Rng` to a `CryptoRng`.

Advantages of this design:

*   Fairly clean

Disadvantages:

*   No direct bytes output from non-crypto RNGs

#### Design 3: Separate `Rng`, `CryptoRng`

Provide two entirely separate traits:

```rust
pub trait Rng {
    fn next_u32(&mut self) -> u32;
    fn next_u64(&mut self) -> u64;
    
    // possibly also a `fill_bytes` function
}

pub trait CryptoRng {
    fn try_fill(&mut self, dest: &mut [u8]) -> Result<(), Error>;
}
```

Advantages of this design:

*   Fairly clean (other than conversion wrappers)
*   Option to put `CryptoRng` in another trait with no dependency on `Rng`

Disadvantages:

*   Implementations of `CryptoRng` should make sure they implement `Rng` too if
    they wish to provide optimal performance there. On the other hand,
    implementations may not want to if they wish to avoid all dependence on
    `Rng`. This could even lead to multiple implementations of an algorithm.

#### Design 4 (not recommended): `RawRng`

The experimental `never_type` feature `!` as well as void types (`enum Void {}`)
allow compile-time elimination of "impossible type" code paths. In theory this
allows use of a `Result` type with compile-time-verified-safe unwrap with zero
overhead (I have benchmarked zero performance overhead, but there may be
memory overhead). This could be used for a more exotic design like the following:

```rust
pub trait RawRng<E> {
    // these *could* also return `Result`, if infallible variants are implemented in `Rng`:
    fn next_u32(&mut self) -> u32;
    fn next_u64(&mut self) -> u64;
    
    fn try_fill(&mut self, dest: &mut [u8]) -> Result<(), E>;
}

pub trait Rng: RawRng<!> {
    // implement infallible version for convenience; optional:
    fn fill_bytes(&mut self, dest: &mut [u8]) {
        // the unwrap is safely eliminated iff error type is not representable:
        self.try_fill(dest).unwrap_or_else(|e| e)
    }
}

// extension trait is purely a marker trait:
trait CryptoRng<E>: RawRng<E> {}
```

Advantages of this design:

*   Very explicit: fallible `CryptoRng<Error>` and infallible `CryptoRng<!>` are both possible
*   A single base trait, `RawRng`, is available (though this is not object-safe,
    meaning you can't have `rng: &RawRng`)

Disadvantages:

*   The never type `!` is still unstable (although `Void` is not and should allow the same)
*   Uses one more trait than other designs, and even more variants when parameterised
*   Function duplication in `Rng`: `try_fill` and `fill_bytes`; if `try_next_u32`
    were used in `RawRng`, `next_u32` would also be needed in `Rng` (so users
    don't have to do the awkward unwrap)

## Default implementations of functions

Most of the functions shown above are prototypes only with no default
implementation, not because we should do this (although it is a reasonable
possibility), but because this is a separate topic. Reasonably, the functions
can be implemented in terms of each other in various ways, with applicability
depending on which type the RNG outputs natively:

*   `next_u32(&mut self) -> u32 { self.next_u64() as u32 }`
*   `next_u64` taking two `next_u32` values and composing
*   `fill_bytes` using `next_u32` or `next_u64` as many times as needed,
    converting each to a byte slice and copying to the output
*   `next_u32` and `next_u64` filling a byte-slice via `fill_bytes` then casting

Should any of these be default implementations? The first transformation is
trivial, the second and fourth are not very complex, while the third is a bit
harder to get right and should definitely be provided. But we could:

*   Provide no default implementations, but make all non-trivial conversions
    available via simple functions
*   Use cyclic default implementations where e.g. `next_u32` uses `fill_bytes`,
    `next_u64` uses `next_u32` and `fill_bytes` uses `next_u64`
*   Provide only a sub-set of implementations by default

*I argue* that the best option is to provide no default implementations, for the
following reasons:

*   It makes the traits themselves as simple as possible
*   It makes the correct way to implement `Rng`/`CryptoRng` obvious (no questions
    about which default implementations are adequate)
*   It forces any *wrapper implementation* to wrap each function explicitly;
    since throwing away bits is allowed (`next_u64() as u32` or `fill_bytes`
    implemented via `next_u*`), a wrapper using a default implementation may
    output different results (see
    [here](https://github.com/rust-lang/rfcs/pull/2106#issuecomment-326763171) and
    [here](https://github.com/rust-lang/rfcs/pull/2106#issuecomment-326794778)
    for more)

Unfortunately this does mean that any implementation of `Rng` must include
several annoying trivial functions (casting or wrapping some provided
implementation), but there should not end up being a huge number of users
needing to implement `Rng` (relative to users of `Rng`).

## 128-bit support?

Should we add `fn next_u128(&mut self) -> u128` besides `next_u32` and
`next_u64`?

Most current PRNG algorithms target the `u32` or `u64` type. Are there any
native-`u128` algorithms available now? I don't know of any, but several
algorithms could be adapted from `u32` or `u64` types. Would there be any
advantages of 128-bit algorithms? Performance *might* be better for applications
wanting `u128` values or byte streams.

Of course the possible performance advantage for byte-streams can be realised
without a `next_u128` function anyway. So is anyone likely to want a fast `u128`
generator? No idea, but I see no reason not to plan for the (reasonable?) future.

Why might we *not* want to add `next_u128`? First, it could be added later
without breakage, but *only if* it has a default implementation. Second, it's
more code for implementations to write, especially since `u128` is currently
unstable and must be used behind a `cfg` attribute; this implies an extra
configuration must be tested too.

On the whole, the best approach may be to give `next_u128` a default
implementation regardless of whether it is added now or later, and whether
other functions have default implementations, simply because very few (if any)
PRNGs will be able to provide a more efficient implementation than simply
combining two `u64` values.

Proposal: add `next_u128` now, behind a feature flag, with a default
implementation.

## Extension traits

### Creation of securely-seeded RNGs

Often, PRNGs are seeded from "somewhat random" sources such as the system
clock. Rust's `rand` tries to make the best option easy by making it possible
to construct random number generators from the system generator, e.g. the
[`ChaChaRng`] type supports the [`Rand`] trait. This makes it possible to
construct a securely seeded `ChaChaRng` with:

```rust
use rand::{Rng, OsRng, ChaChaRng};

let mut rng: ChaChaRng = OsRng::new().unwrap().gen();
```

This RFC seeks to introduce an alternative:

```rust
// items may be moved to other crates, but for now at least are accessible here:
use rand::{ChaChaRng, NewSeeded};

let mut rng = ChaChaRng::new();
```

Here, `NewSeeded` is a trait providing the `new` function. It is automatically
implemented for any type implementing `SeedFromRng`:

```rust
/// Support mechanism for creating securely seeded objects 
/// using the OS generator.
/// Intended for use by RNGs, but not restricted to these.
/// 
/// This is implemented automatically for any PRNG implementing `SeedFromRng`,
/// and for normal types shouldn't be implemented directly. For mock generators
/// it may be useful to implement this instead of `SeedFromRng`.
#[cfg(feature="std")]
pub trait NewSeeded: Sized {
    /// Creates a new instance, automatically seeded via `OsRng`.
    fn new() -> Result<Self>;
}

#[cfg(feature="std")]
impl<R: SeedFromRng> NewSeeded for R {
    fn new() -> Result<Self> {
        let mut r = OsRng::new()?;
        Self::from_rng(&mut r)
    }
}
```

The above code should be included in `rand`, not `rand_core`. Later, if `OsRng`
gets moved to its own crate, this code could be moved there (for discussion in
a new RFC). The `SeedFromRng` type, on the other hand, needs to be in
`rand_core`:

```rust
/// Support mechanism for creating random number generators seeded by other
/// generators. All PRNGs should support this to enable `NewSeeded` support,
/// which should be the preferred way of creating randomly-seeded generators.
pub trait SeedFromRng: Sized {
    /// Creates a new instance, seeded from another `Rng`.
    fn from_rng<R: Rng+?Sized>(rng: &mut R) -> Result<Self>;
}
```

It is possible for types to implement `NewSeeded` directly if they do not
implement `SeedFromRng`. This may be of use to mock RNGs but is probably not
widely useful.

(Note: both `NewSeeded` and `SeedFromRng` could be restricted to types
implementing `Rng`; the current traits do not do this, allowing usage by things
which are not RNGs. This is probably fine.)

#### Rationale for `SeedFromRng` and `NewSeeded`

*Why* should we go with the above two-trait approach to seeding new RNGs?

First, *splitting the code into two parts* gives us a reusable way of creating
one RNG from another without much extra code. Second, it means the RNG
implementations being seeded don't need to know anything about where the seed
data is coming from (`OsRng` in this case). This is in fact what the old
approach achieved.

So why should we replace the old method of creation at all? There are several
reasons:

*   The `NewSeeded` trait makes creation of properly-seeded RNGs as simple and
    intuitive as possible: `MyRng::new()`
*   Using a trait specific for the purpose, `SeedFromRng`, instead of simply
    implementing [`Rand`] lets us better document that creating RNGs is a
    special thing; one should choose here whether they want determinism or
    secure seeding
*   `SeedFromRng` will also let us restrict which RNGs can be used to seed
    other RNGs (see alternatives just below)
*   `SeedFromRng` also aids crate separation: an implementation of the [`Rand`]
    trait or of a distribution trait like [`IndependentSample`] or
    [`Distribution`] could offer similar functionality, but these traits all
    deal with converting RNG output to other types, which (in my opinion)
    should be an extra layer built on top of (and independent from) `rand_core`

#### Alternatives

We may wish to tweak `SeedFromRng::from_rng` to only accept source RNGs of
type `CryptoRng` (or if we have it, `WeakCryptoRng` or similar). Why? Seeding
some non-crypto RNGs this way can [accidentally make one a clone of the other](https://play.rust-lang.org/?gist=6c12ea478440e452b135a6354024a909&version=stable).
This accidental cloning is impossible for even the weakest crypto RNGs (because
there should be no trivial way to predict future output from past output).
(The reason this is an "alternative" and not the default is because the question
of what exactly `CryptoRng` should mean needs an answer before we try to use
it.)

We could require implementation of `SeedableRng<&mut Rng>` or
`impl<R: Rng> SeedableRng<R> for MyRng` instead. Note that the former does not
allow static-dispatch and the latter appears to conflict with any other
implementation of `SeedableRng<T>` even for fixed `T` not implementing `Rng`
(this may be a bug).

We could try to directly implement `NewSeeded` for any PRNG supporting
`SeedableRng` where the `Seed` type can be generated by [`Rand`] or some
distribution. This can only be implemented automatically (via a generic impl
rule) if the `Seed` type is an associated type and is `Sized`;
this implies some of the current impls must be changed (e.g. `ChaChaRng`
impls `SeedableRng<&[u32]>`, which does not have a fixed size `Seed`) and
also that no PRNG can support `SeedableRng` for multiple seed types.

We might also wish to rename `NewSeeded` and/or its `new` function to emphasise
that this seeds the RNG from the OS. (It is possible some users may create an
alternative for their own uses, e.g. seeding from a single master generator for
reproducibility or seeding from some other source for embedded
applications without an OS source.) Never-the-less, `NewSeeded` should be the
default way to create any new RNG, so it and `new` should have simple short
names.

### Deterministic seeding

`rand` currently provides the [`SeedableRng`] trait:

```rust
pub trait SeedableRng<Seed>: Rng {
    fn reseed(&mut self, Seed);
    fn from_seed(seed: Seed) -> Self;
}
```

Is the `reseed` function redundant? Possibly, but it is also easy to implement
(we could provide a default impl in terms of `from_seed`). I don't see much need
to change this myself.

#### Streams

One thing this trait does miss is support for [multiple streams]. Although this
could be supported by implementing for a `Seed` type like `([u32; 4], u32)`,
this is untidy and inconsistent. I suggest changing the functions to the
following (but am not certain this is a good plan):

```rust
pub trait SeedableRng<Seed>: Rng {
    fn reseed(&mut self, Seed, stream: u64);
    fn from_seed(seed: Seed, stream: u64) -> Self;
    fn num_streams() -> u64;
}
```

For consistency between generators, streams should probably be selected using
`stream % num_streams()`. The `num_streams()` function may not be needed so
long as RNGs document how many unique streams are available.

Alternatively, we could ignore streams for normal seeding, but expect all
generators support a fixed type like `(u64, u64)` (seed, stream); see the
guideline on `SeedableRng<u64>` below.

#### Implementation guidelines

[`SeedableRng`] should only be implemented by fixed generators
(i.e. where output is repeatable, cross-platform and it is not expected that the
algorithm will be adjusted in the future). This implies that `StdRng` should not
implement `SeedableRng` because the underlying generator may be changed; also,
output is platform-dependent (currently it may be `IsaacRng` or `Isaac64Rng`).

We could suggest that PRNGs implement of `SeedableRng<u64>`.
This seed type should not be used for cryptography due to the limited bits, but
it is perfectly sufficient for simulators and games wanting reproducible output
(`u32` should also be sufficient). Having standard PRNGs support a common
seed type like `u64` would make it easier for these applications to switch from
one generator to another. (Implementation: generators with more than 64 bits of
internal state could pad the seed with zero or any fixed constant they like; it
shouldn't matter as long as the generator's seeding requirements are met (this
may imply a non-zero constant is more appropriate) and the extra bits are
fixed.)

#### Alternatives

We could use `u32` instead of `u64` for the stream selector and standard seed
type. Probably either is fine.

Existing `Seed` types used by implementations are `[u32; 4]`, `&[u32]` and
`&[u64]`. The intention of using slices is to allow partial seeding; it is
perfectly valid for these PRNGs to have some (or even all) of their state seeded
to 0. The disadvantages of this are that seed types are inconsistent between
generators and are not `Sized`, so cannot be generated by [`Rand`] or similar
without also specifying a size. We could instead suggest implementation only
for `Sized` types, or for a `Sized` type *and* a byte-slice (`&[u8]`).

We could move the `Seed` type from a trait parameter to an associated type:

```rust
pub trait SeedableRng: Rng {
    type Seed;
    ..
}
```

Personally I would rather not do this: it prevents implementation of
`SeedableRng` for multiple types.

Instead of using a generic `Seed` type, we could have multiple versions of
`SeedableRng` each with their own fixed seed type, e.g. one for `u64` or
`(u64, u64)` for simple specification (optionally with stream support), and
another for `&[u8]` allowing seeding from arbitrary byte slices (possibly with
a function `fn seed_len() -> u32` to specify the ideal seed length in bytes).
We could do this in addition to a generic version of `SeedableRng` taking the
full seed as a `Sized` type, or even without this.

### Seeking

Some PRNG algorithms support seeking to an arbitrary position in their output
without having to generate all numbers in between (but not all). For example,
rand already has [`ChaChaRng::set_counter`]. This particular PRNG suggests
using `set_counter` to specify a nonce; another use would be to effectively
divide a PRNG's output into multiple streams.


## Split into multiple crates

We propose splitting `rand` into multiple traits, so that the appropriate
subsets of functionality are available to the main sub-sets of users, without
too many extras. We consider the following usage classes:

*   implementations of RNGs
*   cryptographic applications using mainly byte sequences
*   numeric applications generating values using various distributions
*   other uses of random numbers

The first of these requires the `Rng` and/or `CryptoRng` traits and optionally
some extension traits, but shouldn't directly require anything else (except for
testing).

Cryptographic applications may not require code converting RNG outputs to
other types or distributions mapping to different ranges and/or shapes (this is
debatable; e.g. sampling from ranges or random exponential back-off may be
useful). Cryptographic applications will require access to an external source
of randomness (usually `OsRng`) and may well use a user-space CSPRNG.
Such applications will obviously need the `Rng` or `CryptoRng` trait (directly
or indirectly), will usually need `OsRng`, and may use another crate implementing
a CSPRNG; it is debateable whether other parts of `rand` will be required.

Numeric applications (simulations and games) will often want to map random
values to floating-point types, use specific ranges, and distributions like
`Range` and `Normal`. These applications will usually use a PRNG algorithm
and may use `OsRng` for seeding. For now, we assume this functionality will
all be accessed through the `rand` crate and will not discuss this use-case in
detail in this RFC.

Finally, there are various "tangential" uses of random numbers: most notably
`std::hashmap` randomises its hash function and requires a fast, secure source
of random numbers for applications where an attacker could choose keys to
avoid denial-of-service attacks; other possible uses of random numbers could be
random UUIDs and randomised algorithms (e.g. randomised sort to prevent an
attacker sending data designed to prevoke worst-case performance).

For now, this RFC proposes:

*   a new `rand_core` crate containing the `Rng` trait, `CryptoRng` (in whatever
    form(s) that takes), extension traits like `SeedableRng` and `SeedFromRng`,
    possibly a mock implementation of `Rng`, and functions to aid implementing
    `Rng`
*   the existing `rand` crate should re-export all the above except the helper
    functions so that most users will not be affected by this RFC

In the future, we may wish to introduce some more crates, as follows. This list
is provided for insight into the larger picture only; please do use this RFC to
comment on these crates (use the [Rand crate revision RFC] instead):

*   a `rand_os` crate exposing `OsRng` and possibly `NewSeeded`
*   a `rand_chacha` crate for `ChaChaRng`
*   a `rand_isaac` crate for the ISAAC RNGs
*   other PRNG crates may be adopted, e.g. `pcg_rand`
*   possibly a `rand_thread` crate for `thread_rng` (very speculative)


## Generating values

Removing other member functions from the [`Rng`] trait poses a problem: how do
users access this functionality, and how do existing users adapt their code?

I suggest that for now we add an extension trait to `rand` to house these
functions:

```rust
pub trait Sample: Rng {
    // all member functions removed from Rng go here
}
```

Existing users need then only `use rand::Sample;`, or if they previously had
`use rand::*;` then no adaptation would be needed.

In the future, these member functions could be further tweaked. Please see
[this mock design](https://dhardy.github.io/rand/rand/trait.Sample.html) for an
idea; however this is beyond the scope of this RFC which is mainly concerned
about introducing `rand_core` and tweaking `Rng`.

The naming conflict with [`distributions::Sample`] is unfortunate but not
necessarily a fatal problem since `rand` doesn't currently reexport this trait
at the top level of the crate. Further, there are likely very few users of this
trait (who couldn't simply switch to [`IndependentSample`]), since the extra
capability `Sample` provides (modifying its own state) is not one required by
true *distributions*. To avoid confusion we could simply remove
[`distributions::Sample`], as
[argued in the parent RFC](https://github.com/dhardy/rfcs/blob/rand/text/0000-rand-crate-redesign.md#traits-governing-generation),
or we could leave it for now (although it should be removed eventually in any
case).

# Drawbacks
[drawbacks]: #drawbacks

We may choose not to split `rand` into multiple crates, but I feel this proposal
gives a good division between the parts needed to implement an RNG and the
rest tof `rand`.

Even if we didn't split `rand`, I believe we should still seriously consider the
other modifications proposed here.

# Unresolved questions
[unresolved]: #unresolved-questions

The member functions of the `Rng` trait and extension by a marker-only
`CryptoRng` is a design which has been selected after a long look at various
options; please see the [Rand crate revision RFC] for the history behind this.
This particular design seems to reasonably well meet all design requirements
while remaining reasonably simple, but is not obviously *the best option*.

There are several suggestions for the semantic meaning of `CryptoRng` above,
but no real resolution. Hopefully the community can provide some useful
suggestions here. My personal preference would be to make `CryptoRng`
use the relatively low bar of "no known feasible attack or significant weakness"
as mentioned above. I suspect opinions will differ on this.

The `Error` type needs to be defined.

Should `SeedFromRng`, `NewSeeded` and `SeedableRng` all extend the `Rng` trait?
There seems no real reason for this aside from the name of the third implying
that implementations are `Rng`s, on the other hand all three are designed with
`Rng` in mind and may not have alternative uses. In theory `SeedFromRng` could
be used to randomly initialise buffers for example, but this can also be done
via the `gen()` function or `try_fill`.

There are many more questions regarding the extension traits and seeding of
PRNGs which only have suggestions for answers above.


[Rand crate revision RFC]: https://github.com/rust-lang/rfcs/pull/2106
[`ChaChaRng`]: https://docs.rs/rand/0.3.16/rand/chacha/struct.ChaChaRng.html
[`Rand`]: https://docs.rs/rand/0.3.16/rand/trait.Rand.html
[`IndependentSample`]: https://docs.rs/rand/0.3.16/rand/distributions/trait.IndependentSample.html
[`Distribution`]: https://dhardy.github.io/rand/rand/distributions/trait.Distribution.html
[`SeedableRng`]: https://docs.rs/rand/0.3.16/rand/trait.SeedableRng.html
[multiple streams]: http://www.pcg-random.org/posts/critiquing-pcg-streams.html
[`ChaChaRng::set_counter`]: https://docs.rs/rand/0.3.16/rand/chacha/struct.ChaChaRng.html#method.set_counter
[`Rng`]: https://docs.rs/rand/0.3.16/rand/trait.Rng.html
[`distributions::Sample`]: https://docs.rs/rand/0.3.16/rand/distributions/trait.Sample.html
