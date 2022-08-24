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
or C++ and Rust -compiled code share the same virtual address space.

A safe language -compiled code such as Rust, when sharing the same virtual
address space with an unsafe language -compiled code such as C or C++, may
degrade the security of a program because of different assumptions about
language properties and availability of security features such as exploit
mitigations.

The issue this RFC aims to solve is an example of this, where entirely safe
Rust-compiled code, when sharing the same virtual address space with C or C++
-compiled code with forward-edge control flow protection, may degrade the
security of the program because the indirect branches in Rust-compiled code are
not validated, allowing forward-edge control flow protection to be trivially
bypassed.

This has been extensively discussed[[1]][[2]][[3]][[4]][[5]], and just recently
formalized[[6]] as a new class of attack (i.e., cross-language attacks). It was
also one of the major reasons that initiatives such as Rust GCC--which this
author also fully support--were funded[[5]].

Therefore, support for forward-edge control flow protection needs to be added
to the Rust compiler and is a requirement for large-scale secure Rust adoption.
For more information about this project, see the design document in the
tracking issue [#89653][7][[7]].

## Type metadata
[type-metadata]: #type-metadata

LLVM uses type metadata to allow IR modules to aggregate pointers by their
types.[[8]] This type metadata is used by LLVM Control Flow Integrity to test
whether a given pointer is associated with a type identifier (i.e., test type
membership).

Clang uses the Itanium C++ ABI's[[9]] virtual tables and RTTI typeinfo
structure name[[10]] as type metadata identifiers for function pointers. The
typeinfo name encoding is a two-character code (i.e., “TS”) prefixed to the
type encoding for the function.

For cross-language LLVM CFI support, a compatible encoding must be used by
either

  1. using Itanium C++ ABI mangling for encoding (which is currently used by
     Clang).

  2. creating a new encoding for cross-language CFI and using it for Clang and
     the Rust compiler (and possibly other compilers).

And

  * provide comprehensive protection for Rust-compiled only code if used as main
    encoding (and not require an alternative Rust-specific encoding for
    Rust-compiled only code).

  * provide comprehensive protection for C and C++ -compiled code when linking
    foreign Rust-compiled code into a program written in C or C++.

  * provide comprehensive protection across the FFI boundary when linking
    foreign Rust-compiled code into a program written in C or C++.

### Providing comprehensive protection for Rust-compiled only code if used as main encoding
[protection-rust-compiled]: #protection-rust-compiled

This item is satisfied by the encoding being able to comprehensively encode
Rust types. Both using Itanium C++ ABI mangling for encoding (1) and creating a
new encoding for cross-language CFI (2) may satisfy this item by providing
support for (language or vendor) extended types, by defining a comprehensive
encoding for Rust types using (language or vendor) extended types, and
implementing it in the Rust compiler.

### Providing comprehensive protection for C and C++ -compiled code when linking foreign Rust-compiled code into a program written in C or C++
[protection-c-cpp-compiled]: #protection-c-cpp-compiled

This item is satisfied by the encoding being able to comprehensively encode C
and C++ types, and Clang being able to continue to use a comprehensive encoding
for C and C++ -compiled code when linking foreign Rust-compiled code into a
program written in C or C++.

Both using Itanium C++ ABI mangling for encoding (1) and creating a new
encoding for cross-language CFI (2) may satisfy this item by providing support
for (language or vendor) extended types. However, a new encoding for
cross-language CFI (2) also requires defining a comprehensive encoding for C
and C++ types using (language or vendor) extended types, and implementing it in
Clang, so it is able to continue to use a comprehensive encoding for C and C++
-compiled code when linking foreign Rust-compiled code into a program written
in C or C++. This introduces as much complexity and work as redefining Itanium
C++ ABI mangling and reimplementing it in Clang.

Additionally, a new encoding for cross-language CFI (2), depending on its
requirements, may use a generalized encoding across the FFI boundary. This may
result in using a generalized encoding for all C and C++ -compiled code instead
of only across the FFI boundary, and may also require changes to Clang to use
the generalized encoding only across the FFI boundary (which may also require
new Clang extensions and changes to C and C++ code and libraries).

Either using a generalized encoding for all C and C++ -compiled code or across
the FFI boundary do not satisfy this or the following item, and will degrade
the security of the program when linking foreign Rust-compiled code into a
program written in C or C++ because the program previously used a more
comprehensive encoding for all its compiled code.

### Providing comprehensive protection across the FFI boundary when linking foreign Rust-compiled code into a program written in C or C++
[protection-across-ffi-boundary]: #protection-across-ffi-boundary

This item is satisfied by being able to encode uses of Rust or C types across
the FFI boundary by either

 * changing the Rust compiler to be able to identify and encode uses of C types
   across the FFI boundary.
 * changing Clang to be able to identify and encode uses of Rust types across
   the FFI boundary.

Both using Itanium C++ ABI mangling for encoding (1) and creating a new
encoding for cross-language CFI (2) require changing either the Rust compiler
or Clang to satisfy this item.

It may also require changes to Rust or C and C++ code and libraries. Improving
C types for the Rust compiler to be able to identify C char and integer type
uses at the time types are encoded for cross-language LLVM CFI support is what
this RFC proposes.

However, as described in the previous item, a new encoding for cross-language
CFI (2), depending on its requirements, may use a generalized encoding across
the FFI boundary, and while using a generalized encoding across the FFI
boundary does not require changing the Rust compiler or Clang to be able to
identify and encode uses of Rust or C types across the FFI boundary, it does
not satisfy this item either, and will degrade the security of the program when
linking foreign Rust-compiled code into a program written in C or C++ because
the program previously used a more comprehensive encoding for all its compiled
code.

### Using Itanium C++ ABI mangling for encoding (1) versus creating a new encoding for cross-language CFI (2)
[itanium-vs-new-for-encoding]: #itanium-vs-new-for-encoding

Using Itanium C++ ABI mangling for encoding (1) provides cross-language LLVM
CFI support with C and C++ -compiled code as is, provides more comprehensive
protection by satisfying all previous items, does not require changes to Clang,
and does not require any new Clang extensions and changes to C and C++ code and
libraries.

While using Itanium C++ ABI mangling for encoding (1) requires the defining a
comprehensive encoding for Rust types using (language or vendor) extended types
and implementing it in the Rust compiler, creating a new encoding for
cross-language CFI (2) requires defining comprehensive encodings for both Rust
and C and C++ types using (language or vendor) extended types, and implementing
them in both the Rust compiler and Clang respectively. This introduces as much
complexity and work as redefining Itanium C++ ABI mangling and reimplementing
it in Clang.

Additionally, a new encoding for cross-language CFI (2), depending on its
requirements, may provide less comprehensive protection by either using a
generalized encoding for all C and C++ -compiled code or across the FFI
boundary, not satisfying all previous items, requires changes to Clang, and may
require new Clang extensions and changes to C and C++ code and libraries.

(See [Defined type metadata identifiers [creating a new encoding for
cross-language CFI]](defined-type-metadata-new).)

## Defined type metadata identifiers (using Itanium C++ ABI mangling for encoding)
[defined-type-metadata-itanium]: #defined-type-metadata-itanium

Table II in the design document in the tracking issue [#89653][7][[7]] defines
type metadata identifiers for cross-language LLVM CFI support using Itanium C++
ABI mangling for encoding (1).

### Rust vs C char and integer types

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
type representations[[11]] (i.e., their respective Rust aliased types), making
it currently not possible to identify C char and integer type uses from their
resolved types.

The Rust compiler also assumes that C char and integer types and their
respective Rust aliased types can be used interchangeably. These assumptions
can not be maintained across the FFI boundary (i.e., for extern function types
with the "C" calling convention passed as callbacks across the FFI boundary)
when forward-edge control flow protection is enabled.

To be able to use Itanium C++ ABI mangling for encoding (1) and provide
comprehensive protection across the FFI boundary when linking foreign
Rust-compiled code into a program written in C or C++, the Rust compiler must
be changed to

  * be able to identify C char and integer type uses at the time types are
    encoded.

  * not assume that C char and integer types and their respective Rust aliased
    types can be used interchangeably across the FFI boundary when forward-edge
    control flow protection is enabled.

## Defined type metadata identifiers (creating a new encoding for cross-language CFI)
[defined-type-metadata-new]: #defined-type-metadata-new

Creating a new encoding for cross-language CFI (2) was also explored with the
Clang CFI team. This new encoding needed to be language agnostic and ideally
compatible with any other language. It also needed to support extended types in
case it was used as the main encoding to provide forward-edge control flow
protection.

However, to satisfy these requirements, this new encoding neither distinguishes
between certain types (e.g., bool, char, integers, and enums) nor discriminates
between pointed element types (the latter mainly because of C’s void * abuse).

This results in less comprehensive protection by either using a generalized
encoding for all C and C++ -compiled code or across the FFI boundary, and will
degrade the security of the program when linking foreign Rust-compiled code
into a program written in C or C++ because the program previously used a more
comprehensive encoding for all its compiled code.

This encoding will be provided as an alternative option for interoperating with
foreign code written in languages other than C and C++ or that can not use
Itanium C++ ABI mangling for encoding.

Table III in the design document in the tracking issue [#89653][7][[7]] defines
type metadata identifiers for cross-language LLVM CFI support creating a new
encoding for cross-language CFI (2).

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

TBD.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The Rust compiler also assumes that C char and integer types and their
respective Rust aliased types can be used interchangeably. These assumptions
can not be maintained across the FFI boundary (i.e., for extern function types
with the "C" calling convention passed as callbacks across the FFI boundary)
when forward-edge control flow protection is enabled.

To be able to use Itanium C++ ABI mangling for encoding (1) and provide
comprehensive protection across the FFI boundary when linking foreign
Rust-compiled code into a program written in C or C++, the Rust compiler must
be changed to

  * be able to identify C char and integer type uses at the time types are
    encoded.

  * not assume that C char and integer types and their respective Rust aliased
    types can be used interchangeably across the FFI boundary when forward-edge
    control flow protection is enabled.

This may be done by either

  1. creating a new set of C types in `core::ffi::cfi` as user-defined types
     using `repr(transparent)` to be used across the FFI boundary (i.e., for
     extern function types with the "C" calling convention passed as callbacks
     across the FFI boundary) when cross-language CFI support is needed, and
     keep the existing C-like type aliases.

  2. adding a new set of parameter attributes to specify the corresponding C
     types to be used for encoding across the FFI boundary (i.e., for extern
     function types with the "C" calling convention passed as callbacks across
     the FFI boundary) when cross-language CFI support is needed.

  3. creating a new set of transitional C types in `core::ffi` as user-defined
     types using `repr(transparent)` to be used across the FFI boundary (i.e.,
     for extern function types with the "C" calling convention passed as
     callbacks across the FFI boundary) when cross-language CFI support is
     needed (and taking the opportunity to consolidate all C types in
     `core::ffi`).

  4. waiting for the work in progress in rust-lang/rust#97974 for
     rust-lang/compiler-team#504 and use type alias information for
     disambiguation and to specify the corresponding C types to be used for
     encoding across the FFI boundary (i.e., for extern function types with the
     "C" calling convention passed as callbacks across the FFI boundary) when
     cross-language CFI support is needed.

  5. changing the currently existing C types in `std::os::raw` to user-defined
     types using `repr(transparent)`.

  6. changing C types to `ty::Foreign` and changing `ty::Foreign` to be able to
     represent them.

  7. creating a new `ty::C` for representing C types.

Options (1), (2), (3), (4) are opt in for when cross-language CFI support is
needed. These are not backward-compatibility breaking changes because the Rust
compiler currently does not support cross-language CFI (i.e., calls to extern
function types with the "C" calling convention passed as callbacks across the
FFI boundary).

Option (5), (6), and (7) are backward-compatibility breaking changes because
they will require changes to existing code that use C types.

# Drawbacks
[drawbacks]: #drawbacks

The Rust compiler also assumes that C char and integer types and their
respective Rust aliased types can be used interchangeably. These assumptions
can not be maintained across the FFI boundary (i.e., for extern function types
with the "C" calling convention passed as callbacks across the FFI boundary)
when forward-edge control flow protection is enabled.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

## Why not use the v0 mangling scheme for encoding?

Unfortunately, the v0 mandling scheme can not be used as an encoding for
cross-language CFI support due to the lack of support by other compilers,
mainly Clang.

## Why not just creating a new encoding for cross-language CFI?

(See [Defined type metadata identifiers [creating a new encoding for
cross-language CFI]](defined-type-metadata-new).)

## Why not just use hardware-assisted forward-edge control flow protection?

Newer processors provide hardware assistance for forward-edge control flow
protection, such as ARM Branch Target Identification (BTI), ARM Pointer
Authentication, and Intel Indirect Branch Tracking (IBT) as part of Intel
Control-flow Enforcement Technology (CET). However, ARM BTI and Intel IBT
-based implementations are less comprehensive than software-based
implementations such as [LLVM ControlFlowIntegrity
(CFI)](https://clang.llvm.org/docs/ControlFlowIntegrity.html), and the
commercially available [grsecurity/PaX Reuse Attack Protector
(RAP)](https://grsecurity.net/rap_faq).

## What do you mean by less comprehensive protection?

The less comprehensive the protection, the higher the likelihood it can be
bypassed. For example, Microsoft Windows Control Flow Guard (CFG) only tests
that the destination of an indirect branch is a valid function entry point,
which is the equivalent of grouping all function pointers in a single group,
and testing all destinations of indirect branches to be in this group. This is
also known as "coarse-grained CFI".

(This is even less comprehensive than the initial support for LLVM CFI added to
the Rust compiler as part of this project, which aggregated function pointers
in groups identified by their number of parameters [i.e.,
rust-lang/rust#89652], and provides protection only for the first example
listed in the partial results of this project in the design document in the
tracking issue [#89653][7][[7]])

It means that in an exploitation attempt, an attacker can change/hijack control
flow to any function, and the larger the program is, the higher the likelihood
an attacker can find a function they can benefit from (e.g., a small
command-line program vs a browser).

This is unfortunately the implementation hardware assistance (e.g., ARM BTI and
Intel IBT) were initially modeled based on for forward-edge control flow
protection, and as such they provide equivalent protection with the addition of
specialized instructions. Microsoft Windows eXtended Flow Guard (XFG), ARM
Pointer Authentication -based forward-edge control flow protection, and Intel
Fine Indirect Branch Tracking (FineIBT) aim to solve this by combining hardware
assistance with software-based function pointer type testing similarly to LLVM
CFI. This is also known as "fine-grained CFI".

(This is equivalent to the current support for LLVM CFI added to the Rust
compiler as part of this project, which aggregates function pointers in groups
identified by their return and parameter types [i.e., rust-lang/rust#95548].
(See the partial results of this project in the design document in the tracking
issue [#89653][7][[7]].)

## Why not just a generalized encoding across the FFI boundary?

This results in less comprehensive protection, may result in using a
generalized encoding for all C and C++ -compiled code instead of only across
the FFI boundary depending whether Clang can be changed to use the generalized
encoding only across the FFI boundary (which may also require new Clang
extensions and changes to C and C++ code and libraries), and will degrade the
security of the program when linking foreign Rust-compiled code into a program
written in C or C++ because the program previously used a more comprehensive
encoding for all its compiled code.

Finally, it does not completely solve the issue this RFC aims to solve, which
is that entirely safe Rust-compiled code, when sharing the same virtual address
space with C or C++ -compiled code with forward-edge control flow protection,
may degrade the security of the program because the indirect branches in
Rust-compiled code are not validated, allowing forward-edge control flow
protection to be trivially bypassed.

## Are the changes proposed in this RFC backward-compatibility breaking changes?

Options (1), (2), (3), (4) are opt in for when cross-language CFI support is
needed. These are not backward-compatibility breaking changes because the Rust
compiler currently does not support cross-language CFI (i.e., calls to extern
function types with the "C" calling convention passed as callbacks across the
FFI boundary).

Option (5), (6), and (7) are backward-compatibility breaking changes because
they will require changes to existing code that use C types.

# Prior art
[prior-art]: #prior-art

The author is currently not aware of any cross-language CFI implementation and
support by any other compiler and language.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

See [Reference-level explanation][reference-level-explanation].

# Future possibilities
[future-possibilities]: #future-possibilities

Using Itanium C++ ABI mangling for encoding (1) provides cross-language LLVM
CFI support with C and C++ -compiled code as is, provides more comprehensive
protection, does not require changes to Clang, and does not require any new
Clang extensions and changes to C and C++ code and libraries.

It allows further improvements for both CFI and cross-language CFI support
(e.g., increasing granularity by adding information, etc.), and also provides
the foundation for future implementations of cross-language hardware-assisted
and software-based -combined forward-edge control flow protection, such as ARM
Pointer Authentication -based forward-edge control flow protection.

[1]: https://stanford-cs242.github.io/f17/assets/projects/2017/songyang.pdf "Y. Song. \"On Control Flow Hijacks of unsafe Rust.\" GitHub."
[2]: https://www.cs.ucy.ac.cy/~elathan/papers/tops20.pdf "M. Papaevripides and E. Athanasopoulos. \"Exploiting Mixed Binaries.\" Elias Athanasopoulos Publications."
[3]: https://github.com/rust-lang/rust/files/4723836/Control.Flow.Guard.for.Rust.pdf "A. Paverd. \"Control Flow Guard for Rust.\" GitHub."
[4]: https://github.com/rust-lang/rust/files/4723840/Control.Flow.Guard.for.LLVM.pdf "A. Paverd. \"Control Flow Guard for LLVM.\" GitHub."
[5]: https://opensrcsec.com/open_source_security_announces_rust_gcc_funding "B. Spengler. \"Open Source Security, Inc. Announces Funding of GCC Front-End for Rust.\" Open Source Security."
[6]: https://www.ndss-symposium.org/wp-content/uploads/2022-78-paper.pdf "S. Mergendahl, N. Burow, H. Okhravi. \"Cross-Language Attacks.\" NDSS Symposium 2022."
[7]: <https://github.com/rust-lang/rust/issues/89653> "R. de C Valle. \"Tracking Issue for LLVM Control Flow Integrity (CFI) Support for Rust #89653.\" GitHub."
[8]: <https://llvm.org/docs/TypeMetadata.html> "\"Type Metadata.\" LLVM Documentation."
[9]: <https://itanium-cxx-abi.github.io/cxx-abi/abi.html> "\"Itanium C++ ABI\"."
[10]: <https://itanium-cxx-abi.github.io/cxx-abi/abi.html#mangling-special-vtables> "\"Virtual Tables and RTTI\". Itanium C++ ABI."
[11]: <https://rustc-dev-guide.rust-lang.org/ty.html> "\"The ty module: representing types\". Guide to Rustc Development."
