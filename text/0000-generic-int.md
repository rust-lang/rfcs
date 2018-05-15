- Feature Name: generic_int
- Start Date: 2018-05-15
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Adds the builtin types `uint<N>` and `int<N>`, allowing integers with an
arbitrary size in bits. For now, restricts N ≤ 128.

# Motivation
[motivation]: #motivation

## Bitfields

This feature will ultimately allow bitfields, which allow multiple fields in a
struct to be defined as a range of bits, rather than bytes. Here's an example
in C:

```C
struct MipsInstruction {
    int opcode: 6;
    int rs: 5;
    int rt: 5;
    int rd: 5;
    int shift: 5;
    int function: 6;
}
```

In this format, all the data we need (in this case, one particular kind of MIPS
instruction) is stored within 32 bits, but we don't have any particular way to
map these to fields. In today's Rust, if we wanted to represent this struct,
we'd have to create methods to grab the data for each field using bit shifts and
masking. Here's an example of what this looks like for the `rs` field:

```rust
fn get_rs(inst: u32) -> u32 {
    (inst >> 21) & !(!0 << 5)
}
fn set_rs(inst: u32, rs: u32) -> u32 {
    inst & (!(!0 << 5) << 21) | ((rs & !(!0 << 5)) << 21)
}
```

As you can see, getting the shift (`21`) and mask (`!(!0 << 5)`) is not as
obvious as you'd think. The shift is actually the sum of the widths of all of
the fields after `rs` (`5 + 5 + 5 + 6 == 21`), and the mask is actually
`0b11111`, where the number of ones corresponds to the size of the field. It's
very easy for a human to mess these up, whereas in this case, C does all of the
work for you.

This RFC actually doesn't solve these problems, but solves a subtler problem
that C doesn't even try to solve: what is the type of the `rs` field itself?

If we were to write out this hypothetical struct in Rust, we'd want something
like:

```rust
#[repr(bitfields)]
struct MipsInstruction {
    opcode: u6,
    rs: u5,
    rt: u5,
    rd: u5,
    shift: u5,
    function: u6,
}
```

Unfortunately, `u5` and `u6` aren't valid types in Rust, and C's way of doing
`u8: 5` sounds even less desirable. Luckily, with const generics, we have a
better solution: `uint<N>`:

```rust
#[repr(bitfields)]
struct MipsInstruction {
    opcode: uint<6>,
    rs: uint<5>,
    rt: uint<5>,
    rd: uint<5>,
    shift: uint<5>,
    function: uint<6>,
}
```

Additionally, methods can take or return `uint<5>` and `uint<6>` to statically
guarantee that a value does not have more bits than necessary. If a method to
set `opcode` for example took a `u8`, it's not clear whether the method should
simply ignore the upper two bits or if it should panic if they're non-zero.

As stated before, `#[repr(bitfields)]` is *not* proposed by this RFC and is not
necessarily the way this will work. However, `uint<N>` is what this RFC
proposes.

## Generalising code for integers

Right now, there's a *lot* of boilerplate for implementing methods for integer
primitives. The standard library itself is a great example; almost the entirety
of `core::num` uses some gnarly macros to define all sorts of traits for all the
integer types. One example is `Shl` and `Shr`, which are defined for not just
every integer type, but every *combination* of integer types. We could easily do
this with const generics instead:

```rust
impl<const N: usize, const M: usize> Shl<uint<M>> for uint<N> {
    type Output = uint<N>;
    #[inline]
    fn shl(self, rhs: uint<M>) -> unit<N> {
        // implementation
    }
}
```

This will probably also decrease compiler time as well, as the compiler simply
needs to understand one generic impl instead of 144 macro-generated impls.
Because `Shl` and `Shr` can also handle references, that multiplies our
generated impl count by four per trait, meaning that our macros generate 576
impls total!

## Enum optimisations

Using a smaller number of bits for a value also has the benefit of niche enum
optimisations. For example, `uint<7>` represents a single ASCII character, and
`Option<uint<7>>` can be stored in a single byte. Additionally,
`Result<uint<7>, E>` also takes one byte if `E` is zero-sized.

## Documentation

Having generic impls would drastically reduce the noise in the "implementations"
section of rustdoc. For example, the number of implementations for `Add` for
integer types really drowns out the fact that it's also implemented for strings
and `std::time` types, which is useful to know too.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Up until now, we've only seen *specific* integer types, like `u8` or `i32`. What
if we want to write a trait that works with all of them?

Let's go with a simple example. Let's say that we want to code our own `Default`
trait which returns one instead of zero. We'll define it like this:

```rust
pub trait MyDefault: Sized {
    fn my_default() -> Self;
}
```

Right now, with what we know, we have to copy-paste our impl for every integer
type. It would look something like:

```rust
impl MyDefault for u8 {
    fn my_default() -> u8 { 1 }
}
```

Except, we'd have to replicate this for every single integer type. If we're
clever, we could use a macro:

```rust
macro_rules! impl_my_default {
    ($($int: ident),*) => {$(
        impl MyDefault for $int {
            fn my_default() -> $int { 1 }
        }
    )*}
}
impl_my_default!(i8, u8, i16, u16, i32, u32, i64, u64, i128, u128, isize, usize);
```

But really, that's just making the compiler do the copy-pasting for us. Instead,
we're going to use the special types `uint<N>` and `int<N>` to generalise the
code for us. For this, we're going to use const generics, instead of type
generics. The end result looks like:

```rust
impl<const N: usize> MyDefault for uint<N> {
    fn my_default() -> uint<N> { 1 }
}
impl<const N: usize> MyDefault for int<N> {
    fn my_default() -> int<N> { 1 }
}
impl MyDefault for usize {
    fn my_default() -> usize { 1 }
}
impl MyDefault for isize {
    fn my_default() -> isize { 1 }
}
```

That's a lot better! As you'll notice, we still have to separately implement
code for `usize` and `isize`; that's because they're separate types altogether.
For example, on 64-bit systems, `usize` is technically just a `u64`, but
`uint<64>` refers to `u64`, not `usize`. It's unfortunate, but we've reduced our
number of impls from twelve to four, which is still a very big deal!

One other side effect of having `uint<N>` and `int<N>` is that we can represent
a lot more types than before. For example, `uint<7>` is just a seven-bit
integer, which we might use to represent an ASCII character. Don't be fooled by
these; we can't actually make `N` bigger than 128 (for now), and `uint<N>` is
almost always stored using `uint<N.next_power_of_two()>`. `uint<7>` is stored as
`u8`, `uint<10>` is stored as `u16`, `uint<48>` is stored as `u64`, etc. If you
store a value with a nonstandard amount of bits, you're not actually saving any
space, so, keep that in mind!

That said, there are a few cases where we might want to use one of these
weirdly sized types, as many data formats store values in terms of bits instead
of bytes to save space. Eventually, we'll have a way of representing these, but
not today.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## Primitive behaviour

The compiler will have two new built-in integer types: `uint<N>` and `int<N>`,
where `const N: usize`. These will alias to existing `uN` and `iN` types if `N`
is a power of two and no greater than 128. `usize` and `isize` remain separate
types due to coherence issues, and `bool` remains separate from `uint<1>` as it
does not have most of the functionality that integers have.

`int<N>` and `uint<N>` will have the smallest power-of-two size and alignment.
For example, this means that `uint<48>` will take up 8 bytes and have an
alignment of 8, even though it only has 6 bytes of data.

`int<N>` store values between -2<sup>N-1</sup> and 2<sup>N-1</sup>-1, and
`uint<N>` stores values between 0 and 2<sup>N</sup>-1. One unexpected case of
this is that `i1` represents zero or *negative* one, even though LLVM and other
places use `i1` to refer to `u1`. This case is left as-is because generic code
*should* expect sign extension to happen for all signed integers, and the
`N = 1` case is no different. It's valid to cast `int<N>` or `uint<N>` to
`int<M>` or `uint<M>` via `as`, and sign extension will occur as expected. This
conversion is lossless when converting `int<N>` to `int<M>` or `uint<M>` or
from `uint<N>` to `int<M>` or `uint<M + 1>`, where `M >= N`.

In addition to the usual casts, `u1` and `i1` can also be cast *to* `bool` via
`as`, whereas most integer types can only be cast from `bool`.

For the moment, a monomorphisation error will occur if `N > 128`, to minimise
implementation burden. This error can be lifted in the future, and if any future
`uN` or `iN` types are added, these should alias with existing `int<N>` and
`uint<N>` types. The main justification for this change is to allow the compiler
to continue to represent literals and other values using `u128` internally, and
to allow stabilising this feature *before* larger ints are allowed internally.

`uint<0>` and `int<0>` are allowed as constant zero types. While this is not
inconsistent for `uint<0>`, as zero fits in the range [0, 2<sup>n</sup>-1], it
*is* somewhat inconsistent for `int<0>`, as the expected range for `int<0>` is
-½ to ½. It is decided to make both of these a constant zero type but *not*
identical types, as the invariant
`<uint<N>>::default() == <int<N>>::default() == 0` is desired.

Integer literals can be automatically coerced to `uint<N>` and `int<N>`,
although generic `iN` and `uN` suffixes are left out for future RFCs. Because of
the restriction that `N <= 128`, compiler code can continue to store literals as
`u128`. When coercing literals to `uint<N>` or `int<N>`, bounds checks should be
done just like they are with regular integer literals.

Primitive operations on `int<N>` and `uint<N>` should work exactly like they do
on `int<N>` and `uint<N>`: overflows should panic when debug assertions are
enabled, but ignored when they are not. In general, `uint<N>` will be
zero-extended to the next power of two, and `int<N>` will be sign-extended to
the next power of two.

## Computations and storage

Because sign extension will always be applied, it's safe for the compiler to
internally treat `uint<N>` as `uint<N.next_power_of_two()>` when doing all
computations. As a concrete example, this means that adding two `uint<48>`
values will work exactly like adding two `u64` values, generating exactly the
same code on targets with 64-bit registers. For targets with 32-bit registers,
it may generate slightly different code, although in practice it'll probably be
a long time before these sorts of optimisations are applied.

Additionally, the restriction that `N <= 128` may allow for future changes
where `uint<N>` is not actually stored as `uint<N.next_power_of_two()>` for
sufficiently large `N`. Right now, even storing `uint<65>` as `u128` makes the
most sense, because it will be the most efficient representation on 64-bit CPUs
for performing many operations. However, storing `uint<4097>` as `uint<8192>`,
for example, may be total overkill.

The main goal for these representations is to avoid bit-wrangling as much as
possible. Although truncating the number of bits for integers is no longer free,
e.g. `u64` to `uint<48>` requires an AND and `i64` to `int<48>` requires
conditionally excecuting an AND or an OR, the goal is that in most cases, using
non-power-of-two lengths should be more or less free.

## Niche optimizations for enums

As a hard rule, `uint<N>` will always have its upper `N.next_power_of_two() - N`
bits set to zero, and similarly, `int<N>`'s upper bits will be sign-extended. To
compute niche optimizations for enums, we'll have to take this into account.

Niche values for `uint<N>` are simply any nonzero patterns in the upper bits,
similarly to `bool`. This means that for `uint<7>`, for example, we have `128`
possible niche values.

Niche values for `int<N>` are trickier. The niche with all bits set to one
represents the valid value -1, and should be replaced with a niche where all
the `int<N>` bits are set to one and the extra bits are set to zero. Other than
that, in general, both `uint<N>` and `int<N>` will have
`N.next_power_of_two() - N` bits of niche space.

## Standard library

Existing implementations for integer types should be annotated with
`default impl` as necessary, and most operations should defer to the
implementations for `N.next_power_of_two()`. For example, the `count_zeros`
function in the generic impl would be:

```rust
impl<const N: usize> uint<N> {
    fn count_zeros(self) -> u32 {
        let M = N.next_power_of_two();
        let zeros = (self as uint<M>).count_zeros();
        zeros + (M - N)
    }
}
```

Because of the specialisations when `N` is a power of two, this would always
result in the most efficient code possible. For example, the code for `uint<7>`
would get optimised to the equivalent:

```rust
fn count_zeros(self) -> u32 {
    (self as u8).count_zeros() - 1
}
```

For the most part, the code which uses intrinsics will be specialised, and the
code which doesn't will be replaced with a generic version. For example, `ctpop`
is required for `count_zeros`, and thus it's specialized. However, `Default`
does not require any specialisation, and can just be replaced with a single
generic impl.

## Documentation

For now, additional `primitive.uint` and `primitive.int` pages will be added to
rustdoc, and the existing `uN` and `iN` pages will be left as-is. Eventually, if
we're comfortable with it, we can remove the `uN` and `iN` pages entirely and
use `primitive.int` and `primitive.uint` as the single source of documentation
for these types, plus the pages for `usize` and `isize`.

There certainly is a precedent for this: as of right now, all of these pages
share the same documentation, and the examples are modified to work for all of
these types. Removing these separate pages would help remove documentation
redundancy, although `usize` and `isize` would still have to be kept separate.

# Potential future extensions

## `N > 128`

The most obvious extension is to allow arbitrarily sized integer types. One
clear use for this is cryptographic code which often works with
fixed-but-large-length integers, which would no longer have to decide whether
to offer its own `u256` or `u4096` types. Additionally, generalisation over
integers could allow code that works with e.g. 1024-, 2048-, and 4096-bit keys
to have their arithmetic monomorphised and specialised for free without writing
extra code.

Another benefit of this would be allowing arbitrary-length integer literals,
which would be useful for bignum libraries. Instead of parsing a string or byte
array to create a bignum, the bignum library could simply have:

```
fn new<const N: usize>(num: uint<N>) -> BigNum {
    // ...
}
```

Internally, the compiler could represent the power-of-two `uN` and `iN` as a
`doubled_uint<N / 2>` and `doubled_int<N / 2>`, which could even be
implemented in the standard library. At that point, only a lang item would be
necessary for allowing literal coercion.

Of course, at this point, how this extension would actually work is complete
speculation. It seems inevitable with this RFC, although due to the large number
of unsolved problems, should require one or several more RFCs to stabilise.

## Bit sizes and `repr(bitpacked)`

In the future, types could be sized in terms of bits instead of bytes, with
`(bit_size_of::<T>() + 7) / 8 == size_of::<T>()`. All types would have a bit
size, allowing for a future `repr(bitpacked)` extension which packs all values
in a struct or enum variant into the smallest number of bytes possible, given
their bit sizes. Doing so would prevent referencing the fields of the struct,
although the ability to set/get fields is still possible.

For example, here's a modified version of our previous example:

```rust
#[repr(C, bitpacked)]
struct MipsInstruction {
    opcode: uint<6>,
    rs: uint<5>,
    rt: uint<5>,
    immediate: u16,
}
```

We could allow referencing `&self.immediate` as it is aligned to a byte
boundary, although all of the other fields can only be copied (e.g. `self.rs`)
or set explicitly (e.g. `self.rt = 4`). Because of this restriction, we would
only allow `!Drop` types in bit-packed structs, as `Drop` requires a mutable
reference. We would probably initially want to limit to `Copy` types, but I
haven't thought much on why expanding to all `!Drop` types wouldn't be doable.

## ASCII-specific methods for `uint<7>` and `[uint<7>]`

All of the ASCII-specific methods for `u8` could be copied to `uint<7>`, where
all values are valid ASCII. Additionally, longer-term, the potential for using
`uint<7>` for ASCII-specific matching and splitting in `str` could be discussed.

Additionally, casting from `[uint<7>]` to `str` could be made safe, as all ASCII
strings are valid UTF-8. This could make code which interacts with ASCII much,
much safer, without needing to do `from_utf8_unchecked` or risk the penalty of
`from_utf8`.

# Drawbacks
[drawbacks]: #drawbacks

This is a *big* change to the language, not to be taken lightly. Additionally,
many libraries may start using `uint<N>` for non-power-of-two `N` because it
reduces the number of invalid values, even if it's not the best solution. For
example, a library taking a percentage from 0 to 100 might use a `uint<7>`,
even though it really should just take a `u8` or `u32`.

This requires const generics, which have not even been implemented in the
compiler at the time of writing. This would be the first builtin type which uses
generics at all, too.

No other language provides this kind of generalisation, and it may be more effort
on the compiler side than it's worth. For example, the existing 576 impls for
`Shl` might just be better and more performant than four new generic ones.

# Rationale and alternatives
[alternatives]: #alternatives

## Bound-based generalisation

Generalising integers over their number of bits is certainly a very logical way
to generalise integers *for computers*, but generalising based upon bounds is
a very natural way for humans to do it, *and* more general. For example, instead
of `uint<N>` and `int<N>` types, we could get away with just one type,
`int<A..=B>`. This would be more powerful than the original: for example, a
percentage could be represented exactly as `int<0..=100>`. Whether an integer is
signed simply depends on whether its lower bound is negative.

The primary reason for leaving this out is… well, it's a lot harder to
implement, and could be added in the future as an extension. Longer-term, we
could for example guarantee that `int<0..=2.pow(N)-1>` is equivalent to
`uint<N>`, and `int<-2.pow(N)..=2.pow(N)-1>` is equivalent to `int<N>`, ensuring
that ultimately, we could replace literals with our bounded types.

Again, how these bounded types work would have to be fleshed out in a future
RFC.

## Integer traits

Previously, Rust had traits which generalised the integer types and their
methods, but these were ultimately removed in favour of inherent methods.
Going with a generic `uint<N>` over an `Int` trait would avoid the problem of
determining which methods are suited for these traits; instead, the `uint<N>`
type would have all of them.

Additionally, having these traits does not allow non-power-of-two `uint<N>`
outright, and so, this feature is a strict superset of that.

Additionally, having separate `uint<N>` and `int<N>` types also solves the
problem of generalising signed and unsigned integers: everything would require
one impl for signed integers, and one for unsigned. This would ensure that these
types have exactly the same behaviour for all `N`, only differing in the upper
bound on the number of bits.

## Macro-based bitfield structs

Many proposals for bitfields involved using macros to create structs, which
would ultimately allow the underlying mechanisms for bitfield-based structs to
be decided by the ecosystem, rather than the language itself.

This is actually orthogonal to the proposal, as having language-level `uint<N>`
types means that such macros would be able to return exactly the type necessary
for each field. While something like `repr(bitpacked)` would be very far off in
terms of stabilisation, these macros could simply get and set `uint<N>` values,
and users could determine how to work with them without having to worry about
checking for overflow or manually implementing code which has already been
written.

So, as far as this RFC is concerned, this is again orthogonal to the existing
RFC.

## Offering as a library

Once const generics and specialisation are implemented and stable, almost all of
this could be offered as a crate which offers `uint<N>` and `int<N>` types. I
won't elaborate much on this because I feel that there are many other
optimisations that can be added if this were a compiler feature, and that the
standard library is simpler with them, but this is definitely worth adding here
as an option.

## Going without

In the [Rust 2018 call for community blog posts], a fair number of Rustaceans
expressed a desire for bitfields at the language level, for use in embedded
code. So, while having bitfields is non-negotiable, this RFC's existence is
definitely put into question.

If this RFC were not accepted, then the most suitable alternative would be to
develop a crate in the nursery which is suitable for creating bitflag-based
structs in a way that offers robust error messages and reasonable overflow
behaviour. However, this would not solve all of the other problems this RFC has
mentioned, and this is likely not the ideal path.

[Rust 2018 call for community blog posts]: https://readrust.net/rust-2018/
[`bitfield` crate]: https://crates.io/crates/bitfield

# Prior art
[prior-art]: #prior-art

At the time of writing, no known programming language offers this level of
integer generalisation, and if this RFC were to be accepted, Rust would be the
first.

A few other ways of integer generalisation were explored in the alternatives
section, like integer traits. A number of languages, notably Haskell and other
functional languages, have generalised their integer types over a single trait.

Additionally, Ada offers range typyes similar to those mentioned in the
alternatives. I only noticed this near the completion of this RFC and haven't
elaborated much on it.

# Unresolved questions
[unresolved]: #unresolved-questions

This RFC has a number of unresolved questions, many of which are probably suited
to be solved during the development of this feature, rather than in this RFC.
However, here are just a few:

* Should `uN` and `iN` suffixes for integer literals, for arbitrary `N`, be
  allowed?
* What should the behaviour of `uint<N>` and `int<N>` be when casting to/from
  floats?
* Should `uint<N>` and `int<N>` be the smallest size possible, or have a
  power-of-two size?
* Should `uint<0>` and `int<0>` be allowed as constant zeroes?
* Should `uint<1>` and `int<1>` allow casting to `bool`?
* Should a sketch of a design for bitfields be proposed before this RFC is
  accepted? Would `repr(bitpacked)` even be feasible?
