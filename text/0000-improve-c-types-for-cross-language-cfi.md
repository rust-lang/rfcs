- Feature Name: `improve-c-types-for-cross-language-cfi`
- Start Date: 2022-07-25
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Improve C types for cross-language LLVM CFI support.

# Motivation
[motivation]: #motivation

This RFC is part of the LLVM Control Flow Integrity (CFI) Support for Rust, and
is a requirement for cross-language LLVM CFI support.

For cross-language LLVM CFI support, the Rust compiler must be able to identify
and correctly encode C types in extern "C" function types indirectly called
(i.e., function pointers) across the FFI boundary when cross-language CFI
support is needed.

For convenience, Rust provides some C-like type aliases for use when
interoperating with foreign code written in C, and these C type aliases may be
used for identification. However, at the time types are encoded, all type
aliases are already resolved to their respective Rust aliased types, making it
currently not possible to identify C type aliases use from their resolved types.

For example, the Rust compiler currently is not able to identify that an

```rust
extern "C" {
    fn func(arg: c_long);
}
```

used the `c_long` type alias and is not able to disambiguate between it and an
`extern "C" fn func(arg: c_longlong)` in an LP64 or equivalent data model at the
time types are encoded.

This motivates creating a new set of C types that their use can be identified at
the time types are encoded to be used in extern "C" function types indirectly
called across the FFI boundary when cross-language CFI support is needed.

For more information about and the motivation for the project, see the design
document in the tracking issue [#89653][1][[1]] and the [Appendix][appendix].

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

This RFC proposes creating a new set of C types in `core::ffi::cfi` as
user-defined types using `repr(transparent)` to be used in extern "C" function
types indirectly called across the FFI boundary when cross-language CFI support
is needed, and keeping the existing C-like type aliases.

The new set of C types will make indirect calls to extern "C" function types
across the FFI boundary work when CFI is enabled. These indirect calls will
continue to not work when CFI is enabled unless the new set of C types are used.

These are not backward-compatibility breaking changes because the Rust compiler
currently does not support cross-language CFI (i.e., extern "C" function types
indirectly called across the FFI boundary when CFI is enabled).

For example:

example/src/main.rs
```rust
use std::ffi::c_long;

#[link(name = "foo")]
extern "C" {
    fn hello_from_c(_: c_long);
    fn indirect_call_from_c(f: unsafe extern "C" fn(c_long), arg: c_long);
}

unsafe extern "C" fn hello_from_rust(_: c_long) {
    println!("Hello, world!");
}

unsafe extern "C" fn hello_from_rust_again(_: c_long) {
    println!("Hello from Rust again!\n");
}

fn indirect_call(f: unsafe extern "C" fn(c_long), arg: c_long) {
    unsafe { f(arg) }
}

fn main() {
    // This works
    indirect_call(hello_from_rust, 1);
    // This works when using rustc LTO, but does not work when using (proper)
    // LTO because the Rust compiler and Clang use different encodings for
    // hello_from_c and the test at the indirect call site at indirect_call.
    indirect_call(hello_from_c, 2);
    // This does not work because the Rust compiler and Clang use different
    // encodings for hello_from_rust_again and the test at the indirect call
    // site at indirect_call_from_c.
    unsafe {
        indirect_call_from_c(hello_from_rust_again, 3);
    }
}
```

example/src/foo.c
```c
#include <stdio.h>
#include <stdlib.h>

void
hello_from_c(long arg)
{
    printf("Hello from C!\n");
}

void
indirect_call_from_c(void (*fn)(long), long arg)
{
    fn(arg);
}
```

Will need to be changed to:

example/src/main.rs
```rust
use std::ffi::c_long;
use std::ffi::cfi;

#[link(name = "foo")]
extern "C" {
    fn hello_from_c(_: cfi::c_long);
    fn indirect_call_from_c(f: unsafe extern "C" fn(cfi::c_long), arg: c_long);
}

unsafe extern "C" fn hello_from_rust(_: cfi::c_long) {
    println!("Hello, world!");
}

unsafe extern "C" fn hello_from_rust_again(_: cfi::c_long) {
    println!("Hello from Rust again!\n");
}

fn indirect_call(f: unsafe extern "C" fn(cfi::c_long), arg: c_long) {
    unsafe { f(cfi::c_long(arg)) }
}

fn main() {
    // This will continue to work
    indirect_call(hello_from_rust, 1);
    // This will work both when using rustc LTO and when using (proper) LTO
    // because the Rust compiler and Clang will use the same encoding for
    // hello_from_c and the test at the indirect call site at indirect_call.
    indirect_call(hello_from_c, 2);
    // This will work because the Rust compiler and Clang will use the same
    // encoding for hello_from_rust_again and the test at the indirect call site
    // at indirect_call_from_c.
    unsafe {
        indirect_call_from_c(hello_from_rust_again, 3);
    }
}
```

example/src/foo.c
```c
#include <stdio.h>
#include <stdlib.h>

void
hello_from_c(long arg)
{
    printf("Hello from C!\n");
}

void
indirect_call_from_c(void (*fn)(long), long arg)
{
    fn(arg);
}
```

Direct calls to extern "C" function types across the FFI boundary, whether CFI
is enabled or disabled, will continue to work whether Rust integer types or C
type aliases are used.

For example:

example/src/main.rs
```rust
// Optionally, use std::ffi::c_long. (Note this is the C type alias, not
// the new C type.)

#[link(name = "foo")]
extern "C" {
    fn hello_from_c(_: i64);
    // Or fn hello_from_c(_: c_long). (Note this is the C type alias,
    // not the new C type.)
}

fn main() {
    unsafe { hello_from_c(1); }
}
```

example/src/foo.c
```c
#include <stdio.h>
#include <stdlib.h>

void
hello_from_c(long arg)
{
    printf("Hello from C!\n");
}
```

Will continue to work when `fn hello_from_c(_: i64)` or `fn hello_from_c(_:
c_long)` represents a `void hello_from_c(long arg)` in an LP64 or equivalent
data model.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## Type metadata
[type-metadata]: #type-metadata

LLVM uses type metadata to allow IR modules to aggregate pointers by their
types.[[2]] This type metadata is used by LLVM Control Flow Integrity to test
whether a given pointer is associated with a type identifier (i.e., test type
membership).

Clang uses the Itanium C++ ABI's[[3]] virtual tables and RTTI typeinfo structure
name[[4]] as type metadata identifiers for function pointers.

For cross-language LLVM CFI support, a compatible encoding must be used. The
compatible encoding chosen for cross-language LLVM CFI support is the Itanium
C++ ABI mangling with vendor extended type qualifiers and types for Rust types
that are not used across the FFI boundary.

## Encoding C integer types

Rust defines `char` as an Unicode scalar value, while C defines `char` as an
integer type. Rust also defines explicitly-sized integer types (i.e., `i8`,
`i16`, `i32`, ...) while C defines abstract integer types (i.e., `char`,
`short`, `long`, ...), which actual sizes are implementation defined and may
vary across different data models. This causes ambiguity if Rust integer types
are used in extern "C" function types that represent C functions because the
Itanium C++ ABI specifies encodings for C integer types (e.g., `char`, `short`,
`long`, ...), not their defined representations (e.g., 8-bit signed integer,
16-bit signed integer, 32-bit signed integer, ...).

For example, the Rust compiler currently is not able to identify if an

```rust
extern "C" {
    fn func(arg: i64);
}
```

represents a `void func(long arg)` or `void func(long long arg)` in an LP64 or
equivalent data model.

For cross-language LLVM CFI support, the Rust compiler must be able to identify
and correctly encode C types in extern "C" function types indirectly called
across the FFI boundary when CFI is enabled.

For convenience, Rust provides some C-like type aliases for use when
interoperating with foreign code written in C, and these C type aliases may be
used for disambiguation. However, at the time types are encoded, all type
aliases are already resolved to their respective `ty::Ty` type
representations[[5]] (i.e., their respective Rust aliased types) making it
currently not possible to identify C type aliases use from their resolved types.

For example, the Rust compiler currently is also not able to identify that an

```rust
extern "C" {
    fn func(arg: c_long);
}
```

used the `c_long` type alias and is not able to disambiguate between it and an
`extern "C" fn func(arg: c_longlong)` in an LP64 or equivalent data model at the
time types are encoded.

This RFC proposes creating a new set of C types in `core::ffi::cfi` as
user-defined types using `repr(transparent)` to be used in extern "C" function
types indirectly called across the FFI boundary when cross-language CFI support
is needed, and keeping the existing C-like type aliases.

The new set of C types will make indirect calls to extern "C" function types
across the FFI boundary work when CFI is enabled. These indirect calls will
continue to not work when CFI is enabled unless the new set of C types are used.

These are not backward-compatibility breaking changes because the Rust compiler
currently does not support cross-language CFI (i.e., extern "C" function types
indirectly called across the FFI boundary when CFI is enabled).

For example:

example/src/main.rs
```rust
use std::ffi::c_long;

#[link(name = "foo")]
extern "C" {
    // This declaration would have the type id "_ZTSFvlE", but at the time types
    // are encoded, all type aliases are already resolved to their respective
    // Rust aliased types, so this is encoded as either "_ZTSFvu3i32E" or
    // "_ZTSFvu3i64E", depending to what type c_long type alias is resolved to,
    // which currently uses the u<length><type-name> vendor extended type
    // encoding for the Rust integer types--this is the issue this RFC
    // describes.
    fn hello_from_c(_: c_long);

    // This declaration would have the type id "_ZTSFvPFvlElE", but is encoded
    // as either "_ZTSFvPFvu3i32ES_E" (compressed) or "_ZTSFvPFvu3i64ES_E"
    // (compressed), similarly to the hello_from_c declaration above--this may
    // be ignored for the purposes of this example.
    fn indirect_call_from_c(f: unsafe extern "C" fn(c_long), arg: c_long);
}

// This definition would have the type id "_ZTSFvlE", but is encoded as either
// "_ZTSFvu3i32E" or "_ZTSFvu3i64E", similarly to the hello_from_c declaration
// above.
unsafe extern "C" fn hello_from_rust(_: c_long) {
    println!("Hello, world!");
}

// This definition would have the type id "_ZTSFvlE", but is encoded as either
// "_ZTSFvu3i32E" or "_ZTSFvu3i64E", similarly to the hello_from_c declaration
// above.
unsafe extern "C" fn hello_from_rust_again(_: c_long) {
    println!("Hello from Rust again!\n");
}

// This definition would also have the type id "_ZTSFvPFvlElE", but is encoded
// as either "_ZTSFvPFvu3i32ES_E" (compressed) or "_ZTSFvPFvu3i64ES_E"
// (compressed), similarly to the hello_from_c declaration above--this may be
// ignored for the purposes of this example.
fn indirect_call(f: unsafe extern "C" fn(c_long), arg: c_long) {
    // This indirect call site tests whether the destinatin pointer is a member
    // of the group derived from the same type id of the f declaration, which
    // would have the type id "_ZTSFvlE", but is encoded as either
    // "_ZTSFvu3i32E" or "_ZTSFvu3i64E", similarly to the hello_from_c
    // declaration above.
    //
    // Notice that since the test is at the call site and generated by the Rust
    // compiler, the type id used in the test is encoded by the Rust compiler.
    unsafe { f(arg) }
}

// This definition has the type id "_ZTSFvvE"--this may be ignored for the
// purposes of this example.
fn main() {
    // This demonstrates an indirect call within Rust-only code using the same
    // encoding for hello_from_rust and the test at the indirect call site at
    // indirect_call (i.e., "_ZTSFvu3i32E" or "_ZTSFvu3i64E").
    indirect_call(hello_from_rust, 1);

    // This demonstrates an indirect call across the FFI boundary with the Rust
    // compiler and Clang using different encodings for hello_from_c and the
    // test at the indirect call site at indirect_call (i.e., "_ZTSFvu3i32E" or
    // "_ZTSFvu3i64E" vs "_ZTSFvlE").
    //
    // When using rustc LTO (i.e., make using_rustc_lto), this works because the
    // declaration used is the Rust-declared hello_from_c, which has the type id
    // encoded by the Rust compiler (i.e., "_ZTSFvu3i32E" or "_ZTSFvu3i64E").
    //
    // When using (proper) LTO (i.e., make), this does not work because the
    // declaration used is the C-defined hello_from_c, which has the type id
    // encoded by Clang (i.e., "_ZTSFvlE").
    indirect_call(hello_from_c, 2);

    // This demonstrates an indirect call to a function passed as a callback
    // across the FFI boundary with the Rust compiler and Clang using different
    // encodings for the passed-callback declaration and the test at the
    // indirect call site at indirect_call_from_c (i.e., "_ZTSFvu3i32E" or
    // "_ZTSFvu3i64E" vs "_ZTSFvlE").
    //
    // When Rust functions are passed as callbacks across the FFI boundary to be
    // called back from C code, the tests are also at the call site but
    // generated by Clang instead, so the type ids used in the tests are encoded
    // by Clang, which will currently not match the type ids of declarations
    // encoded by the Rust compiler (e.g., hello_from_rust_again). (The same
    // happens the other way around for C funtions passed as callbacks across
    // the FFI boundary to be called back from Rust code.)
    unsafe {
        indirect_call_from_c(hello_from_rust_again, 3);
    }
}
```

example/src/foo.c
```c
#include <stdio.h>
#include <stdlib.h>

// This definition has the type id "_ZTSFvlE".
void
hello_from_c(long arg)
{
    printf("Hello from C!\n");
}

// This definition has the type id "_ZTSFvPFvlElE"--this may be ignored for the
// purposes of this example.
void
indirect_call_from_c(void (*fn)(long), long arg)
{
    // This call site tests whether the destinatin pointer is a member of the
    // group derived from the same type id of the fn declaration, which has the
    // type id "_ZTSFvlE".
    //
    // Notice that since the test is at the call site and generated by Clang,
    // the type id used in the test is encoded by Clang.
    fn(arg);
}
```

Will need to be changed to:

example/src/main.rs
```rust
use std::ffi::c_long;
use std::ffi::cfi;

// The new set of C types in `core::ffi::cfi` as user-defined types using
// `repr(transparent)` will be equivalent to (using c_long as an example):
//
// pub mod cfi {
//     #[allow(non_camel_case_types)]
//     #[repr(transparent)]
//     pub struct c_long(pub std::ffi::c_long);
// }

#[link(name = "foo")]
extern "C" {
    // This declaration will have the type id "_ZTSFvlE".
    fn hello_from_c(_: cfi::c_long);

    // This declaration will have either the type id "_ZTSFvPFvlEu3i32E" or
    // "_ZTSFvPFvlEu3i64E"--this may be ignored for the purposes of this
    // example.
    fn indirect_call_from_c(f: unsafe extern "C" fn(cfi::c_long), arg: c_long);
}

// This definition will have the type id "_ZTSFvlE".
unsafe extern "C" fn hello_from_rust(_: cfi::c_long) {
    println!("Hello, world!");
}

// This definition will have the type id "_ZTSFvlE".
unsafe extern "C" fn hello_from_rust_again(_: cfi::c_long) {
    println!("Hello from Rust again!\n");
}

// This definition will also have either the type id "_ZTSFvPFvlEu3i32E" or
// "_ZTSFvPFvlEu3i64E"--this may be ignored for the purposes of this example.
fn indirect_call(f: unsafe extern "C" fn(cfi::c_long), arg: c_long) {
    // This indirect call site tests whether the destinatin pointer is a member
    // of the group derived from the same type id of the f declaration, which
    // will have the type id "_ZTSFvlE".
    //
    // Notice that since the test is at the call site and generated by the Rust
    // compiler, the type id used in the test is encoded by the Rust compiler.
    unsafe { f(cfi::c_long(arg)) }
}

// This definition has the type id "_ZTSFvvE"--this may be ignored for the
// purposes of this example.
fn main() {
    // This demonstrates an indirect call within Rust-only code using the same
    // encoding for hello_from_rust and the test at the indirect call site at
    // indirect_call (i.e., "_ZTSFvlE").
    indirect_call(hello_from_rust, 1);

    // This demonstrates an indirect call across the FFI boundary with the Rust
    // compiler and Clang using the same encoding for hello_from_c and the test
    // at the indirect call site at indirect_call (i.e., "_ZTSFvlE").
    indirect_call(hello_from_c, 2);

    // This demonstrates an indirect call to a function passed as a callback
    // across the FFI boundary with the Rust compiler and Clang using the same
    // encoding for the passed-callback declaration and the test at the indirect
    // call site at indirect_call_from_c (i.e., "_ZTSFvlE").
    unsafe {
        indirect_call_from_c(hello_from_rust_again, 3);
    }
}
```

example/src/foo.c
```c
#include <stdio.h>
#include <stdlib.h>

// This definition has the type id "_ZTSFvlE".
void
hello_from_c(long arg)
{
    printf("Hello from C!\n");
}

// This definition has the type id "_ZTSFvPFvlElE"--this may be ignored for the
// purposes of this example.
void
indirect_call_from_c(void (*fn)(long), long arg)
{
    // This call site tests whether the destinatin pointer is a member of the
    // group derived from the same type id of the fn declaration, which has the
    // type id "_ZTSFvlE".
    //
    // Notice that since the test is at the call site and generated by Clang,
    // the type id used in the test is encoded by Clang.
    fn(arg);
}
```

Direct calls to extern "C" function types across the FFI boundary, whether CFI
is enabled or disabled, will continue to work whether Rust integer types or C
type aliases are used.

For example:

example/src/main.rs
```rust
// Optionally, use std::ffi::c_long. (Note this is the C type alias, not
// the new C type.)

#[link(name = "foo")]
extern "C" {
    // This declaration will have the type id "_ZTSFvu3i64E".
    fn hello_from_c(_: i64);
    // This declaration will have either the type id "_ZTSFvu3i32E" or
    // "_ZTSFvu3i64E".
    // Or fn hello_from_c(_: c_long). (Note this is the C type alias,
    // not the new C type.)
}

// This definition has the type id "_ZTSFvvE"--this may be ignored for the
// purposes of this example.
fn main() {
    // This will continue to work because direct call sites do not test type
    // membership.
    unsafe { hello_from_c(1); }
}
```

example/src/foo.c
```c
#include <stdio.h>
#include <stdlib.h>

// This definition has the type id "_ZTSFvlE".
void
hello_from_c(long arg)
{
    printf("Hello from C!\n");
}
```

Will continue to work when `fn hello_from_c(_: i64)` or `fn hello_from_c(_:
c_long)` represents a `void hello_from_c(long arg)` in an LP64 or equivalent
data model.

# Drawbacks
[drawbacks]: #drawbacks

The Rust compiler assumes that C char and integer types and their respective
Rust aliased types can be used interchangeably. These assumptions can not be
maintained for extern "C" function types indirectly called across the FFI
boundary when CFI is enabled and the new set of C types are used.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

The alternatives considered were:

  1. creating a new set of C types in `core::ffi::cfi` as user-defined types
     using `repr(transparent)` to be used in extern "C" function types
     indirectly called across the FFI boundary when cross-language CFI support
     is needed, and keeping the existing C-like type aliases.

  2. waiting for the work in progress in rust-lang/rust#97974 for
     rust-lang/compiler-team#504 and use type alias information for
     disambiguation and to specify the corresponding C types in extern "C"
     function types when cross-language CFI support is needed.

  3. adding a new set of parameter attributes to specify the corresponding C
     types to be used in extern "C" function types indirectly called across the
     FFI boundary when cross-language CFI support is needed.

  4. creating a new set of transitional C types in `core::ffi` as user-defined
     types using `repr(transparent)` to be used in extern "C" function types
     indirectly called across the FFI boundary when cross-language CFI support
     is needed (and taking the opportunity to consolidate all C types in
     `core::ffi`).

  5. changing the currently existing C types in `std::os::raw` to user-defined
     types using `repr(transparent)`.

  6. changing C types to `ty::Foreign` and changing `ty::Foreign` to be able to
     represent them.

  7. creating a new `ty::C` for representing C types.

Alternatives (1), (2), and (3) are opt in for when cross-language CFI support is
needed. These alternatives are not backward-compatibility breaking changes
because the Rust compiler currently does not support cross-language CFI (i.e.,
extern "C" function types indirectly called across the FFI boundary when CFI is
enabled).

Alternatives (4), (5), (6), and (7) are backward-compatibility breaking changes
because they will require changes to existing code that use C types.

The solution this RFC proposes (1) is opt in, is not a backward-compatibility
breaking change, and is one of the less intrusive changes to the language among
the alternatives listed.

# Prior art
[prior-art]: #prior-art

The author is currently not aware of any cross-language CFI implementation and
support by any other compiler and language.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

None.

# Future possibilities
[future-possibilities]: #future-possibilities

The project this RFC is part of and solving the issue this RFC describes
provides the foundation for cross-language CFI support for the Linux kernel
(i.e., cross-language kCFI support) and Intel Fine Indirect Branch Tracking
(FineIBT), which use the same encoding and also depend on solving the issue this
RFC describes.

It also provides the foundation for future implementations of cross-language
hardware-assisted and software-based -combined forward-edge control flow
protection, such as Microsoft Windows eXtended Flow Guard (XFG) and ARM Pointer
Authentication -based forward-edge control flow protection, that also depend on
the Rust compiler being able to identify C char and integer type uses at the
time types are encoded.

# Acknowledgment

Thanks to pnkfelix (Felix Klock) and the Rust community for all their help on
this RFC.

# Appendix
[appendix]: #appendix

As the industry continues to explore Rust adoption, the absence of support for
forward-edge control flow protection in the Rust compiler is a major security
concern when migrating to Rust by gradually replacing C or C++ with Rust, and C
or C++ and Rust -compiled code share the same virtual address space.

A safe language -compiled code such as Rust, when sharing the same virtual
address space with an unsafe language -compiled code such as C or C++, may
degrade the security of a program because of different assumptions about
language properties and availability of security features such as exploit
mitigations.

The issue the project this RFC is part of aims to solve is an example of this,
where entirely safe Rust-compiled code, when sharing the same virtual address
space with C or C++ -compiled code with forward-edge control flow protection,
may degrade the security of the program because the indirect branches in
Rust-compiled code are not validated, allowing forward-edge control flow
protection to be trivially bypassed.

This has been extensively discussed[[6]][[7]][[8]][[9]][[10]], and just recently
formalized[[11]] as a new class of attack (i.e., cross-language attacks). It was
also one of the major reasons that initiatives such as Rust GCC--which this
author also fully support--were funded[[10]]. Therefore, support for
forward-edge control flow protection needs to be added to the Rust compiler and
is a requirement for large-scale secure Rust adoption.

# Frequently asked questions (FAQ)
[faq]: #faq

## Are the changes proposed in this RFC backward-compatibility breaking changes?

These are not backward-compatibility breaking changes because the Rust compiler
currently does not support cross-language CFI (i.e., extern "C" function types
indirectly called across the FFI boundary when CFI is enabled).

## Why not use the v0 mangling scheme for encoding?

The v0 mandling scheme can not be used because it is not a compatible encoding
for cross-language LLVM CFI support.

## Why not create a new encoding for cross-language CFI?

See Using Itanium C++ ABI mangling for encoding (1) versus creating a new
encoding for cross-language CFI (2) in the design document in the tracking issue
[#89653][1][[1]].

## Why not use a generalized encoding across the FFI boundary?

This results in less comprehensive protection, may result in using a generalized
encoding for all C and C++ -compiled code instead of only across the FFI
boundary depending whether Clang can be changed to use the generalized encoding
only across the FFI boundary (which may also require new Clang extensions and
changes to C and C++ code and libraries), and will degrade the security of the
program when linking foreign Rust-compiled code into a program written in C or
C++ because the program previously used a more comprehensive encoding for all
its compiled code.

## Why not use hardware-assisted forward-edge control flow protection?

Newer processors provide hardware assistance for forward-edge control flow
protection, such as ARM Branch Target Identification (BTI), ARM Pointer
Authentication, and Intel Indirect Branch Tracking (IBT) as part of Intel
Control-flow Enforcement Technology (CET). However, ARM BTI and Intel IBT -based
implementations are less comprehensive than software-based implementations such
as [LLVM ControlFlowIntegrity
(CFI)](https://clang.llvm.org/docs/ControlFlowIntegrity.html), and the
commercially available [grsecurity/PaX Reuse Attack Protector
(RAP)](https://grsecurity.net/rap_faq).

## What do you mean by less comprehensive protection?

The less comprehensive the protection, the higher the likelihood it can be
bypassed. For example, Microsoft Windows Control Flow Guard (CFG) only tests
that the destination of an indirect branch is a valid function entry point,
which is the equivalent of grouping all function pointers in a single group, and
testing all destinations of indirect branches to be in this group. This is also
known as "coarse-grained CFI".

(This is even less comprehensive than the initial support for LLVM CFI added to
the Rust compiler as part of the project this RFC is also part of, which
aggregated function pointers in groups identified by their number of parameters
[i.e., rust-lang/rust#89652], and provides protection only for the first example
listed in the partial results in the design document in the tracking issue
[#89653][1][[1]])

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
compiler as part of the project this RFC is also part of, which aggregates
function pointers in groups identified by their return and parameter types
[i.e., rust-lang/rust#95548]. See the partial results in the design document in
the tracking issue [#89653][1][[1]].)

[1]: <https://github.com/rust-lang/rust/issues/89653> "R. de C Valle. \"Tracking Issue for LLVM Control Flow Integrity (CFI) Support for Rust #89653.\" GitHub."
[2]: <https://llvm.org/docs/TypeMetadata.html> "\"Type Metadata.\" LLVM Documentation."
[3]: <https://itanium-cxx-abi.github.io/cxx-abi/abi.html> "\"Itanium C++ ABI\"."
[4]: <https://itanium-cxx-abi.github.io/cxx-abi/abi.html#mangling-special-vtables> "\"Virtual Tables and RTTI\". Itanium C++ ABI."
[5]: <https://rustc-dev-guide.rust-lang.org/ty.html> "\"The ty module: representing types\". Guide to Rustc Development."
[6]: https://stanford-cs242.github.io/f17/assets/projects/2017/songyang.pdf "Y. Song. \"On Control Flow Hijacks of unsafe Rust.\" GitHub."
[7]: https://www.cs.ucy.ac.cy/~elathan/papers/tops20.pdf "M. Papaevripides and E. Athanasopoulos. \"Exploiting Mixed Binaries.\" Elias Athanasopoulos Publications."
[8]: https://github.com/rust-lang/rust/files/4723836/Control.Flow.Guard.for.Rust.pdf "A. Paverd. \"Control Flow Guard for Rust.\" GitHub."
[9]: https://github.com/rust-lang/rust/files/4723840/Control.Flow.Guard.for.LLVM.pdf "A. Paverd. \"Control Flow Guard for LLVM.\" GitHub."
[10]: https://opensrcsec.com/open_source_security_announces_rust_gcc_funding "B. Spengler. \"Open Source Security, Inc. Announces Funding of GCC Front-End for Rust.\" Open Source Security."
[11]: https://www.ndss-symposium.org/wp-content/uploads/2022-78-paper.pdf "S. Mergendahl, N. Burow, H. Okhravi. \"Cross-Language Attacks.\" NDSS Symposium 2022."
