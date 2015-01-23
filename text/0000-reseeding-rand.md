- Start Date: 2013-01-23
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Stabilise the `std::rand` module by focusing it on a smaller set of
functionality, moving "fancy" functionality to a crates.io crate.

# Motivation

The `std::rand` module provides various facilities for working with
random numbers and randomness in general. This is a surprisingly
sensitive topic, with different use cases. `std` does not aim to
completely satisfy everything, but rather aims to provide a minimal
set of functionality that tries to be simple and straightforward
without restricting people building on it, and this reasoning extends
to `std::rand`.

A major area of use for randomness is cryptography where the
recommended approach is to use the operating system if at all
possible. There is no particular reason for `std::rand` to deviate
from this. However, other uses of randomness (e.g. simulations) do not
need the cryptographical strength, and prefer the much improved
performance one can get from using a weaker RNG. Similarly, a use-case
like seeding a hashmap to avoid algorithmic complexity DoS attacks
does not require a (possibly expensive) call into the operating system
for every `HashMap::new()` call; a high-quality user-space RNG with an
unpredictable seed (e.g. from the OS) is almost certainly good enough.

To justify the inclusion of non-OS interfaces, the following benchmark
was run on my Linux 3.16 system (before the `getrandom(2)` syscall was
added):

```rust
extern crate test;

use std::rand::{Rng,OsRng,ChaChaRng};

#[bench]
fn os(b: &mut test::Bencher) {
    let mut rng = OsRng::new().unwrap();
    let mut buf = [0; 1000];
    b.iter(|| {
        rng.fill_bytes(&mut buf)
        test::black_box(&buf);
    })
}

#[bench]
fn chacha(b: &mut test::Bencher) {
    let mut rng = ChaChaRng::new_unseeded();
    let mut buf = [0; 1000];
    b.iter(|| {
        rng.fill_bytes(&mut buf);
        test::black_box(&buf);
    })
}
```

Results with `-O`:

```
running 2 tests
test chacha ... bench:      4495 ns/iter (+/- 140)
test os     ... bench:     73136 ns/iter (+/- 6551)

test result: ok. 0 passed; 0 failed; 0 ignored; 2 measured
```

NB. the above (*only* 16&times; slower) is actually the case when
`OsRng` is going to perform best: reading a large chunk of bytes in
one go. Changing the `.fill_bytes` call to `.gen::<u32>()` makes `OsRng`
250&times; slower than `ChaChaRng`.

Relevant links for this RFC:

- [C++11's `<random>` header][cpp_random]
- [D's `std.random`][d_random]
- ["Investigate replacing the ISAAC Rng with directly using the operating system,
  or an eSTREAM Rng (etc.)"](https://github.com/rust-lang/rust/issues/10047)
- ["Should RNGs really be clone?"](https://github.com/rust-lang/rust/issues/20973)

[cpp_random]: http://en.cppreference.com/w/cpp/numeric/random
[d_random]: http://dlang.org/phobos/std_random.html

## Cryptographic security

The only interface that will be recommended as cryptographically
secure will be the RNG that interfaces with the operating system. This
will be crystal clear in the documentation.

# Detailed design

The stable-at-1.0 parts of the module will essentially just consist of
2 traits (plus one extension trait) and 3 RNG algorithms (plus one
wrapper).

## Traits

`std::rand` will include the following generic set-up;

```rust
/// Things that can be created randomly, where the creation can be
/// mediated by information contained in `Data`.
trait Random<Data = FullRange> {
    /// Create a random value of type `Self`
    fn random<R: ?Sized + Rng>(data: Data, rng: &mut R) -> Self;
}

// Example:

// create a raw u8 with no restriction.
impl Random<FullRange> for u8 {
    ...
}

// create a u8 between the bounds of range (same exclusivity properties as usual)
impl Random<Range<u8>> for u8 {
    ...
}


/// Random number generator.
trait Rng: io::Reader {
    /// Generate/fetch the next `u32` (no range restriction)
    #[unstable]
    fn next_u32(&mut self) -> u32 { ... }

    /// Generate/fetch the next `u64` (no range restriction)
    #[unstable]
    fn next_u64(&mut self) -> u64 { ... }
    /// Generate/fetch the next `f32` in the range [0,1)
    #[unstable]
    fn next_f32(&mut self) -> f32 { ... }
    /// Generate/fetch the next `f64` in the range [0,1)
    #[unstable]
    fn next_f64(&mut self) -> f64 { ... }
}

impl<'a, T: ?Sized + Rng> Rng for &'a mut T {}
impl<T: ?Sized + Rng> Rng for Box<T> {}

/// Basic extension functionality to ensure that `Rng` is object-safe.
trait RngExt {
    fn gen<T, D>(&mut self, data: D) -> T where T: Random<D> { ... }

    /// Return an iterator that generates an infinite sequence of random values of type `T`.
    fn gen_iter<T, D>(self, data: D) -> Generator<T, D, Self> where T: Random<D> { ... }

    /// Randomize the order of the elements of `values`
    fn shuffle<T>(&mut self, values: &mut [T]) { ... }
    /// Select one value uniformly at random from the slice.
    #[unstable]
    fn choose<'a>(&mut self, values: &[T]) -> Option<&'a T> { ... }
    /// Select `n` values uniformly at random from the iterator.
    fn sample<I: Iterator>(&mut self, n: uint, values: I) -> Vec<I::Item> { ... }
}

impl<T: Rng> RngExt for T {}
```

As today, the relationship between output of the methods of `Rng`, and
between them and `io::Reader` is unspecified.

### The `Random` trait

The change to `Random` (currently `Rand`) is motivated by allowing
`gen` and `gen_range` to be unified, what was previously
`rng.gen_range(a, b)` can become `rng.gen(a..b)`. However, this makes
the non-ranged generation slightly more complicated/"magic
incantation"-y: `rng.gen(..)` (this relies on having explicit syntax
for `FullRange`, per [RFC 702]).

[RFC 702]: https://github.com/rust-lang/rfcs/pull/702

The convention for using `FullRange` for unrestricted generation may
seem peculiar for types that don't have any concept of ranges,
e.g. creating a randomly seeded RNG, but the consistency and
"vagueness" of `..` likely means this is OK.

(NB. general discussion, non-normative) This approach allows us to
naturally leverage any "range combinators" used elsewhere,
e.g. inclusive endpoints, and so remove the need for the `Closed01`
and `Open01` types. It also allows us to implement `Random` for
e.g. `Vec`, say, `rng.gen::<Vec<u8>>(10..20)` would create a vector of
some length between 10 and 20 and fill it with `u8`s.

#### `#[derive]`

The deriving mode will be removed from the compiler, or at least,
disabled on the stable channel; it can be rarely used since most data
types have some restriction on their values and/or non-trivial
relationships between them, which a derived `Random` implementation
can do nothing to preserve.

#### Alternatives

Call it `Rand` as it is today.

Not introduce the `Data` parameter. This would be simpler for this
trait, but require duplication elsewhere.

Have either `Self` or `Data` be associated types. This would require
more proxy types since e.g. the following is illegal:

```rust
impl Random for u8 {
    type Data = FullRange;
    // ...
}
impl Random for u8 {
    type Data = Range<u8>;
    // ...
}
```

### The `Rng` trait

[the `Rng` trait]: #the-rng-trait

This trait represent random number generators, it is mostly provided
for expressing intent, and for optimisation purposes over a plain
`Reader`.

It is expected and recommended that types implementing `Rng` generates
a sequence of bits where each bit:

- has equal probability of being either 0 or 1,
- is independent of all other bits.

Other functions will be assuming these properties to give sensible
answers (e.g. `shuffle`, `choose` and `sample`) but there's no way any
function can rely on them and cause unsafety only when violated, so
this is not overly problematic.

Currently this trait is not object safe, but like `Iterator` and
`Reader` (etc.) it may be desirable to abstract over many different
RNG types, so making it object safe is desirable.

The methods provided are all optional, implemented in terms of
`Reader::read`, e.g. as a sketch (no [error handling])

```rust
fn read_u32(&mut self) -> u32 {
    let mut buf = [0u8; 4];
    self.read(&mut buf);

    ((buf[0] as u32) << 24) |
    ((buf[1] as u32) << 16) |
    ((buf[2] as u32) << 8) |
    ((buf[3] as u32) << 0)
}
```

They are provided because most RNG types naturally generate one of
`u32`, `u64`, `f32` or `f64`. Using the `Reader` implementation
directly to retrieve one of these types (or a value that builds on
these types) will have to convert from the natural type into bytes and
then from bytes back to the natural type, and so providing these
methods (which `Random` implementations can call) removes this
overhead.

The choice of these 4 types to specialize to is designed to tailor to
the simulation use-case (many cryptographic uses will be quite happy
with the bytes interface provided by `Reader`). Those types are the
most prominent in simulations, and there are generators that produce
values of those types directly. However, many generators can
efficiently produce more that this, e.g. the [ChaCha] algorithm can
very naturally produce 128-bit `i32x4` SIMD vectors (and equally
naturally produce up to 512-bit AVX512 vectors), and [dSFMT] naturally
generates `f64x2` vectors. Future algorithms are likely to be designed
for use with SIMD so it may be desirable to provide some way to
special case more than just those 4 types.

[ChaCha]: http://en.wikipedia.org/wiki/Salsa20#ChaCha_variant
[dSFMT]: http://www.math.sci.hiroshima-u.ac.jp/~%20m-mat/MT/SFMT/index.html#dSFMT

These methods are optional, and users are recommended to call
`RngExt::gen` instead of these, so they are tentatively marked
`unstable` in case an alternative approach is devised (they could also
just be stabilised as is, since it is presumably desirable for the
current four types to be efficient; any other types that we find that
need this sort overloading can be added as default methods in future).


#### Alternatives

Inherit from `Iterator<u32>` instead of `Reader`, replacing `next_u32`
with a substitute for `read` (the current `Rng` calls it
`fill_bytes`). Most PRNG algorithms will find it annoying to implement
`Reader` (they are defined in terms of `u32` or larger, not bytes),
and, to retain efficiency, there may be some surprising behaviour with
`Reader`, e.g.

    let mut a = [0, 0];
    rng.read(&mut a);

    let mut b = [0, 0];
    rng.read(&mut b[0..1]);
    rng.read(&mut b[1..2]);

will give different results. That said, this is inherently expected
with randomness: a true random number generator can not be relied on
at all.

### The `RngExt` trait

[the `RngExt` trait]: #the-rngext-trait

`RngExt` provides basic useful functionality on RNG. Users will almost always
want to `use` this trait (not `Rng`) into scope.

`self.gen(data)` is a thin wrapper around `Random::random` as exists
today, i.e. `Random::random(data, self)`.

`rng.gen_iter(data)` returns an infinite iterator of random values,
so that `for x in rng.gen_iter::<T>(data) { ... }` is like `loop { let
x = rng.gen::<T>(data); ... }`. This method is unusual and consumes
`self` so that the returned value is maximally flexible, e.g. it can
be returned from the stack frame in which the RNG is created
(currently it takes `&'a mut self` and returns a `Generator<'a, ...`),
however non-consuming behaviour can be achieved by using the
implementation of `Rng` for `&mut R` `R: Rng`.

`rng.shuffle(values)` reorders the data in `values: &mut [T]`. If
`rng` is a properly uniform RNG then any particular ordering occurs
with probability `1/values.len()!`. This method could be more general:
the restriction for the values to be contiguous in memory (`[T]`) is
not necessary (e.g. it could perfectly well reorder the elements of a
`RingBuf`), but `std` does not have the required abstraction to make
it generic now. However, it is likely that it could be made generic in
future, backwards-compatibly.

The last methods are `rng.choose(values)` and `rng.sample(n, values)`.
They are unfortunately similar: `rng.choose(values)` is (semantically)
just `sample(1, values.iter())` without the `Vec` allocation. It would
be extremely nice to reconcile them but this does not seem feasible at
the moment. NB. by taking a slice, `choose` is O(1): it generates a
random number in-bounds of the slice and just indexes. Laying out the
iterator-based options: taking an fully generic iterator would require
traversing the full iterator while generating a random number at each
step (like `sample`), taking an `ExactSizeIterator` would reduce this
to a single random number in total and allow truncating the traversal,
andlastly taking a `RandomAccessIterator` would give the same indexing
behaviour as a slice. The RFC author is unsure about the best path
here, but notes that `Iterable` may make it possible to generalise
this backwards-compatibly in future.

The current `Rng` trait has two pieces of functionality that will be
removed (or at least made `unstable`):

- `rng.gen_weight_bool(n)`, this is trivially `rng.gen(0..n) == 0`.
- `rng.gen_ascii_chars()`, this seems relatively specialised, and the
  full range can be simulated with `rng.gen_iter('0'..'{')` (NB. `{`
  is the char after `z`, necessary due to exclusivity of the RHS)
  which unfortunately picks up some punctuation in the middle, but
  most use cases will be perfectly well served by just
  `rng.gen_iter('a'..'{')`. We could also provide extra
  implementations, such as `struct PrintableAscii; impl
  Random<PrintableAscii> for char`, allowing
  `rng.gen_iter(PrintableAscii)`.

#### Alternatives

This trait doesn't need to exist, everything could be free functions
in the `rand` module.

There could be a `fn by_ref<'a>(&'a mut self) -> ByRef<'a, Self>`
function, like occurs for iterators and in IO, to provide a real type
backing up `impl<'a, T: ?Sized + Rng> Rng for &'a mut T {}`.

## Free functions

[free functions]: #free-functions

Top level convenience functions are provided:

```rust
fn thread_rng() -> ThreadRng {
    // fetch the cached RNG from TLS, or create it if it doesn't exist
}

fn random<T: Random<D>, D>(data: D) -> T {
    thread_rng().gen(data)
}
```

### Alternatives

There is valid concern that the choice of RNGs described below leaves
users looking at jargony names for RNGs, which no particularly obvious
default choice; possibly leading to poor selection. Functions like:

```rust
fn secure_rng() -> OsRng { ... }
fn balanced_rng() -> ChaChaRng { ... }
fn weak_rng() -> XorShiftRng { ... }
```

could be provided to give some guidance. As a counterpoint, the
`collections` library does not provide `ordered_map() -> BTreeMap`,
`unordered_map() -> HashMap`.

## Error handling

[error handling]: #error-handling

As it stands, there is no error handling at all, which is
unfortunate. Most PRNGs do not have any error conditions (some define
reaching the end of the sequence and repeating as an error), but the
interface with the operating system may.

Reconciling this is similar to IO error handling, where some readers
and writers are guaranteed to not return errors (e.g. reading from
memory). This consideration is postponed until the design in
[RFC 576](https://github.com/rust-lang/rfcs/pull/576) is definite, but
it would likely be along the lines of making each function return
`Result<..., <Self as Reader>::Error>`. instead of just `...`. This
may have unfortunate ergonomic impact, and may be more complicated if
building on `Iterator<u32>`.

## Provided RNGs

[provided rngs]: #provided-rngs

The module will provide 3 basic algorithms to try to provide more clarity:

- `OsRng`, a "true" random number generator calling into the operating system,
- `XorshiftRng`, an [xorshift][xorshift] random number generator (or
  one of the variations), for extremely high performance random
  numbers,
- `ChaChaRng`, a [ChaCha] random number generator, for "secure"
  user-space random numbers.

[xorshift]: http://en.wikipedia.org/wiki/Xorshift

### Algorithms

#### `OsRng`

An *unbuffered* wrapper around the best available operating system
primitive, e.g.

- `CryptGenRandom` on Windows,
- `getrandom(2)` on recent Linux versions,
- `/dev/urandom` on OSX, BSD, and older linux versions.

(See [the current `std::rand`][current] docs for more info and
discussion about this, particularly `/dev/urandom`. NB. to future
readers, the information will still be available in the docs after
this RFC lands, but may have moved.)

[current]: http://doc.rust-lang.org/nightly/std/rand/#cryptographic-security

The lack of buffering by default means every call will interact with
the operating system, making it very slow; but this is the only way
that the RFC author is comfortable to call it cryptographically
secure, adding complications is risky e.g. naively buffering would not
be cryptographically secure due to `fork` sharing memory.

This kind of functionality is sometimes called "random device", as a
possible alternative name.

#### PRNGs

Only two user-space pseudo-random number generators are provided:
[ChaCha] and [Xorshift][xorshift].

This specifically means the ISAAC and ISAAC64 implementations will be
moved out of `std::rand`. Compared to ISAAC*, the ChaCha RNG is much
simpler, has had deeper cryptanalysis and is designed with SIMD
optimisations in mind so is more suited to being part of a standard
library.

These two types can be deterministically seeded and would expose this
functionality as inherent methods (not trait implementations), and
would implement `Random` to "optimally" randomly seed themselves from
another source of randomness.

For example, the stable public interface for `ChaChaRng` (and
`XorShiftRng`) will look like:

```rust
pub struct ChaChaRng { ... }

impl ChaChaRng {
    pub fn from_seed(seed: &[u32]) -> ChaChaRng { ... } // seed: [u32; 4] for XorshiftRng
}

impl Random<FullRange> for ChaChaRng {
    ...
}
impl Reader for ChaChaRng {
    ...
}
impl Rng for ChaChaRng {
    ...
}
```

These names are chosen to be somewhat vague: `ChaChaRng` is actually,
currently, an implementation of the ChaCha20 algorithm, but the "20"
value could be able to be customised backwards-compatibly in future,
with default type parameters. (Similarly Xorshift is strictly
Xorshift128, but has parameters that could be tweaked.) E.g. in
hypothetical syntax

```rust
struct ChaChaRng<static N: u32 = 20> {
    /* state */
}
let default: ChaChaRng = ...;
let faster: ChaChaRng<8> = ...;
```


#### Alternatives

As per the Alternatives section of the [free functions] discussion,
these could be named `SecureRng`, `BalancedRng` and `WeakRng` (or have
wrapper types of those names). However, the precedent is to not do
this, both in other parts of the Rust library (e.g. `collections`) and
in other random libraries, e.g. [C++11's `random`][cpp_random] and
[D's `std.random`][d_random] both use the algorithms' names.

Note, `XorShiftRng` is renamed to `XorshiftRng` since the "Xorshift"
capitalisation is the correct one.

### Thread RNG

There will be one other RNG-implementer: the thread RNG. This RNG is
stored in thread-local storage (TLS) to avoid having to reinitialise
it each time, it will be defined something like

```
struct ThreadRngInner {
    inner: ChaChaRng,
    generated_entropy: usize,
}
pub struct ThreadRng {
    inner: Rc<RefCell<ThreadRngInner>>
}
```

so that the internal state is shared between all instances of the
random number generator.

The `generated_entropy` field will record the number of bytes output
by the RNG, and the whole `ThreadRng` will be reseeded from the
operating system once this passes a certain threshold.

#### Alternatives

Do not regularly reseed from the operating system: only the OS RNG is
designed for use in cryptographic circumstances and so the overhead
may not be useful. Also, reseeding like this restricts the ability for
a program to have forcibly deterministic execution by seeding the
`ThreadRng`. Changing this is backwards compatible.

This may make more sense to exist in `std::thread` or
`std::thread_local`.

### Alternatives (for "Provided RNGs")

Repeating `Rng` in ever name may not be necessary, a scheme like the
following may be interesting:

- `Os`, `RandomDevice`, `Secure`, `SecureRng` (or just `OsRng`)
- `ChaCha`
- `Xorshift`, `XorShift`
- `std::rand::ThreadLocal` (a little confusing), `std::thread_local::Rng`

We currently provide a `StdRng` type as an alias for another
generator, this type disappears in this RFC, but could be reintroduced
as a direct wrapper for `ChaChaRng`.

## `Copy` and `Clone`

Duplicating a random number generator can lead to generating
correlated sequences of random numbers, which may not be a good idea.

Random number generators should only implement `Copy` if copied values
generate independent/uncorrelated sequences. Notably, this does not
apply to PRNGs where the full internal state is stored inline, where
copied values will have duplicated the state and hence all generate
exactly the same sequence. This trait is particularly dangerous due to
the implicit nature of the duplication.

Random number generators can implement `Clone` in a way that
duplicates the state and causes identical/correlated sequences. In
fact, having identical sequences is likely one of the most valuable
uses of this, to e.g. replay/re-record failing test runs. (D's
`std.random` offers `save` on many RNGs for this purpose, and C++'s
`<random>` has copy constructors implemented for most types.)

### Alternatives

Provide a non-generic way to duplicate an RNG, e.g. a manual `fn
save(&self)` on each type. This gets annoying if the RNG has to be
used in generic code in any way, although it is somewhat questionable
if generic code duplicating an RNG is correct (that said, it could
just be code that is generic over different RNG types, e.g. a general
framework for rerunning/recording tests as mentioned above).

## `std::rand::distributions`

Moved from the standard library into a crates.io package.

## Omissions

The following is a (non-exhaustive) list of things which are explicitly not included in
this cut-down version of `std::rand`:

- Generically handling RNGs more deeply, e.g.:
    - Seeding
    - Splitting RNGs to run in parallel, uncorrelatedly
    - Fast-forwarding/rewinding (e.g. `ChaChaRng` provides
      `set_counter` to jump around in the output sequence, and it can
      be done efficiently with `XorShiftRng` via matrix exponentiation.)
- Many common and historical RNG algorithms, e.g. MT19937, linear
  congruence RNGs.
- "RNG adaptors" like `discard_block_engine`, `independent_bits_engine`
  in C++11's `random` header.
- Detecting process forks and providing user-space RNGs the tools to
  deal with it. A PRNG (almost always) stores its entire state in
  memory, and a forked process will receive a copy of this state,
  meaning the RNG in the main process and the one in the forked
  process will generate the same sequence.
- Non-uniform random numbers (e.g. what will have been `std::rand::distributions`)

# Drawbacks

- Losing functionality is sad.
- `gen_iter(a..b)` (i.e. explicit end-points) will be surprisingly
  inefficient for integers: to ensure uniformity and avoid bias and
  maintain efficiency it has to approximately do rejection sampling
  (after some precomputation). The author cannot see a way that this
  can be addressed for the `..` notation specifically without
  introducing more complexity (e.g. provide a way to generate a
  "cached" version of the precomputation which can then be used for
  generating every value in the iterator).
- `gen_iter` taking by value may encourage users to call
  `rng.clone().gen_iter()` instead of `(&mut rng).gen_iter()` if they
  need it to not be consumed.

# Alternatives

Listed inline.

# Unresolved questions

- Both C++ and D use 'engine' instead of 'generator', but literature
  generally seems to use 'generator'. Which term suits Rust best? The
  author(s) of C++'s `<random>`
  [suggests "random bit generator" (`Rbg`)][rng-is-hard],
  although this seems quite non-standard.
- Is `Random` taking a type parameter in the form that it does worth the complications?
- Should `rng.gen::<f64>(..)` i.e. "FullRange" really only be
  `[0,1)`? (This is *very* strongly the convention for the default floating-point generation range.)
- Is the exclusivity of the RHS of the `..` syntax too confusing for types like `char`?

- `choose` is peculiar, and is unfortunate to take a slice when an iterator could also work.

- Does it really make sense for RNGs to be `Clone`?

- Maybe this RFC conflates pure randomness (i.e. random bits) and distributions/random variates with the `Data` parameter to `Random`, see [the "RBG" paper][rng-is-hard]
  for info on that too.


[rng-is-hard]: http://www.open-std.org/jtc1/sc22/wg21/docs/papers/2014/n3847.pdf
