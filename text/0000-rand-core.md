- Feature Name: rand-core crate
- Start Date: 2017-09-07
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Introduction

## Summary
[summary]: #summary

Publish a new `rand-core` crate, containing:

*   the `Rng` trait, cut down to just `next_u32`, `next_u64`, `fill_bytes` and `try_fill_bytes` [new]
*   a [new] `CryptoRng` marker trait as an extension of `Rng`
*   extension traits `SeedFromRng` [new] and `SeedableRng` (modified)
*   a [new] `Error` struct with associated `ErrorKind` enum
*   helper functions for implementing for `Rng` functions

*For now*, re-export all the above in the `rand` crate. Also add two things to
`rand`:

*   the `NewSeeded` trait and its implementation for `SeedFromRng`
*   a `Sample` trait

## Links

* [Sample implementation](https://github.com/dhardy/rand/tree/master/rand_core)
* [Sample rustdoc](https://docs.rs/rand_core)
* [RFC comments](https://github.com/rust-lang/rfcs/pull/2152)
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
*   How they should be published (new `rand-core` crate)
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
need to depend on `rand-core` directly. On the other hand,
some of the traits concerned, such as `Rng` and `SeedableRng`, will be of
interest to end users and will be documented appropriately.

It is intended that crates publishing RNG implementations may depend on the
`rand-core` crate directly, and not on the `rand` crate.

Crates mainly using integral (especially `u32` and `u64`) or byte sequence
(`[u8]`) random values (i.e. cryptographic code) may choose to depend on
`rand-core` directly.
This may be viable for cryptography-focussed crates, but lacks several useful
features of `rand`: `thread_rng`, the `ReseedingRng` wrapper, conversion to
floating point, `Range` distribution, statistical distributions like `Normal`.

## Implementing RNGs

Comment thread: [issue #13](https://github.com/dhardy/rand/issues/13)

RNGs may be implemented by implementing the `Rng` trait from `rand-core`.

It is recommended that RNGs implement the following additional traits:

*   RNGs recommendable for use in cryptography should also implement the
    `CryptoRng` trait.
*   RNGs should implement the `Debug` trait with a custom implementation which
    displays only the struct name (e.g. `write!(f, "IsaacRng {{}}")`). This is
    to avoid accidentally leaking the state of RNGs (especially cryptographic
    RNGs) in logs.
*   RNGs should implement `Clone` if and only if they are entirely
    deterministic: that is, a clone will output the same sequence as the
    original generator, assuming no additional perturbation. This implies that
    RNGs using external resources (e.g. `OsRng`, `ReadRng`) should not implement
    `Clone`.
*   RNGs should never implement `Copy` since this makes it easy for users to
    write incorrect code (inadvertently cloning and reusing part of the output
    sequence)
*   Non-deterministic RNGs should not implement `Eq` or `PartialEq`. We also do
    not encourage deterministic RNGs to implement these traits; they do not
    appear either useful or harmful. (Tests are recommended to use a short
    sequence of output to test equivalence, e.g.
    `for _ in 0..16 { assert_eq!(rng1.next_u32(), rng2.next_u32()); }`.)
*   We recommend deterministic RNGs implement serialization via `serde`
    behind a feature gate, as in
    [rand#189](https://github.com/rust-lang-nursery/rand/pull/189). [TODO]

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

A summary of the proposed changes:

1.  Publish a new `rand-core` crate as a member of
    `https://github.com/rust-lang-nursery/` using
    [this code](https://github.com/dhardy/rand/tree/master/rand_core).
2.  Remove the following from the `rand` crate: `Rng`, `SeedableRng`.
3.  Publically export (`pub use`) the following `rand-core` items in `rand`:
    `Rng`, `CryptoRng`, `SeedFromRng`, `SeedableRng`, `ErrorKind`, `Error`.
4.  Add two new traits to `rand`: `Sample` and `NewSeeded`; additionally
    implement `NewSeeded` for any type implementing `SeedFromRng`.

Details and justification follow.

## `Rng` and `CryptoRng` traits

Introduce the following new traits:

```rust
pub trait Rng {
    fn next_u32(&mut self) -> u32;
    fn next_u64(&mut self) -> u64;
    
    fn fill_bytes(&mut self, dest: &mut [u8]);
    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), Error>;
}

pub trait CryptoRng: Rng {}
```

Justification follows.

### Current `Rng` trait

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
*   `CryptoRng` should likely have a `try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), E>` function

That said, there are many less clear things:

*   Should `CryptoRng` extend `Rng`?
*   If so, should `CryptoRng` have any extra functions (e.g. `try_fill_bytes`) or
    should all functions be in the base trait?
*   Does `Rng` need a byte-sequence function (`fill_bytes`)?
*   Should we have a `fill_bytes` *and* `try_fill_bytes`?
*   Do we need `try_next_u32` and similar functions? Hopefully not, for simplicity.
*   Can we rely on `panic` for error handling and not use `Result`?
*   If we do use `Result`, what should be the error type?

### Design questions

There are several as-yet unanswered questions:

#### Do we need to return a `Result` for error handling?

Comment thread: [issue #8](https://github.com/dhardy/rand/issues/8)

Relying on being able to catch an unwinding "panic" is not a typical design
pattern in Rust; although it appears to work, there is no user-prompting of
unhandled unwind paths (as there is for `Result`), this is a more advanced
part of Rust which will not be familiar to many Rust programmers, and it will
not work at all for binaries configured to abort on panic.

[Benchmarks](https://github.com/rust-lang/rfcs/pull/2106#issuecomment-328161354)
appear to show no overhead of using `Result` on the `fill_bytes` function at least,
although it does complicate code a little (on the other hand for the `next_u*`
functions, there may be a tiny overhead, and code becomes significantly more
messy).

I think the best answer may be a compromise returing a `Result<(), Error>` from
`fill_bytes` (renamed to `try_fill_bytes`), while leaving the `next_u*` functions
returning simple numbers (and panicking on error). On the other hand, it is
arguable that `fill_bytes` should not return a `Result` since most generators
should be very nearly infallible anyway (only things like external hardware
generators or seekable PRNGs with short cycles where the user jumps close to
the cycle end are likely to fail without necessarily killing the whole program).

#### What should the error type be?

Roughly, we settled on the following. [Details later](#error-handling).

```rust
pub struct Error {
    pub kind: ErrorKind,
    pub cause: Option</* omitted */>,
}
```

#### Given that at least one function returns a `Result`, do we also need equivalent functions not returning a `Result`?

Regarding the `next_u*` functions, my opinion is that there should be infallible
versions of these functions (returning simple numbers) since this is what many
uses (e.g. the `rand::distributions`) expect. I have not seen any real
demand for a version of these functions returning a `Result`; hence in my
opinion we do not need two versions of these functions.

Regarding `fill_bytes` / `try_fill_bytes`, the little benchmarking done shows no
performance impact of returning a `Result`. Handling a `Result` involves some
additional code complexity, but since this function is mostly of interest to
cryptographic code wanting a sequence of bytes, and these users are the ones
requesting error handling, this extra code seems reasonable. It is slightly
unfortunate that any code using `try_fill_bytes` on an infallible PRNG must still do
error handling or use `unwrap`, but this is probably not a big deal. Therefore
I believe a `try_fill_bytes(..) -> Result<(), Error>` function is sufficient and an
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
*are there any bad states?*, and *how trusted is the implementation*? It is not
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

### `Rng` and `CryptoRng` design, including alternatives

Here we look at to some actual designs for the traits.

For those designs where `CryptoRng` does not extend or automatically implement
`Rng`, we could add a wrapper type allowing any `CryptoRng` to be used as an
`Rng`; but this is likely not necessary. `CryptoRng` implementations could also
implement `Rng` directly.

For all the following, we could add a wrapper type going the other way, named
something like `FakeCryptoRng`, allowing any `Rng` to be used as a `CryptoRng`.
I'm not sure if there's much need for this.

#### Design 1: `CryptoRng` extends `Rng` (marker only)

(This is the design used above.)

Provide two related traits:

```rust
pub trait Rng {
    fn next_u32(&mut self) -> u32;
    fn next_u64(&mut self) -> u64;
    
    fn fill_bytes(&mut self, dest: &mut [u8]);
    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), Error>;
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
    
    fn fill_bytes(&mut self, dest: &mut [u8]);
}

pub trait CryptoRng: Rng {
    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), Error>;
}
```

As above, we *might* wish to provide a wrapper to convert an `Rng` to a `CryptoRng`.

#### Design 3: Separate `Rng`, `CryptoRng`

Provide two entirely separate traits:

```rust
pub trait Rng {
    fn next_u32(&mut self) -> u32;
    fn next_u64(&mut self) -> u64;
    
    fn fill_bytes(&mut self, dest: &mut [u8]);
}

pub trait CryptoRng {
    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), Error>;
}
```

Advantages of this design:

*   Fairly clean (other than conversion wrappers)
*   Option to put `CryptoRng` in another trait with no dependency on `Rng`

Disadvantages:

*   Implementations of `CryptoRng` should implement `Rng` too. This implies
    that both traits should probably be provided by the same crate.
*   Users requiring a `CryptoRng` don't get access to `next_u*` unless they
    also require `Rng`.
*   Users requiring `Rng` don't get access to a byte-fill function unless
    `Rng` *also* has such a function (redundancy)

#### Design 4 (not recommended): `RawRng`

The experimental `never_type` feature `!` as well as void types (`enum Void {}`)
allow compile-time elimination of "impossible type" code paths. In theory this
allows use of a `Result` type with compile-time-verified-safe unwrap with zero
overhead (I have benchmarked zero performance overhead, but there may be
memory overhead). This could be used for a more exotic design like the following:

```rust
pub trait RawRng<E> {
    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), E>;
}

pub trait Rng: RawRng<!> {
    // These could potentially be moved to RawRng:
    fn next_u32(&mut self) -> u32;
    fn next_u64(&mut self) -> u64;
    
    // implement infallible version of try_fill_bytes for convenience; optional:
    fn fill_bytes(&mut self, dest: &mut [u8]) {
        // the unwrap is safely eliminated iff error type is not representable:
        self.try_fill_bytes(dest).unwrap_or_else(|e| e)
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
*   Function duplication in `Rng`: `try_fill_bytes` and `fill_bytes`; if `try_next_u32`
    were used in `RawRng`, `next_u32` would also be needed in `Rng` (so users
    don't have to do the awkward unwrap)

### Default implementations of functions

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

### Native 16-bit and 128-bit support?

Should we add `next_u16` and `next_u128` functions besides
`next_u32` and `next_u64`?

### 16-bit

It might make sense to have a `next_u16` function if a generator interface is
desired on a 16-bit CPU, but so far there has been little interest in this.
See [issue #11](https://github.com/dhardy/rand/issues/11) for a discussion on
this topic.

There appears to be very little in the way of published 16-bit PRNGs, and any
cryptographic algorithm would likely be quite slow (needing to touch a lot of
state), so simple weak generators and hardware generators are the most likely
candidates.

If there is demand for this in the future, it will be possible to add `next_u16`
with a default implementation.

### 128-bit

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

Proposal: do nothing now. After the type is made stable, consider adding a
`next_u128` function to `Rng` with a default implementation in terms of
`next_u64`, but *only* if there is actual usage for the function.

## Constructing and seeding RNGs

Unlikely most data types, PRNGs do not have an "empty" or "initial" state. For
most uses, RNGs should generate distinct numbers each time, hence PRNGs require
*seeding*. There are two distinct types of seeding:

*   Using a fixed seed to yield the same output sequence deterministically
*   Using "fresh entropy" to output an unpredictable and unique number sequence

We use two distinct traits to provide this functionality:

*   [`SeedFromRng`] should be implemented by every PRNG to seed from another RNG
    (usually an external RNG providing fresh entropy)
*   [`SeedableRng`] allows seeding using a fixed seed, and should only be
    implemented by stable PRNGs

### Seeding with fresh entropy

We add the following trait, and recommend that PRNGs implement this but do not
implement [`Rand`]:

```rust
pub trait SeedFromRng: Sized {
    fn from_rng<R: CryptoRng>(rng: R) -> Result<Self, Error>;
}
```

This allows seeding with fresh entropy from an external RNG like [`OsRng`] as
well as from master RNG. For convenient seeding with fresh entropy, we also
provide [`NewSeeded`] (see later).

`from_rng` returns a `Result` simply to allow forwarding errors from
`try_fill_bytes`.

#### Why require a `CryptoRng` parent?

Seeding some non-crypto PRNGs from a parent PRNG of the same type can
[accidentally make one a clone of the other](https://play.rust-lang.org/?gist=6c12ea478440e452b135a6354024a909&version=stable).
This accidental cloning is impossible for even the weakest crypto RNGs (because
there should be no trivial way to predict future output from past output).
Additionally, restricting the parent RNG to a `CryptoRng` ensures the derived
seed will be of high quality and provides a strong guarantee that all generated
child PRNGs are independent (if they weren't, this would provide a way to
predict something about future output of the `CryptoRng` from past output, which
is not supposed to be possible).

Unfortunately there is a disadvantage: most of the time seeding a non-crypto
PRNG from another non-crypto PRNG causes no significant issues, so this
prevents users from using a very small, fast parent PRNG (without hacky wrapper
types). On the other hand if a user *is* generating many PRNGs from a master
PRNG, the performance and memory requirements of the master PRNG is unlikely of
concern (especially since there are quite fast CSPRNGs anyway).

There is another disadvantage: `thread_rng` is commonly used to seed other
PRNGs. `thread_rng` currently uses ISAAC, which awkwardly is
unpredictable-but-not-provably-secure. To avoid giving users a tricky problem
to solve we should mark `thread_rng` as a `CryptoRng`, however, despite the
[outstanding prize on offer](http://burtleburtle.net/bob/rand/isaacafa.html)
(since 1998) for an attack against ISAAC, it seems inappropriate to implement
`CryptoRng` for it. This puts us in a slightly tricky situation. One solution
would be to add another trait like `NonTrivialRng` as a half-way mark to
`CryptoRng`, but this is messy. Another would be to
[replace ISAAC with a better reviewed generator](https://github.com/dhardy/rand/issues/53).
If we do not resolve this, we should allow `from_rng` to use any parent
implementing `Rng` (TODO).

#### Why not `Rand`?

Given that [`Rand`] is already implemented by existing generators, why change
things? First, crate separation: `Rand` is part of the `rand` crate and to be
effective must have a whole bunch of implementations for built-in and `std`
types within the same crate; we wish to allow RNG implementations using only
the much smaller `rand-core` crate. Second, documentation: having a trait
expressely for seeding-from-RNGs allows us to document how it should be
implemented and potential pit-falls for users. Third, PRNGs are a little bit
special and it's useful to be clear about when new instances are created;
`SomeRng::from_rng(parent_rng)` is clearer than `parent_rng.gen()`. Fourth,
this allows the requirement to use a `CryptoRng` parent as explained above.

There is a significant drawback to this: breaking code for existing users. But
this is only an issue where users explicitly create a new PRNG themselves
rather than use `thread_rng()` or `weak_rng()`, and is easy to fix.

### Deterministic seeding

Discussion topic: [issue #18](https://github.com/dhardy/rand/issues/18).

`rand` currently provides this [`SeedableRng`] trait:

```rust
pub trait SeedableRng<Seed>: Rng {
    fn reseed(&mut self, Seed);
    fn from_seed(seed: Seed) -> Self;
}
```

We propose replacing this with:

```rust
pub trait SeedableRng: Rng {
    type Seed: From<SomeHash>;
    
    fn from_seed(seed: Self::Seed) -> Self;
    
    fn from_hashable<T: Hashable>(x: T) -> Self {
        let seed = SomeHash::hash_fixed(x).into();
        Self::from_seed(seed)
    }
}
```

Here:

*   `SomeHash` is an as-yet undecided hash function (must be fixed)
*   `From<SomeHash>` finalizes the hash state and converts to output of one of
    the following types: `[u8; 8]`, `[u8; 16]`, `[u8; 32]`. This constrains
    which seed types can be used.

#### `Seed` as an associated type

The current parameterised trait has a significant draw-back: generic code cannot
get the seed type or its size, thus generic code can't use
`SeedableRng::from_seed` for a parameterised type `R: SeedableRng`.
Since a single seed type should be sufficient for generic usage, we can fix
that by using an associated type, and asserting that the type used must be
`Sized` (i.e. not a slice).

We require the seed to be a byte-slice (e.g. `[u8; 8]` not `u64`) because this
avoids endianness issues, helping avoid portability problems.

Note that certain generators may wish to provide other constructors allowing the
seed to be specified in various ways; e.g. `ChaChaRng` could have multiple
constructors similar to [Peter Reid's ChaCha library](https://github.com/PeterReid/chacha/blob/master/src/lib.rs#L93).
These constructors, however, would be specific to the generator and would likely
require specific names, thus it seems pointless trying to support them in
`SeedableRng`.

#### Seeding from a `Hashable`

To properly seed a PRNG for cryptographical uses, one should use `from_seed`
with a strong seed or `from_rng`. For uses where a crypto-strength seed is not
required, we provide `from_hashable` as a convenient way to seed a PRNG with a
seed with good bit-distribution. This function achieves two things at once:

*   Allowing seeding from many different types of input, e.g. strings and simple
    numbers
*   Converting input of arbitrary length and possibly highly biased
    bit-distribution to a seed of the required length with good bit-distribution

In theory good PRNGs should produce high-quality random numbers with any seed,
but this is not always the case. Some PRNGs (e.g. MT19937) initially produce
low-quality output when using a seed which is mostly zeros, and some PRNGs (e.g.
Xorshift and Xoroshiro) explicitly fail when the seed is zero. By using a hash
function to produce a seed with good bit-distribution and good avalanche (small
changes to input cause large changes to output) we reduce the chance of users
providing weak input (e.g. a key phrase from a user) getting statistically poor
output, and we also allow the whole input to have an effect on the output.

The hash function `SomeHash` will be fixed in the code, since it is intended to
be used where reproducible seeds are required. We also make it public and the
body of `from_hashable` very simple to allow users to generate compatible seeds
in their own code, if desired.

Note that this function is not intended for cryptographic uses, e.g. converting
a password to a secure sequence of random numbers. Specific hash functions
like Argon2 exist for passwords; besides being cryptographically secure
(meaning recovering input from output or finding clashes is computationally
infeasible), these are designed to be slow and/or memory intensive to make
brute-force attacks hard, and potentially also to make sideband attacks looking
at memory access patterns ineffective. The hash function for `from_hashable` is
not required to have any of these properties.

Hash functions currently under consideration: MetroHash, SeaHash, HighwayHash.

#### Removing `reseed`

The `reseed` and `from_seed` functions do exactly the same thing except that
`reseed` requires an existing implementation. (In fact, they are required to
yield the same result: a given PRNG seeded with a given seed should always
produce the same output.) `from_seed` is significantly more useful, so we
propose removing `reseed`.

On a related topic, a function to "mix fresh entropy into an existing PRNG"
could be added somewhere (potentially to `SeedableRng`), but it should *not* be
named `SeedableRng::reseed` to avoid confusion. There has been some discussion
around such a function, but little attempt to add one, in part because similar
functionality can be achieved without it (e.g. generate two seeds, one using the
current PRNG and one using an external RNG, then XOR the two and seed from the
result).

#### Streams

One thing this trait does miss is support for explicitly selecting from
[multiple streams]. Explicit selection of stream may not be very important
however, as the primary use for multiple streams would appear to be reducing
the probability of two randomly seeded generators having any overlap in their
output sequences (assuming long sequences of output are used), and this can be
achieved by using part of the seed to select the PRNG stream.

#### Implementation guidelines

[`SeedableRng`] should only be implemented by fixed generators
(i.e. where output is repeatable, cross-platform and it is not expected that the
algorithm will be adjusted in the future). This implies that `StdRng` should not
implement `SeedableRng` because the underlying generator may be changed; also,
output is platform-dependent (currently it may be `IsaacRng` or `Isaac64Rng`).

### Support function: seeding with fresh entropy

Often, PRNGs are seeded from "somewhat random" sources such as the system
clock. We try to make the best option easy by giving all PRNGs a `new()`
function which seeds with strong, fresh entropy.

Note that roughly the same functionality is already available in `rand`:

```rust
use rand::{Rng, OsRng, ChaChaRng};

// OsRng::new() returns a Result
let mut rng: ChaChaRng = OsRng::new().unwrap().gen();
```

We wish to make this slightly easier:

```rust
use rand::{ChaChaRng, NewSeeded};

// new() returns a Result
let mut rng = ChaChaRng::new().unwrap();
```

Here, `NewSeeded` is a trait providing just the `new` function. It is
automatically implemented for any type implementing `SeedFromRng`:

```rust
/// Seeding mechanism for PRNGs, providing a `new` function.
/// This is the recommended way to create (pseudo) random number generators,
/// unless a deterministic seed is desired (in which case the `SeedableRng`
/// trait should be used directly).
#[cfg(feature="std")]
pub trait NewSeeded: SeedFromRng {
    fn new() -> Result<Self>;
}

#[cfg(feature="std")]
impl<R: SeedFromRng> NewSeeded for R {
    fn new() -> Result<Self> {
        ...
    }
}
```

`NewSeeded` is essentially just a function, provided as a trait to allow
`MyType::new()` syntax. It cannot be overridden by users. Internally it uses a
strong entropy source (`OsRng` or, as a fallback, the new `JitterRng`) and
constructs the PRNG via `from_rng`.

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
    intuitive as possible: `MyRng::new()?`
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
    should be an extra layer built on top of (and independent from) `rand-core`

## Error handling
[error-handling]: #error-handling

Comment threads:
[issue #9 / error type](https://github.com/dhardy/rand/issues/9),
[issue #10 / error kinds](https://github.com/dhardy/rand/issues/10)

Assuming we do use a `Result` (and it's useful outside of `Rng` too, e.g. for
constructing an RNG), what should the error type be?

We wish to make the library `no_std` compatible, so the type cannot depend on
`std::io::Error`, or `Box` (in the future `Box` should be
available to some `no_std` environments, but depend on having an allocator). On
the flip side, `OsRng` implementations may have to handle a `std::io::Error`,
in which case *if* we don't want to throw away the details, our error type
either needs to be able to hold a *cause* of this type or hold the string
version, retrieved with type `&str` from `std::error::Error::description`.

We propose the following. Note that the `Error` type has public fields and is
directly constructible and deconstructible by users; this is by design. It is
unfortunate that `std::error::Error` does not support `PartialEq` or `Clone`,
but there is no requirement to deny `no_std` this functionality.

```rust
/// Error kind which can be matched over.
#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub enum ErrorKind {
    /// Permanent failure: likely not recoverable without user action.
    Unavailable,
    /// Temporary failure: recommended to retry a few times, but may also be
    /// irrecoverable.
    Transient,
    /// Not ready yet: recommended to try again a little later.
    NotReady,
    /// Uncategorised error
    Other,
    // no hidden members: allow exclusive match
}

#[cfg(feature="std")]
// impls: Debug, Display
pub struct Error {
    pub kind: ErrorKind,
    pub cause: Option<Box<std::error::Error>,
}
#[cfg(not(feature="std"))]
// impls: Debug, Display, Clone, PartialEq, Eq
pub struct Error {
    pub kind: ErrorKind,
    pub cause: Option<&'static str>,
}
```

##### `String`

We could give the `cause` type `Option<String>` or `Option<Box<str>>` for
`std`, capturing the output of `std::error::Error::description`.
Neither of these approaches are available in `no_std` and both depend on the
existance of an allocator, not a given on embedded systems, so there is little
reason to do this over capturing the whole source error.

##### `&'static str` only

We could use static strings only. Via [a trick](https://github.com/rust-lang/rfcs/pull/2106#issuecomment-327442573) it is possible
to cast a dynamically-allocated `Box<String>` to a `&'static str` and leak the
memory (no free) on recovery — but leaking memory is not generally recommended,
so likely this would mean not including a cause much of the time.

##### No cause

We could omit the cause altogether (or only in `no_std` mode). In theory it
should be clear from the context of which generator failed and the *kind* what
the problem is.


## Split into multiple crates

We propose splitting `rand` into multiple traits, so that the appropriate
subsets of functionality are available to the main sub-sets of users, without
too many extras. We consider the following usage classes:

*   implementations of RNGs
*   cryptographic applications using mainly byte sequences
*   numeric applications generating values using various distributions
*   other uses of random numbers

The first of these requires the `Rng` and/or `CryptoRng` traits, may use some
helper "impl" functions, and may use some extension traits, but shouldn't
directly require anything else (except for testing).

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

*   a new `rand-core` crate containing the `Rng` trait, `CryptoRng` (in whatever
    form(s) that takes), extension traits like `SeedableRng` and `SeedFromRng`,
    possibly a mock implementation of `Rng`, and functions to aid implementing
    `Rng`
*   the existing `rand` crate should re-export all the above except the helper
    functions so that most users will not be affected by this RFC

In the future, we may wish to introduce some more crates, as follows. This list
is provided for insight into the larger picture only; please do not use this RFC to
comment on these crates (use the [Rand crate revision RFC] instead):

*   a `rand-os` crate exposing `OsRng` and possibly `NewSeeded`
*   a `rand-chacha` crate for `ChaChaRng`
*   a `rand-isaac` crate for the ISAAC RNGs
*   other PRNG crates may be adopted, e.g. `pcg_rand`
*   possibly a `rand-thread` crate for `thread_rng` (very speculative)


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
about introducing `rand-core` and tweaking `Rng`.

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

# Alternatives
[alternatives]: #alternatives

## NonCryptoRng

It [has been suggested](https://github.com/rust-lang/rfcs/pull/2152#issuecomment-329804139)
that `Rng` be renamed to `NonCryptoRng`. With the current trait design, that
would imply `CryptoRng: NonCryptoRng` and every `CryptoRng` is also a
`NonCryptoRng`; this seems reasonable. On the other hand, this naming has two
disadvantages: (1) extra breakage from the current `rand` crate, and (2) giving
a commonly-used type a significantly longer and more complex name for what
appears to me a weak rationale. It is suggested that we might fall into the same
trap as C and make the easiest source of random numbers weak; but that is not
the case: the easiest sources are (and will probably remain) `OsRng::new()` (very
secure) and `thread_rng()` (not "crypto approved", but still hypothesised to be
quite strong and with no known attack; can be switched should a weakness be
found). Additionally, with `CryptoRng` right next to `Rng` and good
documentation, it will be hard to miss the conclusion that `CryptoRng` should
be preferred for cryptographic usage.

## Seeking

Some PRNG algorithms support seeking to an arbitrary position in their output
without having to generate all numbers in between (but not all). For example,
rand already has [`ChaChaRng::set_counter`]. This particular PRNG suggests
using `set_counter` to specify a nonce; another use would be to effectively
divide a PRNG's output into multiple streams.

Perhaps
we should add a trait supporting seeking, for documentation and to allow wrapper
types to automatically re-implement seeking. However, we don't yet have a good
design, so this may be better left until later, if ever. The following proposal
sort-of works, but is complicated and doesn't support the full 128-bit index
`ChaChaRng` allows.

```rust
pub enum SeekMode {
    /// Seek relative to the start of the stream
    Abs,
    /// Seek relative to the current position
    Rel,
    /// Only get the position
    Get,
}

pub enum SeekBlock {
    U8,
    U32,
    U64,
    U128,   // if we introduce next_u128
}

pub trait SeekableRng: Rng {
    /// Seek to a new position within the stream, then return the position in
    /// absolute terms (relative to the start).
    /// 
    /// `block` specifies units: `U8` implies bytes, `U32` the number of
    /// `next_u32` calls, etc. Specification is required because some generators
    /// skip some bits; e.g. `next_u32` may use 64 bits and `fill_bytes` may
    /// round up to the next 32 or 64 bit boundary (or other).
    /// 
    /// If the requested seek position is unavailable the generator may round
    /// up, skipping bits in the same way as `next_u*` and `fill_bytes` do.
    fn seek_to(&mut self, pos: isize, block: SeekBlock) -> usize;
}
```

Unfortunately this ends up being rather complicated since quite a bit of
functionality is wrapped into a single function. Using multiple functions would
make reimplementation by wrapper types more tedious, and default implementations
would not necessarily do the correct thing, in the same way that default
implementations of functions like `next_u64` could behave incorrectly in
wrappers.

Since we don't have a good solution, this is not a pressing problem, and a
trait can be added later without breakage, the best option is probably to leave
this out for now.

## Crate names: `-` vs `_`

As far as I am aware, both naming conventions have already been used for crates.
`rand_derive` uses the underscore syntax, as do a few other rand-related crates
and quite a few others (`lazy_static`, `num_cpus`, `thread_local`).
The dash syntax may be slightly more common among the most downloaded crates:
`regex-syntax`, `aho-corasick`, `num-traits`, `pkg-config`, `utf8-ranges`.
(This looks at only the 25 most downloaded crates. From the first 100,
I count 15 using `_` and 29 using `-`.)

A vote [showed a preference for `rand-core` over `rand_core`](https://github.com/rust-lang/rfcs/pull/2152#issuecomment-333884702).
The sample implementation was named `rand_core`, but according to this vote
should be published as `rand-core` instead on acceptance of this RFC.

## Function names: `fill_bytes`

Previously `try_fill_bytes` was called as `try_fill`; the rename is for
consistency [as pointed out here](https://github.com/dhardy/rand/issues/8#issuecomment-338633414).
In line with the comment, we could use the names `fill` and `try_fill` instead,
although this is an unnecessary breaking change. Or we could just use
`fill_bytes` and `try_fill`.

# Unresolved questions
[unresolved]: #unresolved-questions

The member functions of the `Rng` trait and extension by a marker-only
`CryptoRng` is a design which has been selected after a long look at various
options; please see the [Rand crate revision RFC] for the history behind this.
This particular design seems to reasonably well meet all design requirements
while remaining reasonably simple, but is not obviously *the best option*.

There are several suggestions for the semantic meaning of `CryptoRng` above.
The most requested seems to be "something suitable for cryptography" or
"well studied algorithms designed for cryptography", but this is a bit vague.

The `Error` type needs to be defined.

Should `SeedFromRng`, `NewSeeded` and `SeedableRng` all extend the `Rng` trait?
There seems no real reason for this aside from the name of the third implying
that implementations are `Rng`s, on the other hand all three are designed with
`Rng` in mind and may not have alternative uses. In theory `SeedFromRng` could
be used to randomly initialise buffers for example, but this can also be done
via the `gen()` function or `fill_bytes`.

Should we adapt `SeedableRng` for stream support?


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
