- Feature Name: rand-crypto
- Start Date: 2017-08-18
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Background

This RFC is an offshoot of the [Rand revision RFC], on the subject of:

*   splitting rand into multiple crates
*   splitting the current `Rng` trait into a crypto-oriented trait and
    numerical-application-oriented trait
*   impl of above trait(s) for OS-provided randomness
*   naming conventions for above things

# Summary
[summary]: #summary

Create a new crate, `crypto-rand`, with only the following contents:

```rust
pub enum Error {
    pub Unspecified,
}

pub trait CryptoRng {
    fn try_fill(&self, dest: &mut [u8]) -> Result<(), Error>;
}
```

Associated with this, create another crate called `crypto-rng-os` which defines
a struct `OsRng`, implementing `CryptoRng` via OS functionality (using code from
[rand/os.rs](https://github.com/rust-lang-nursery/rand/blob/master/src/os.rs)
and/or
[ring/rand.rs](https://github.com/briansmith/ring/blob/master/src/rand.rs).

This RFC does not directly propose adding other implementations of `CryptoRng`,
but if any are written to be shared, they can be placed in a small crate named
`crypto-rand-NAME`.

All crates should be adopted into
[rust-lang-nursery](https://github.com/rust-lang-nursery/) or (eventually)
[rust-lang](https://github.com/rust-lang), or another community-maintained
collection of repositories. This should prevent crates from becoming orphaned.

Finally, add an impl for `rand::Rng` of `CryptoRng` so that the
`rand` crate can use this `OsRng`. (Alternatively, `rand` could create a wrapper
`rand::OsRng` maintaining the current functionality. This is beyond the scope of this RFC.)

`rand::Rng` is not the subject of this RFC, but it is assumed here that this
trait will have an API like the following:

```rust
pub trait Rng {
    fn next_u32(&mut self) -> u32;
    fn next_u64(&mut self) -> u64 { /* default impl here */ }
    // possibly also next_u128
    
    // possibly also the following (no Result unlike try_fill above):
    fn fill(&mut self, dest: &mut [u8]) { /* default impl here */ }
}
```


# Motivation
[motivation]: #motivation

The [Rand revision RFC]
brings up roughly this design several times:

*   https://github.com/rust-lang/rfcs/pull/2106#issuecomment-322329159
*   https://github.com/rust-lang/rfcs/pull/2106#issuecomment-323320722
*   https://github.com/rust-lang/rfcs/pull/2106#pullrequestreview-56252714
*   https://github.com/rust-lang/rfcs/pull/2106#issuecomment-323326062

The appeal of this split is that cryptographic applications can use exactly the
parts of `rand` they need, and crypto-algs have a very minimal trait filling a
buffer and returning a result.

Currently the [ring crate](https://github.com/briansmith/ring) implements its
own version of [rand](https://github.com/rust-lang-nursery/rand)'s `Rng` and
`OsRng` since it desires a slightly different interface. With this change, the
"OS RNG" code need not be duplicated. Further, cryptographic PRNG
implementations can be used by cryptographic-specific code as well as used by
numerical applications through the implementation of `rand::Rng` for `CryptoRng`.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Random number generation is a complex topic, and usage can be categorised into
two main groups: cryptographic applications requiring secure *keys*, and
non-cryptographic applications (which can be further divided into at least
randomised algorithms, stochastic simulations, and games). For cryptographic
applications, we recommend using a well-reviewed library; these libraries may
or may not choose to use `crypto-rand` and `crypto-rng-os` to generate their
random numbers. For non-cryptographic applications, users may make use of the
`rand` crate [etc.]

[Note that the `cryto-rand` and `crypto-rng-os` crates are only intended to be
used directly by advanced cryptograhic users, hence the guide need not be very
detailed on the subject, aside from a short preamble and link to the `rand`
crate.]

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The `CryptoRng` trait and `Error` enum are specified above. We should make it
clear that although `Error::Unspecified` is directly accessible, `Error` could
be extended in the future (is formal specification of this possible?).

The `crypto-rng-os` crate should contain a single public member:

```rust
pub struct OsRng {
    // private internals
}

impl OsRng {
    pub fn new() -> OsRng {
        // ...
    }
}

impl CryptoRng for OsRng {
    // ...
}
```

The API of the `rand` crate itself is not the subject of this RFC, but this
RFC can be implemented without any breaking changes to `rand` (by creating a
wrapper type for `OsRng`, even if this gets removed again in the near future).


# Drawbacks
[drawbacks]: #drawbacks

See alternatives below.

# Rationale and Alternatives
[alternatives]: #alternatives

We may prefer *not to have multiple RNG traits*; Aaron Turon
[advocates avoiding a hard split](https://internals.rust-lang.org/t/crate-evaluation-for-2017-07-25-rand/5505/57).
In this case, the single generator trait could still be placed in its own crate,
`crypto-rand` or `rand-core`. See [an alternative proposal for the `Rng` trait](https://github.com/rust-lang/rfcs/pull/2106#issuecomment-323511494).

Alternatively, we could have separate `Rng` and `CryptoRng` traits, but where
`CryptoRng: Rng`. [See joshjf's proposed design](https://github.com/rust-lang/rfcs/pull/2106#issuecomment-323388931).
In this case, possibly both traits should be in the `crypto-rand` crate or
possibly there should be no such separation of crates.

`Error` only has one member, `Unspecified`, since knowing the *reason* an RNG
fails is probably not useful (at best, it may be possible to differentiate
between transient failures and permanent failures, but it seems unlikely any
implementation is going to want to return a transient failure: Linux's
`/dev/random` is
the only interface I am aware of likely to have a transient failure, but under
normal usage it would simply block, and in any case we never use it).

Many other names could be used instead of `CryptoRng`: `Rng` (as in `rand`),
`SecureRandom` (as in `ring`), `Generator`, `CryptoGenerator`, ...
Brian Smith argues for the name `Generator` [here](https://internals.rust-lang.org/t/crate-evaluation-for-2017-07-25-rand/5505/49) and [here](https://github.com/rust-lang/rfcs/pull/2106#discussion_r133107790).

In line with [RFC #356], `OsRng` could simply be named `Generator` within the
`crypto_rng_os` crate. Personally I do not like the idea of using the same name
for a trait and its implementation. Another possible name would be `OsGen`.

The `crypto-rng-os` name may not be the best; however (a) having a common prefix
for random number generators and (b) a common prefix for crypto-rand crates
seems like a good idea. An alternative would be `crypto-gen-os`. Readers should
bear in mind that implementations of `rand::Rng` will likely also want crate
names; these could be simply `rng-NAME`. Alternatively, we could use the crate
names `csrand` and `csrng` or `csgen-*` (Cryptographically Secure Rand / Random
Number Generator / Generator) along with `rand` and `rng-*`, or perhaps `csrand`
and `csrand-*` along with `rand` and `rand-*`.


# Unresolved questions
[unresolved]: #unresolved-questions

Given that some implementations of `OsRng` require a file handle (e.g. to
`/dev/urandom` on older versions of Linux), it is questionable how this should
interact with threading and users who keep an `OsRng` object. Possibly the best
option would be for `OsRng` to internally use a single handle via `lazy_static`
guarded by a mutex for threads, thus avoiding using many file handles within
the same process.


[Rand revision RFC]: https://github.com/rust-lang/rfcs/pull/2106#issuecomment-323329253
[RFC #356]: https://github.com/rust-lang/rfcs/pull/356
