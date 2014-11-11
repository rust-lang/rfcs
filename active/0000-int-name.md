- Start Date: 2014-11-09
- RFC PR #: (leave this empty)
- Rust Issue #: (leave this empty)

# Summary

Rename the pointer-size integer types from `int` and `uint` to `index` and `uindex` to avoid misconceptions and misuses. They aren't the default types you're looking for.

This RFC assumes a fixed-size integer type will become the type inference fallback and thus the language's "default integer type." See [RFC: Change integer fallback RFC to suggest `i32` instead of `int` as the fallback](https://github.com/rust-lang/rfcs/pull/452). That fixed-size type will be used heavily in tutorials and libraries.


# Motivation

  - To avoid programmer misconceptions and misuses about integer types. Target-dependent integers are for specific uses and should not look like the "default" types to use.
  - To avoid a class of bugs when porting code to different size address spaces.
  - To avoid excess performance costs when porting Rust code to larger address spaces.
  - To resolve many discussions about these issues.


# Background

The Rust language does array indexing using the smallest integer types that span the address space. For this purpose, Rust defines integer types, currently named `int` and `uint`, that are large enough to hold a pointer in the target environment -- `uint` for indexing and `int` for pointer differences.

(For memory safety, the memory allocator will limit each node to half of address space so any array index will fit in a signed, pointer-sized integer.)

But contrary to expectations set by other programming languages, these are not the fastest, "native," register, C-sized, nor 32-bit integer types.

Given the history, `int` and `uint` _look_ like default integer types, but a target-dependent size is not a good default.

Using pointer-sized integers for computations that are not limited by memory produces code with overflow bugs ([checked or unchecked](https://github.com/rust-lang/rfcs/pull/146)) on different size targets, non-portable binary I/O, and excess performance costs.

This RFC replaces [RFC: int/uint portability to 16-bit CPUs](https://github.com/rust-lang/rfcs/pull/161).


# Detailed design

Rename these two types. The names `index` and `uindex` are meant to convey their intended use with arrays. Use them more narrowly for array indexing and related purposes.


# Drawbacks

  - Renaming `int`/`uint` requires changing a bunch of existing code. (The Rust Guide will change anyway, once the integer fallback type is chosen.)
  - The new names are longer.


# Alternatives

Alternative names:

  - `index` and `uindex`, named for their uses and preserving Rust's "i" and "u" integer prefixes.
  - `intptr` and `uintptr`, [borrowing from C's](http://en.cppreference.com/w/cpp/types/integer) `intptr_t` and `uintptr_t`. These names are awkward by design.
  - `isize` and `usize`, [borrowing from C's](http://en.cppreference.com/w/cpp/types/integer) `ssize_t` and `size_t` with Rust's "i/u" prefixes. But these types are defined as having the same number of bits as a pointer, not as a way of measuring sizes. A `usize` would be larger than needed for the largest memory node.
  - `intps` and `uintps`.
  - `PointerSizedInt` and `PointerSizedUInt`.
  - ...

The impact of not doing this: Portability bugs, peformance bugs, difficulties explaining the language, and recurring discussions about this. A possible impact on language adoption when people read warnings not to use `int`.

Another alternative considered is a lint warning on every use of `int` or `uint` that's not directly related to array indexing.


# Unresolved questions

  - Change this before Rust 1.0?


# References

  - [Guide: what to do about int](https://github.com/rust-lang/rust/issues/15526)
  - [If `int` has the wrong size …?](http://discuss.rust-lang.org/t/if-int-has-the-wrong-size/454)
  - [integer type style guidelines](https://github.com/rust-lang/rust-guidelines/issues/24)
  - [Encourage fixed-size integer](https://github.com/rust-lang/rust/issues/16446)

  - [Change integer fallback RFC to suggest `i32` instead of `int` as the fallback](https://github.com/rust-lang/rfcs/pull/452)
  - [Restore int fallback](https://github.com/rust-lang/rust/issues/16968)
  - [Restore int/f64 fallback for unconstrained literals](https://github.com/rust-lang/rfcs/pull/212) and [consider removing the fallback to int for integer inference](https://github.com/rust-lang/rust/issues/6023)
  - [Specify that int and uint are at least 32 bits on every CPU architecture](https://github.com/rust-lang/rust/issues/14758)
  - [RFC: rename `int` and `uint` to `intptr`/`uintptr`](https://github.com/rust-lang/rust/issues/9940)
  - [Decide whether to keep pointer sized integers as the default](https://github.com/rust-lang/rust/issues/11831)

Example `int`/`uint` portability bugs [listed by](https://github.com/rust-lang/rust/issues/16446#issuecomment-59621753) Mickaël Salaün:

  - [`std::num::pow`: exponent should not be a `uint`](https://github.com/rust-lang/rust/issues/16755)
  - [Bitv uses architecture-sized uints for backing storage](https://github.com/rust-lang/rust/issues/16736)
  - [libcollection : Switches from uint to u32 in BitV and BitVSet](https://github.com/rust-lang/rust/pull/18018)
  - [uint -> u32](https://github.com/dwrensha/capnproto-rust/commit/87ab4ee0fc03939ef2a186274395c8c69cb6689c), [update for uint -> u32](https://github.com/dwrensha/capnp-rpc-rust/commit/b2e0c953f60b389afd884264ea53cdec7f4de7b3)
