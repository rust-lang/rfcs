- Feature Name: `generic_integers`
- Start Date: 2024-08-07
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Adds the builtin types `u<N>` and `i<N>`, allowing integers with an arbitrary size in bits.

# Motivation
[motivation]: #motivation

## Generalising code for integers

Right now, there's a *lot* of boilerplate for implementing methods for integer primitives. The standard library itself is a great example; almost the entirety of `core::num` uses some gnarly macros to define all sorts of traits for all the
integer types. One example is `Shl` and `Shr`, which are defined for not just every integer type, but every *combination* of integer types. We could easily do this with const generics instead:

```rust
impl<const N: usize, const M: usize> Shl<u<M>> for u<N> {
    type Output = u<N>;
    #[inline]
    fn shl(self, rhs: u<M>) -> u<N> {
        // implementation
    }
}
```

This will decrease compilation time across the entire Rust ecosystem, maybe not by a noticeable amount, but by some amount, due to the presence of macro-based trait implementations across the board.

## Documentation decluttering

Having generic impls would drastically reduce the noise in the "implementations" section of rustdoc. For example, the number of implementations for `Add` for integer types really drowns out the fact that it's also implemented for strings and `std::time` types, which is useful to know too.

## Enum optimisations

Using a smaller number of bits for a value also has the benefit of niche enum optimisations. For example, `u<7>` represents a single ASCII character, and `Option<u<7>>` can be stored in a single byte. Additionally, `Result<u<7>, E>` also takes one byte if `E` is zero-sized.

This works because we know that values in the range `0b0_0000000..=0b0_1111111` are valid `u<7>`, but the values in the range `0b1_0000000..=0b1_1111111` are not and can represent the zero-sized variants `None` and `Err(E)`.

## Bit masks

Integers are very useful as a simple list of bits, and specifically for generic integers, this allows numbers of bits that aren't an existing integer type. There will probably always be a need for dedicated data structures like [`BitVec`], but at least for simple cases, being able to do this with your standard integer types is nice too.

In particular, encoding these as an integer helps avoids the issues you might get with endianness when you start splitting them into arrays. The compiler always knows the order of the bits, and you can take them out and put them back in whatever order you want. [In fact, the portable SIMD working group has already been considering generic integers as a useful construct for this.][SIMD bitmasks]

[`BitVec`]: https://docs.rs/bitvec
[SIMD bitmasks]: https://github.com/rust-lang/rust/issues/126217

## Packed-bits structures

One commonly requested feature from C is bitfields, where multiple fields in a struct can be defined as ranges of bits, rather than bytes. Here's an example in C:

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

In this format, all the data we need (in this case, one particular kind of MIPS instruction) is stored within 32 bits, but we don't have any particular way to map these to fields. In today's Rust, if we wanted to represent this struct, we'd have to create methods to grab the data for each field using bit shifts and masking. Here's an example of what this looks like for the `rs` field:

```rust
fn get_rs(inst: u32) -> u32 {
    (inst >> 21) & !(!0 << 5)
}
fn set_rs(inst: u32, rs: u32) -> u32 {
    inst & (!(!0 << 5) << 21) | ((rs & !(!0 << 5)) << 21)
}
```

As you can see, getting the shift (`21`) and mask (`!(!0 << 5)`) is not as obvious as you'd think. The shift is actually the sum of the widths of all of the fields after `rs` (`5 + 5 + 5 + 6 == 21`), and the mask is actually `0b11111`, where the number of ones corresponds to the size of the field. It's very easy for a human to mess these up, whereas in this case, C does all of the work for you.

While having an explicit bitfield representation is a ways off, with generic integers, we can at least make a proc macro to generate all this code for us, and use an API that explicitly specifies the sizes of each field:

```rust
bitfield! {
    struct MipsInstruction {
        opcode: u<6>,
        rs: u<5>,
        rt: u<5>,
        rd: u<5>,
        shift: u<5>,
        function: u<6>,
    }
}
```

Which would roughly be equivalent to:

```rust
struct MipsInstruction { /* ... */ }
impl MipsInstruction {
    fn pack(opcode: u<6>, rs: u<5>, rt: u<5>, rd: u<5>, shift: u<5>, function: u<6>) -> MipsInstruction { /* ... */ }
    fn opcode(&self) -> u<6> { /* ... */ }
    fn rs(&self) -> u<5> { /* ... */ }
    fn rt(&self) -> u<5> { /* ... */ }
    fn rd(&self) -> u<5> { /* ... */ }
    fn shift(&self) -> u<5> { /* ... */ }
    fn function(&self) -> u<6> { /* ... */ }
}
```

Having the ability to explicitly state in your API how many bits a field takes up, and to be able to statically ensure that someone is providing exactly that many bits, is a pretty nice thing to have. Without generic integers, we can't make those kinds of API guarantees.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Up until now, we've only seen *specific* integer types, like `u8` or `i32`. What if we want to write a trait that works with all of them?

Let's go with a simple example. Let's say that we want to code our own `Default` trait which returns one instead of zero. We'll define it like this:

```rust
pub trait MyDefault: Sized {
    fn my_default() -> Self;
}
```

Right now, with what we know, we have to copy-paste our impl for every integer type. For example, an impl for `u8` would be:

```rust
impl MyDefault for u8 {
    fn my_default() -> u8 { 1 }
}
```

Except, we'd have to replicate this for every single integer type. If we're clever, we could use a macro:

```rust
macro_rules! impl_my_default {
    ($($i: ident),*) => {$(
        impl MyDefault for $i {
            fn my_default() -> $i { 1 }
        }
    )*}
}
impl_my_default!(i8, u8, i16, u16, i32, u32, i64, u64, i128, u128, isize, usize);
```

But really, that's just making the compiler do the copy-pasting for us. Instead, we're going to use the special types `u<N>` and `i<N>` to generalise the code for us. The end result looks like:

```rust
impl<const N: usize> MyDefault for u<N> {
    fn my_default() -> u<N> { 1 }
}
impl<const N: usize> MyDefault for i<N> {
    fn my_default() -> i<N> { 1 }
}
impl MyDefault for usize {
    fn my_default() -> usize { 1 }
}
impl MyDefault for isize {
    fn my_default() -> isize { 1 }
}
```

That's a lot better. Now, as you'll notice, we still have to cover the types `usize` and `isize` separately; that's because they're still separate from the `u<N>` and `i<N>` types. If you think about it, this has always been the case before generic integers; for example, on a 64-bit system, `u64` is not the same as `usize`.

## Zero-sized integers

There's one slight caveat here: our `my_default` method might overflow. This seems silly, but there's three types, `i<1>`, `u<0>`, and `i<0>`, which can't have the value 1. In general, if you're casting a literal *to* a generic integer, you can't expect any value other than zero to work. In the future, we'll be able to annotate our `i<N>` impl with something like `where N > 1` or `where N >= 8`, but until then, we'll have to deal with this weird overflow behaviour.

The rules you'd expect apply to `u<N>` and `i<N>`, which is that `u<N>` stores values in the range `0..2.pow(N)`, and `i<N>` stores integers in the range `-2.pow(N - 1)..2.pow(N - 1)`. This means that `u<1>` only holds the values `0` and `1`, and `i<1>` only holds the values `-1` and `0`. The meaning for `u<0>` and `i<0>` is a little less clear, but they both are only allowed to contain the value `0`; the ranges end up being `0..1` and `-0.5..0.5`, which… yeah, zero is the only integer in those ranges, but it still can be confusing.

For now, if you want to ensure that your integers are the right size, you can add a `const { ... }` assertion to your implementations like so:

```rust
impl<const N: usize> MyDefault for u<N> {
    fn my_default -> u<N> {
        const { assert!(N > 1); }
        1
    }
}
```

This will cause the compiler to fail when `MyDefault` is used for `u<0>` or `u<1>`, since it will force the constant block to be evaluated. Not ideal, but it's the best we've got for now.

## Uncommonly sized integers

One other side effect of having `u<N>` and `i<N>` is that we can represent a lot more types than before. For example, `u<7>` is just a seven-bit integer, which we might use to represent an ASCII character. That said, using fewer bits doesn't necessarily mean you'll use up fewer bits of memory-- for example, `u<7>` by itself still requires a `u<8>` in memory.

Overall, you should expect integers where `N` is not a power of two to take up more size/alignment than their bits might imply. There may be ways of packing the bits together in a way that optimizes the amount of space used, but once you read those values into a `u<N>` or `i<N>`, this is no longer the case.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## Basic semantics

The compiler will gain the built-in integer types `u<N>` and `i<N>`, where `const N: usize`. These be identical to existing `uN` and `iN` types wherever possible, e.g. `u<8> == u8`.

`usize`/`isize` continue to be separate from `u<N>`/`i<N>`, where `N` is the target pointer width, as this also applied to `uN` and `iN`. `bool` will also be made different from `u<1>` for similar reasons, even though casting `false` to `0` and `true` to `1` will remain possible.

`u<N>` are able to store integers in the range `0..2.pow(N)` and `i<N>` are able to store integers in the range `-2.pow(N-1)..2.pow(N-1)`. The cheeky specificity of "integers in the range" ensures that, for `i<0>`, the range `-0.5..0.5` only contains the integer zero; in general, `u<0>` and `i<0>` will need to be special-cased anyway, as they must be ZSTs.

It's always valid to `as`-cast `u<N>` or `i<N>` to `u<M>` or `i<M>`, and the usual sign extension or truncation will occur depending on the bits involved. A few additional casts which are possible:

* from `bool` to `u<N>` or `i<N>`
* from `char` to `u<N>` or `i<N>`
* from `u<1>` to `bool`
* from `u<N>` to `char`, where `N < 16`

Note that casting directly to `u<0>` or `i<0>` is still allowed, to avoid forcing users to special-case their code. See later notes on possible lints.

Integer literals can be automatically coerced to `u<N>` and `i<N>`, although generic `iN` and `uN` suffixes are left out for future RFCs. When coercing literals to an explicit `u<N>` or `i<N>`, the `overflowing_literals` lint should trigger as usual, although this should not apply for generic code. See later notes on possible lint changes.

In general, operations on `u<N>` and `i<N>` should work the same as they do for existing integer types, although the compiler may need to special-case `N = 0` and `N = 1` if they're not supported by the backend.

When stored, `u<N>` should always zero-extend to the size of the type and `i<N>` should always sign-extend. This means that any padding bits for `u<N>` can be expected to be zero, but padding bits for `i<N>` may be either all-zero or all-one depending on the sign.

## Size, alignment, and ABI

The ABI of `u<N>` and `i<N>` is not required to be compatible with C23's [`_BitInt`] by this RFC, although `ffi::c_unsigned_bit_int` and `ffi::c_bit_int` types could be added in the future. Adding those two types, either as special aliases or wrapper structs, is left for as a future change, mostly because the ABI for `_BitInt` is still uncertain on some targets.

For now, the exact size and alignment of `u<N>` and `i<N>` can be decided as part of the actual implementation, based upon what the community most desires. There are a few things to consider, namely:

* Users can always *increase* the alignment and size of integers by wrapping them in `repr(align(...))` structs, but they cannot *decrease* them. This is an argument for making the size/alignment of these integers as small as possible.
* Loading unaligned integers (lower than the alignment of `usize`) can be particularly slow and require additional operations. However, once these integers are loaded, the backend doesn't need to do any extra operations until they need to be stored again. This is an argument for making the size/alignment of these integers larger than necessary.
* Right now, C's [`_BitInt`] (on x86_64 Unix) does prefer the second option, but the actual status of the spec is in flux due to [incompatibilities with the ABI for `__int128`][`__int128`], and it's unclear what the final version will be. Whether we should always match the ABI for C depends on what the community prefers most, and whether wrapping values in a struct at FFI boundaries is reasonable.

[`_BitInt`]: https://en.cppreference.com/w/c/language/arithmetic_types
[`__int128`]: https://gitlab.com/x86-psABIs/x86-64-ABI/-/issues/11

## Limits on `N`

There are two primary limits that restrict how large `N` can be:

1. All allocations in Rust are limited to `isize::MAX` bytes.
2. Most integer methods and constants use `u32` when counting bits.

The first restriction doesn't matter since `isize::MAX` bytes is `isize::MAX * 8` bits, which is larger than `usize::MAX = isize::MAX * 2 + 1` bits.

However, the second restriction is somewhat significant: for systems where `usize::MAX > u32::MAX`, we are still effectively restricted to `N <= u32::MAX` unless we wish to change these APIs. We can treat this as effectively a post-monomorphisation error similar to the error you might get when adding very large arrays inside your type; it's unlikely that someone might encounter them, but they do exist and have to be accounted for.

It's worth noting that `u32::MAX` bits is equivalent to 0.5 GiB, and thus no integer in Rust will be able to be larger than this amount. This is seen as acceptable because at that size, people can just use their own big-integer types. For now, adding a dedicated big-integer type to the standard library is left as a potential future change.

The compiler should be allowed to restrict `N` even further, maybe even as low as `u16::MAX`, due to other restrictions that may apply. For example, the LLVM backend currently only allows integers with widths up to `u<23>::MAX` (not a typo; 23, not 32). On 16-bit targets, using `usize` further restricts these integers to `u16::MAX` bits.

While `N` could be a `u32` instead of `usize`, keeping it at `usize` makes things slightly more natural when converting bits to array lengths and other length-generics, and these quite high cutoff points are seen as acceptable. In particular, this helps using `N` for an array index until [`generic_const_exprs`] is stabilized.

[`generic_const_exprs`]: https://github.com/rust-lang/rust/issues/76560

## Standard library

The existing macro-based implementation for `uN` and `iN` should be changed to implement for only `u<N>`, `i<N>`, `usize`, and `isize` instead; this has already been implemented in a mostly-generic way and should work as expected.

Unfortunately, there are a couple things that will have to remain implemented only for the existing powers of two due to the lack of constant bounds and complex const generics, namely:

* `From` and `TryFrom` implementations (requires const-generic bounds)
* `from_*e_bytes` and `to_*e_bytes` methods (requires [`generic_const_exprs`])

Currently, the LLVM backend already supports generic integers (you can refer to `iN` and `uN` as much as you want), although other backends may need additional code to work with generic integers.

## Overflow semantics

One important factor to consider for non-power-of-two integers is that overflow will require more work than usual to account for. In particular, because we can't rely on values being truncated auto-magically when stored back in memory, we'll have to explicitly mask or shift them to ensure that the correct values are stored for the padding bits.

Because of this, the `unchecked_*` methods may actually be more important and more-often used for these integers, at least when they are not powers of two.

The compiler, or at least backends like LLVM, should be able to optimise series of operations to perform these conversions less often, but it should be noted that they must always occur, even in release mode.

## Enum variants

For now, enum variants will still be restricted to their current set of integer types, since even [`repr(u128)`] isn't stable yet. If an RFC like [#3659] gets passed, allowing arbitrary types for enum variant tags, then `u<N>` should be included in that, although that can be added as a future extension.

[`repr(u128)`]: https://github.com/rust-lang/rust/issues/56071
[#3659]: https://github.com/rust-lang/rfcs/pull/3659

## Documentation

For now, additional `primitive.u` and `primitive.i` pages will be added to rustdoc, and the existing `uN` and `iN` pages will be left as-is. Eventually, if we're comfortable with it, we can remove the `uN` and `iN` pages entirely and use `primitive.i` and `primitive.u` as the single source of documentation for these types, plus the pages for `usize` and `isize`.

There certainly is a precedent for this: as of right now, all of these pages share the same documentation, and the examples are modified to work for all of these types. Removing these separate pages would help remove documentation redundancy, although `usize` and `isize` would still have to be kept separate.

Since the `uN` modules are currently deprecated, no `std::u` or `std::i` module should be added.

## Possible lints

Due to the presence of edge cases like `N = 0` and `N = 1`, it feels reasonable to add in a few lints to prevent people from doing silly things like:

* casting anything to `u<0>` or `i<0>` (these are just the singleton zero, and so a cast is a meaningless operation)
* coercing a literal integer to a generic integer (anything besides zero might overflow without a restriction on `N`, and once restrictions on `N` become possible, we can take that into account)

Preferably, a lot of the lints surrounding generic integers should be added to clippy before being accepted into the compiler, since it's likely many of them will cause more headaches than they're worth. We can continue to investigate possible lints to avoid these errors.

# Drawbacks
[drawbacks]: #drawbacks

This is a *big* change to the language, not to be taken lightly.

One of the biggest drawbacks is that this *only* allows encoding the storage size of an integer, and doesn't let users refine integers further to only allow a range of values. For example, it would be excellent for an API taking a "percent" value to enforce that the value is between 0 and 100, but generic integers alone cannot do this.

As an example, someone might end up using `u<7>` for a percent since it allows fewer extraneous values (`101..=127`) than `u<8>` (`101..=255`), although this actually just overcomplicates the code for little benefit, and may even make the performance worse.

Overall, things have changed dramatically since [the last time this RFC was submitted][#2581]. Back then, const generics weren't even implemented in the compiler yet, but now, they're used throughout the Rust ecosystem. Additionally, it's clear that LLVM definitely supports generic integers to a reasonable extent, and languages like [Zig] and even [C][`_BitInt`] have implemented them. A lot of people think it's time to start considering them for real.

Finally, there are still a few features lacking in the compiler that will add additional hurdles to implementation, like:

* a lack of const-generic bounds, like `N >= 8`
* the lack of [generic const expressions][`generic_const_exprs`], like `[u8; {N.div_ceil(8)}]`

However, this is substantially fewer hurdles than last time, and more cases have been brought up where generic integers will be useful despite these.

[pattern types]: https://github.com/rust-lang/rust/issues/123646
[#2581]: https://github.com/rust-lang/rfcs/pull/2581
[Zig]: https://ziglang.org/documentation/master/#Primitive-Types

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

## Use `uN` and `iN` instead of `u<N>` and `i<N>`

Since one of the major benefits of the generic types is to be able to use them in generic code, this isn't an option.

However, this RFC doesn't actually stop `uN` and `iN` from being added as aliases to `u<N>` and `i<N>` in the future. While the version with an explicit generic parameter is required for generic code, it should in theory be possible to add these aliases if people want them. These would require special changes to the language itself, since it does not make sense to add `2.pow(32)` items to the prelude.

## Use `uint<N>` and `int<N>` instead of `u<N>` and `i<N>`

This was the previous proposal used by the original RFC, and they were chosen due to a misconception about name conflicts. Although the `i` variable is commonly used for loops and other index variables, variable names and type names are entirely separate and can coexist peacefully. Having names closer to the originals feels more discoverable here.

Additionally, the names `uint` and `int` remain available for future, potentially unrelated integer types.

## Bound-based generalisation

Generalising integers over their number of bits is certainly a very logical way to generalise integers *for computers*, but generalising based upon bounds is a very natural way for humans to do it, *and* more general. For example, instead of `u<N>` and `i<N>` types, we could get away with just one type,
`int<A..=B>`. This would be more powerful than the original: for example, a percentage could be represented exactly as `int<0..=100>`. Whether an integer is signed simply depends on whether its lower bound is negative.

The primary reason for leaving this out is… well, it's a lot different from the existing integer types in the language. Additionally, such a proposal can coexist with this one, since there's nothing stopping us from making an `int<A..=B>` type in the future such that `u<N>` and `i<N>` are equivalent to the correct range of integers.

## Integer traits

Previously, Rust had traits which generalized the integer types and their methods, but these were ultimately removed in favour of inherent methods. Going with a generic `u<N>` over an `Int` trait would avoid the problem of determining which methods are suited for these traits; instead, the `u<N>` type would have all of them.

Additionally, having these traits does not allow non-power-of-two `u<N>` outright, and so, this feature is a strict superset of that.

Having separate `u<N>` and `i<N>` types also solves the problem of generalising signed and unsigned integers: everything would require one impl for signed integers, and one for unsigned. This would ensure that these types have exactly the same behaviour for all `N`, only differing in the upper bound on the number of bits.

## Offering as a library

This was the main proposal last time this RFC rolled around, and as we've seen, it hasn't really worked.

Crates like [`u`], [`bounded-integer`], and [`intx`] exist, but they come with their own host of problems:

* None of these libraries can easily unify with the existing `uN` and `iN` types.
* Generally, they require a lot of unsafe code to work.
* These representations tend to be slower and less-optimized than compiler-generated versions.
* They still require you to generalise integer types with macros instead of const generics.

A library solution really doesn't feel like the right option here. While libraries can create general integer *traits* to work over all of the existing `uN` and `iN` types, they can't easily make generic integer types.

[`u`]: https://docs.rs/u
[`bounded-integer`]: https://docs.rs/bounded-integer
[`intx`]: https://docs.rs/intx

## Going without

This is always an option, but hopefully it seems like a worse option after all that's been said so far.

# Prior art
[prior-art]: #prior-art

* [The previous RFC.][#2581]
* [Zulip RFC revival topic.][Zulip]
* [Generic integers in Zig.][Zig]
* [Generic integers in C23.][`_BitInt`]
* Probably several others discussions I'm missing.

[Zulip]: https://rust-lang.zulipchat.com/#narrow/stream/260443-project-const-generics/topic/adding.20int.3CN.3E

# Unresolved questions
[unresolved-questions]: #unresolved-questions

* How should `NonZero` be updated to account for `u<N>` and `i<N>`. Should `NonZero<u<0>>` and `NonZero<i<0>>` be uninhabited?
* Should we generalise even further between `u`, `i`, `usize`, and `isize`? This could be possible with [`adt_const_params`].
* How can we implement const-generic bounds in a way that supports implementations of `From` and `TryFrom` for generic integers?

[`adt_const_params`]: https://github.com/rust-lang/rust/issues/95174

# Future possibilities
[future-possibilities]: #future-possibilities

## Bit sizes and `repr(bitpacked)`

In the future, types could be sized in terms of bits instead of bytes, with `bit_size_of::<T>().div_ceil(8) <= size_of::<T>()`. Note that the `<=` here specifically allows for cases where types include multiple bytes of padding, like `char`.

All types would have a bit size, allowing for a future `repr(bitpacked)` extension which packs all values in a struct or enum variant into the smallest number of bytes possible, given their bit sizes. Doing so would prevent referencing the fields of the struct, although the ability to set/get fields is still possible.

For example, here's a modified version of our previous example:

```rust
#[repr(C, bitpacked)]
struct MipsInstruction {
    opcode: u<6>,
    rs: u<5>,
    rt: u<5>,
    immediate: u16,
}
```

We could allow referencing `&self.immediate` as it is aligned to a byte boundary, although all of the other fields can only be copied (e.g. `self.rs`) or set explicitly (e.g. `self.rt = 4`).

## Unsized integer types

Similar to the way that arrays can be unsized to slices, this feature currently lends itself to having an "unsized" version of `u<N>` or `i<N>` in a rather natural way.

Unfortunately, there are a lot of design problems to solve for this type, one of them being that operations will generally either require asserting that two integers are the same size, or allocating some owned big-integer type. For now, this is left as a future addition to the language instead of part of this RFC.

## Never-overflowing operations

With the addition of variable-bits integers, operations can be added that simply increase the size of the operand instead of overflowing:

```rust
impl<const N: usize> u<N> {
    fn widening_add(self, rhs: u<N>) -> u<{N+1}>;
    fn widening_mul(self, rhs: u<N>) -> u<{2*N}>;
}
```

In particular, `widening_mul` could solve the [current questions regarding the existing `widening_mul` type][bigint methods]. These are interesting methods, but left out of the RFC to be designed later.

[bigint methods]: https://github.com/rust-lang/rust/issues/85532

## ASCII-specific methods for `u<7>` and `[u<7>]`

Right now, the standard library has an unstable [`ascii::Char`] to represent ASCII characters, but this could be replaced with `u<7>` instead. Ultimately, it's unclear whether it's useful to distinguish between ASCII chars and `u<7>`, since unlike `u32` and `char`, all possible values are allowed.

[`ascii::Char`]: https://github.com/rust-lang/rust/issues/110998

## Generic floats

At some point in the future, floating-point numbers could similarly be generalised, allowing users to implement things for `float<N>` instead of `f16`, `f32`, `f64`, and `f128`. Unfortunately, floating-point numbers are [substantially more complicated][IEEE 754] than integers, and there's no one-size-fits-all formula to compute the various parameters of these types. Even if we generalized to `float<M, E>` (mantissa bits, exponent bits), we would still have issues with "weird" floating-point types like `f80` and `f64f64`, both proposed in RFC [#3456] (nice). Ultimately, a proper API for generic floats would require substantially more work and thus is not included in this RFC.

[IEEE 754]: https://en.wikipedia.org/wiki/IEEE_754
[#3456]: https://github.com/rust-lang/rfcs/pull/3456
