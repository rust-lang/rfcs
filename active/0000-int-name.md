- Start Date: 2014-11-09
- RFC PR #: (leave this empty)
- Rust Issue #: (leave this empty)

# Summary

Rename the pointer-size integer types `int` and `uint` to avoid misconceptions and misuses. They aren't the default types you're looking for. Several candidate names are listed under **Alternatives**, below, starting with `iptr` and `uptr`.

This RFC assumes a fixed-size integer type will become the type inference fallback and thus the language's "default integer type." See [RFC: Change integer fallback RFC to suggest `i32` instead of `int` as the fallback](https://github.com/rust-lang/rfcs/pull/452). That fixed-size type will be used heavily in tutorials and libraries.


# Motivation

  - To avoid programmer misconceptions and misuses about integer types. Target-dependent integers are for specific uses and should not look like the "default" types to use.
  - To avoid a class of bugs when porting code to different size address spaces.
  - To avoid excess performance costs when porting Rust code to larger address spaces.
  - To resolve many discussions about these issues.


# Background

The Rust language does array indexing using the smallest integer types that span the address space. For this purpose, Rust defines two [machine-dependent integer types](http://doc.rust-lang.org/reference.html#machine-dependent-integer-types) that have the same number of bits as the target platform's pointer type. They're currently named `uint` for indexing and `int` for pointer differences.

(For memory safety, the language sets the theoretical upper bound on object and array size to the maximum `int` value.)

These types are useful for "memory numbers": indices, counts, sizes, offsets, etc. The problem is their names.

Contrary to expectations set by other programming languages, these are not the fastest, "native," register, C-sized, "word" sized, nor 32-bit integer types.

Given the history, `int` and `uint` _look_ like default integer types, but a target-dependent size is not a good default.

Using pointer-sized integers for computations that are not limited by memory produces code with overflow bugs ([checked or unchecked](https://github.com/rust-lang/rfcs/pull/146)) on different size targets, non-portable binary I/O, and excess performance costs.

This RFC replaces [RFC: int/uint portability to 16-bit CPUs](https://github.com/rust-lang/rfcs/pull/161).


# Detailed Design

Rename these two pointer-sized integer types. Decide on new names that convey their intended memory-scale uses rather than general-purpose integers.

Update code and documentation to use pointer-sized integers more narrowly for array indexing and related purposes. Provide a deprecation period to carry out these updates.

Rename the integer literal suffixes `i` and `u` to new names that suit the new type names. The suffix could be the same as the type, e.g. `32umem`, `32uptr`, or `32usize` (depending on the new names selected) or a shorter form, e.g. `32um` and `100im`.


# Drawbacks

  - Renaming `int`/`uint` requires changing a bunch of existing code. On the other hand, this is an ideal opportunity to fix integer portability bugs.
  - The Rust Guide also needs to change, but it'll mostly change for the integer type inference fallback type.
  - The new names are longer.


# Alternatives

Alternative names:

  - `iptr` and `uptr`, which refer directly to the (variable) *pointer* length just like `i32` refers to the length 32 bits.
  - `index` and `uindex`, related to array indexing and preserving Rust's "i"/"u" integer prefixes, however `uindex` is the type used for indexing. (Is "index" too good of an identifier to sacrifice to a keyword?)
  - `sindex` and `index`, since the unsigned type is the one used for indexing.
  - `intptr` and `uintptr`, [borrowing from C's](https://en.wikipedia.org/wiki/C_data_types#Fixed-width_integer_types) `intptr_t` and `uintptr_t`. These names are awkward by design.
  - `isize` and `usize`, [borrowing from C's](https://en.wikipedia.org/wiki/C_data_types#Size_and_pointer_difference_types) `ssize_t` and `size_t` with Rust's "i/u" prefixes, indicating integers large enough to hold the *size-in-bytes* of a memory object, and thus ([as in C++](http://en.cppreference.com/w/cpp/types/size_t)) the right range to index an in-memory array of elements at least 1 byte each.
  - `imem` and `umem`, meaning *"memory numbers."* These type names are suitable for indexes, counts, offsets, and sizes (unlike `uptr`, `uindex`, and `usize`). As memory numbers, it makes sense that they're sized to fit the address space.
  - `index` and `ptrdiff`.
  - `offset` and `size`.
  - `ioffset` and `ulength` or `ulen` or `uaddr`.
  - `intps` and `uintps`.
  - `PointerSizedInt` and `PointerSizedUInt`.
  - ...

The impact of not doing this: Portability bugs, peformance bugs, difficulties explaining the language, and recurring discussions about this. Also a possible impact on language adoption when people read warnings to be careful about using `int`.

Another alternative considered is a lint warning on every use of `int` or `uint` that's not directly related to array indexing.


# Unresolved Questions

  - Change this before Rust 1.0?


# Discussion References

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
