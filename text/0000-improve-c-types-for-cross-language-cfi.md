- Feature Name: `improve-c-types-for-cross-language-cfi`
- Start Date: 2022-07-25
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Improve C types to be able to identify C char and integer type uses at the time
types are encoded for cross-language LLVM CFI support.

# Motivation
[motivation]: #motivation

As the industry continues to explore Rust adoption, the absence of support for
forward-edge control flow protection in the Rust compiler is a major security
concern when migrating to Rust by gradually replacing C or C++ with Rust, and C
or C++ and Rust -compiled code share the same virtual address space. Thus,
support for forward-edge control flow protection needs to be added to the Rust
compiler and is a requirement for large-scale secure Rust adoption. For more
information about LLVM CFI and cross-language LLVM CFI support, see the design
document in the tracking issue [#89653][1][[1]].

## Type metadata
[type-metadata]: #type-metadata

LLVM uses type metadata to allow IR modules to aggregate pointers by their
types.[[2]] This type metadata is used by LLVM Control Flow Integrity to test
whether a given pointer is associated with a type identifier (i.e., test type
membership).

Clang uses the Itanium C++ ABI's[[3]] virtual tables and RTTI typeinfo
structure name[[4]] as type metadata identifiers for function pointers. The
typeinfo name encoding is a two-character code (i.e., “TS”) prefixed to the
type encoding for the function.

For cross-language LLVM CFI support, a compatible encoding must be used by
either

  1. Using a superset of types that encompasses types used by Clang (i.e.,
     Itanium C++ ABI's type encodings[[5]]), or at least types used at the FFI
     boundary.

  2. Reducing the types to the least common denominator between types used by
     Clang (or at least types used at the FFI boundary) and the Rust compiler
     (if even possible).

  3. Creating a new encoding for cross-language CFI and using it for Clang and
     Rust compilers (and possibly other compilers).

Option (1) provides a more comprehensive protection than option (2) and (3) for
Rust-compiled only code and when interoperating with foreign code written in C
and possibly other languages.

Option (2) may result in less comprehensive protection for Rust-compiled only
code, so it should be provided as an alternative to a Rust-specific encoding
for when mixing Rust and C and C++ -compiled code.

Option (3) would require changes to Clang to use the new encoding and,
depending on its requirements, may result in less comprehensive protection for
Rust-compiled only code and when interoperating with foreign code written in C
and other languages, similarly to option (2), so it should also be provided as
an alternative to a Rust-specific encoding for when mixing Rust and other
languages -compiled code.

## Defined type metadata identifiers (using Itanium C++ ABI)
[defined-type-metadata-1]: #defined-type-metadata-1

Option (1) is satisfied by using the Itanium C++ ABI with vendor extended type
qualifiers and types for Rust types that are not used at the FFI boundary.
Table II in the design document in the tracking issue [#89653][1][[1]] defines
type metadata identifiers for cross-language LLVM CFI support using option (1).

## Defined type metadata identifiers (using new encoding for cross-language CFI)
[defined-type-metadata-2]: #defined-type-metadata-2

Option (3) was also explored with the Clang CFI team by defining a new encoding
for cross-language CFI. This new encoding needed to be language agnostic and
ideally compatible with any other language. It also needed to support extended
types in case it was used as the main encoding to provide forward-edge control
flow protection.

To satisfy these requirements, however, this new encoding neither distinguishes
between certain types (e.g., bool, char, integers, and enums) nor discriminates
between pointed element types (the latter mainly because of C’s void * abuse).
This results in less comprehensive protection for Rust-compiled only code and
when interoperating with foreign code written in C, so this encoding will be
implemented and provided as an alternative option for interoperating with
foreign code written in languages other than C.

Option (3) is satisfied by using this new encoding with extended types for Rust
types that are not used at the FFI boundary. Table III in the design document
in the tracking issue [#89653][1][[1]] defines type metadata identifiers for
cross-language LLVM CFI support using option (3).

## Rust vs C char and integer types

Rust defines char as a Unicode scalar value, which is different from C’s char.
On most modern systems, C’s char is either an 8-bit signed or unsigned integer.
The Itanium C++ ABI specifies a distinct encoding for it (i.e., ‘c’).

Rust also uses explicitly-sized integer types (i.e., `i8`, `i16`, `i32`, ...)
while C uses abstract integer types (i.e., `char`, `short`, `long`, ...), which
actual sizes are implementation defined and may vary across different systems.
The Itanium C++ ABI specifies encodings for the C integer types (i.e., `char`,
`short`, `long`, ...), not their defined representations/sizes (i.e., 8-bit
unsigned integer, 16-bit unsigned integer, 32-bit unsigned integer, ...).

For convenience, some C-like type aliases are provided by libcore and libstd
(and also by the libc crate) for use when interoperating with foreign code
written in C. For instance, one of these type aliases is `c_char`, which is a
type alias to Rust’s `i8`.

To be able to encode these correctly, the Rust compiler must be able to
identify C char and integer type uses at the time types are encoded, and the C
type aliases may be used for disambiguation. However, at the time types are
encoded, all type aliases are already resolved to their respective `ty::Ty`
type representations[[6]] (i.e., their respective Rust aliased types), making
it currently not possible to identify C char and integer type uses from their
resolved types.

The Rust compiler also assumes that C char and integer types and their
respective Rust aliased types can be used interchangeably. These assumptions
can not be maintained when forward-edge control flow protection is enabled, at
least not at the FFI boundary (i.e., for extern function types with the "C"
calling convention).

To be able to use the defined type metadata identifiers defined using option
(1), the Rust compiler must be changed to:

  * be able to identify C char and integer type uses at the time types are
    encoded.

  * not assume that C char and integer types and their respective Rust aliased
    types can be used interchangeably when forward-edge control flow protection
    is enabled, at least not at the FFI boundary.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

TBD.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

To be able to use the defined type metadata identifiers defined using option
(1), the Rust compiler must be changed to:

  * be able to identify C char and integer type uses at the time types are
  encoded.

  * not assume that C char and integer types and their respective Rust aliased
  types can be used interchangeably when forward-edge control flow protection
  is enabled, at least not at the FFI boundary.

This may be done by either:

  1. creating a new set of transitional C types in `core::ffi` as user-defined
     types using `repr(transparent)` to be used at the FFI boundary (i.e., for
     extern function types with the "C" calling convention) when cross-language
     CFI support is needed (and taking the opportunity to consolidate all C
     types in `core::ffi`).

  2. changing the currently existing C types in `std::os::raw` to user-defined
     types using `repr(transparent)`.

  3. changing C types to `ty::Foreign` and changing `ty::Foreign` to be able to
     represent them.

  4. creating a new `ty::C` for representing C types.

Option (1) is opt in for when cross-language CFI support is needed, and
requires the user to use the new set of transitional C types for extern
function types with the "C" calling convention.

Option (2), (3), and (4) are backward-compatibility breaking changes and will
require changes to existing code that use C types.

# Drawbacks
[drawbacks]: #drawbacks

The Rust compiler assumes that C char and integer types and their respective
Rust aliased types can be used interchangeably. These assumptions can not be
maintained when forward-edge control flow protection is enabled, at least not
at the FFI boundary (i.e., for extern function types with the "C" calling
convention).

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

## Why not use the v0 mangling scheme?

Unfortunately, the v0 mandling scheme can not be used as an encoding for
cross-language CFI support due to the lack of support by other compilers,
mainly Clang.

# Prior art
[prior-art]: #prior-art

The author is currently not aware of any cross-language CFI implementation and
support by any other compiler and language.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

See [Reference-level explanation][reference-level-explanation].

# Future possibilities
[future-possibilities]: #future-possibilities

The defined type metadata identifiers using Itanium C++ ABI not only allows
cross-language CFI support, but also provides a more comprehensive protection
than a new encoding for cross-language CFI, while also allowing further
improvements for both CFI and cross-language CFI support (e.g., increasing
granularity by adding information, etc.).

[1]: <https://github.com/rust-lang/rust/issues/89653> "R. de C Valle. “Tracking Issue for LLVM Control Flow Integrity (CFI) Support for Rust #89653.” GitHub."
[2]: <https://llvm.org/docs/TypeMetadata.html> "\"Type Metadata.\" LLVM Documentation."
[3]: <https://itanium-cxx-abi.github.io/cxx-abi/abi.html> "\"Itanium C++ ABI\"."
[4]: <https://itanium-cxx-abi.github.io/cxx-abi/abi.html#mangling-special-vtables> "\"Virtual Tables and RTTI\". Itanium C++ ABI."
[5]: <https://itanium-cxx-abi.github.io/cxx-abi/abi.html#mangling-type> "\"Type Encodings\". Itanium C++ ABI."
[6]: <https://rustc-dev-guide.rust-lang.org/ty.html> "\"The ty module: representing types\". Guide to Rustc Development."
