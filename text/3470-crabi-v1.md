- Feature Name: `crabi1`
- Start Date: 2023-07-26
- RFC PR: [rust-lang/rfcs#3470](https://github.com/rust-lang/rfcs/pull/3470)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Provide the initial version of a new ABI and in-memory representation
supporting interoperability between high-level programming languages that have
safe data types.

Provide the infrastructure to support and evolve towards future versions.

This work was already part of a
[compiler MCP](https://github.com/rust-lang/compiler-team/issues/631), but that
MCP just proposed the concept and requirements and goals of crABI, not a
concrete ABI. This RFC defines the initial version of the concrete ABI.

# Motivation
[motivation]: #motivation

Today, developers building projects incorporating multiple languages, or
calling a library written in one language from another, often have to use the C
ABI as a lowest-common-denominator for cross-language function calls. As a
result, such cross-language calls use unsafe C representations, even for types
that both languages understand. For instance, passing a string from Rust to
another high-level language will typically use an unsafe C `char *`, even if
both languages have a safe type for counted UTF-8 strings.

For popular pairs of languages, developers sometimes create higher-level
binding layers for combining those languages. However, the creation of such
binding layers requires one-off effort between every pair of programming
languages. Such binding layers also add work and overhead to the project for
each pair of languages, and may not play well together when using more than one
in the same project.

Furthermore, higher-level data types such as `Option` and `Result` currently
require translation into C-ABI-compatible types, which discourages the use of
such types in cross-language interfaces, and encourages the use of more complex
and less safe encodings (e.g. manually encoding `Option` via an invalid value
of a parameter).

Finally, *system* libraries and other shared libraries typically use the C ABI
as well. Software making a Linux `.so`, Windows DLL, or macOS `dylib`, will
typically expose a C-compatible ABI, and cannot easily provide a higher-level
safe ABI without shipping language-specific high-level bindings.

crABI defines a common way to make calls across high-level languages, passing
high-level data types, without dropping to the lowest common denominator of C.
crABI will work with any language providing a C-compatible FFI (including C
itself), and languages can also add specific higher-level native support for
crABI.

crABI aims to be a reasonable default for compiled libraries in both static and
dynamic form, including system libraries.

This proposal provides version 1 of crABI, with basic support for various
common data types. Future versions will support additional data types and
functionality.

# C-based ABI
[c-based-abi]: #c-based-abi

crABI uses the target's C ABI as a base. For example, on x86-64 Linux, crABI
uses the [x86-64 psABI](https://gitlab.com/x86-psABIs/x86-64-ABI) as a base,
just like `extern "C"` does.

Any type whose ABI is already defined by C will be passed through crABI
identically. Types defined by crABI that the C ABI does not support will be
translated into a representation using types the C ABI supports (potentially
indirectly via other crABI-supported types).

crABI does not extend the target C ABI in any ways that would require the use
of target-specific assembly language to support, such as the use of CPU flags
or registers not already defined by the C ABI. Doing so would require specific
enablement work in other languages or runtimes, rather than just building on
existing support for C FFI.

# Rust language interface and interaction with other features
[language-interface]: #language-interface

The Rust implementation of crABI will include:
- An ABI for defining functions or referencing external functions:
  `extern "crabi"`.
- A repr for laying out data structures (`struct`, `union`, `enum`) compatible
  with crABI: `repr(crabi)`.
- A new lint `improper_crabi_types`, analogous to `improper_ctypes`. This lint
  defaults to `deny`.

crABI structures and functions also support any structure defined using
`repr(C)`, as well as enums that have a discriminant type specified.

crABI structures and functions support types defined using `repr(transparent)`
if they would support the type of the field.

For an enum, the crABI `repr` allows additionally specifying the discriminant
type: `repr(crabi, u8)`. If an `enum` specifies `repr(crabi)` but does not
specify a discriminant type, the enum is guaranteed to use the smallest
discriminant type that holds the maximum discriminant value used by a variant
in the enum. (This differs from the behavior of `repr(C)` enums without a
discriminant type.)

crABI supports passing and storing pointers to `repr(Rust)` structures, to
allow using these opaque pointers as handles, but does not support passing such
structures by value.

crABI works in combination with `repr(align)`, with the same meaning.

crABI does not support `repr(packed)`.

# crABI versioning and evolution
[crabi-versioning-and-evolution]: #crabi-versioning-and-evolution

crABI has has a major and minor version number, similar to semver.

The major version number will change if crABI changes *incompatibly*; for
instance, crABI 2.0 would not necessarily interoperate with crABI 1.3.

The minor version number will change if crABI changes *compatibly*, such that
newer callers can understand and work with older functions and structures.
Older callers cannot necessarily understand and work with newer functions and
structures, unless the newer side restricts itself to features understood by an
older version.

This RFC defines crABI 1.0.

Versions of crABI will start out as unstable features, and follow the usual
Rust stabilization process. As with any other nightly feature, versions of
crABI may, prior to stabilization, change in ways incompatible with prior
*nightly-only* implementations of that version of crABI. Versions of crABI
should not be considered stable until available in stable Rust.

Future versions of crABI may also establish allow-by-default lints for the use
of features newer than a particular crABI version; such lints will allow
libraries or modules exposing an ABI to restrict themselves to versions of
crABI compatible with specific other software (e.g. languages or runtimes) they
want to interoperate with.

An implementation of crABI should document which version of crABI it
implements, which compactly conveys supported and unsupported functionality.

# Pointers, ownership, and allocators
[pointers-ownership-and-allocators]: #pointers-ownership-and-allocators

crABI passes certain types over the ABI as pointers. crABI specifies whether
the translation passes ownership of the pointer or not. Thus, the Rust type
signature conveys whether a pointer is owned or borrowed. However, other
languages implementing crABI will not necessarily make this distinction at the
type level, making it the responsibility of the API documentation (or API
documentation generator) to document whether a pointer is passed/returned as
owned or borrowed.

Owned objects require the recipient to free them when done with them. crABI 1.0
does not specify the precise mechanism for making a `free` function for the
allocator available to the recipient. It is the responsibility of the API
designer to document how to free owned values, whether by guaranteeing which
allocator allocated them (e.g. the system allocator), or by exporting an
appropriate `free` function that passes ownership back.

Lifetimes are incredibly valuable for checking the correctness of pointer
usage, but as lifetimes only exist at compile time, they are not represented in
the ABI. It's the responsibility of a function declaration to define lifetimes
correctly, or to document them in the case of an implementation that doesn't
model lifetimes in its type system.

The crABI ABI also does not encode whether a pointer is mutable or not; both
are passed equivalently. It's the responsibility of API documentation (or API
documentation generators) to reflect whether a pointer permits mutation. Any
language supporting a distinction between mutable and immutable pointers should
reflect this when declaring a crABI type or function.

# Types
[types]: #types

For each type, crABI specifies the translation into C types, either directly or
via other crABI-supported types. Using the type in a struct, union, enum field,
function parameter, or function return value, results in a C-compatible
function equivalent to using the specified translation to C types in place of
the crABI-supported type. This includes the size and alignment of the
underlying C types.

Note that C supports passing and returning structures by value, and the ABIs of
of major targets support passing and returning structures containing multiple
fields in a manner as efficient as passing multiple parameters, such as by
passing or returning multiple struct fields via multiple registers. Thus, crABI
often uses representations involving structures passed or returned by value.
This has the net effect of grouping these together into multiple types, without
introducing unnecessary indirection.

Some crABI representations use an underlying C type but do not use the full
value space of that type. In that case, passing or returning values outside the
valid range invokes undefined behavior. Programs may assume that no values
outside that range are passed or returned, and in particular the compiler may
generate code that does not check this assumption, or may optionally include
validation assertions when debugging.

## `char` - Unicode character type

crABI supports Unicode characters, which in Rust use the `char` type. This
translates to the C `uint32_t` type. This type will never contain a value
larger than 0x10FFFF, or a value in the range 0xD800 to 0xDFFF inclusive.

Note that there is no special handling for an array of values of this type,
which is not equivalent to a string (unless using a UCS-4 encoding).

## Zero-sized types (e.g. `()` and `PhantomData`)

crABI supports zero-sized types, such as `()` and `PhantomData`, as long as
they have an alignment of 1. These types have only one value, and require zero
bits to convey, so they are not passed through the ABI at all, and do not
appear in the translated C types or functions.

Pointers and references to zero-sized types are valid in crABI, and are passed
like any other pointer or reference to an opaque Rust type.

## `&T` and `&mut T` - borrowed reference

As with `extern "C"`, crABI supports references (immutable and mutable) to
types. These do not pass ownership of the value, only a pointer to the value.
(Note that crABI 1.0 does not have lifetime information; the provider of an API
must document the lifetime of a reference, and the recipient of the reference
must handle it accordingly.)

Both types of reference are passed as a pointer to the underlying type. The
pointer must never be null.

## Tuple

crABI supports tuples with arbitrarily many fields. These are translated to a
by-value C structure containing fields of the specified types in the same
order. For example:

```rust
extern "crabi" fn func(a: u32, t: cr#(u64, u16), b: i8) -> cr#(u8, u32);
```

is equivalent to:

```c
struct func_t_arg {
    uint64_t f1;
    uint16_t f2;
};
struct func_ret {
    uint8_t f1;
    uint32_t f2;
};
extern struct func_ret func(uint32_t a, struct func_t_arg t, int8_t b);
```

Note that crABI tuples are not the same type as Rust tuples, as Rust tuples do
not guarantee a stable layout.

## `&[T]` or `&mut [T]` - borrowed slice

crABI supports slices (references to arrays) as long as crABI supports the
element type.

Slices translate to a by-value struct containing two fields: a pointer to the
element type, and a `size_t` number of elements, in that order.

For instance:

```rust
extern "crabi" fn func(buf: &mut [u8]);
```

is equivalent to:

```c
struct u8_slice {
    uint8_t *data;
    size_t len;
};
extern void func(struct u8_slice buf);
```

## `&str` - borrowed counted UTF-8 string

crABI supports `&str`, a borrowed counted UTF-8 string. This type must always
contain valid UTF-8. (Use a different type for non-UTF-8 strings.) These are
not typically NUL-terminated.

crABI handles this type equivalently to a byte slice `&[u8]`.

## `Box<[T]>` - boxed slice

crABI supports passing ownership of a slice (an array with size determined at
runtime). To represent this via the C ABI, crABI passes this equivalently to a
C struct containing an element pointer and a `size_t` number of elements, in
that order.

For instance:

```rust
extern "crabi" fn func(data: Box<[u32]>) -> Box<[f64]>;
```

is equivalent to:

```c
struct u32_boxed_slice {
    uint32_t *data;
    size_t len;
};
struct f64_boxed_slice {
    double *data;
    size_t len;
};
extern struct f64_boxed_slice func(struct u32_boxed_slice data);
```

## `Box<str>` - boxed counted UTF-8 string

crABI supports passing an owned string. To represent this via the C ABI, crABI
passes this equivalently to a boxed slice of type `Box<[u8]>`.

## `[T; N]` - fixed-size array by value

crABI supports passing a fixed-size array by value (as opposed to by
reference). To represent this via the C ABI, crABI treats this equivalently to
a C array of the same type passed by value.

Note that this means C code can use an array directly in a context where it
will be interpreted by-value (such as in a struct field), but needs to use a
structure with a single array field in contexts where it would otherwise be
interpreted as a pointer (such as in a function argument or return value).

For instance:

```rust
extern "crabi" fn func(rgb: [u16; 3])
```

is equivalent to:

```c
struct func_rgb_arg {
    uint16_t array[3];
};
extern void func(struct func_rgb_arg rgb);
```

Note that crABI does *not* pass the length, since it's a compile-time constant;
the recipient must also know the correct size. (Use one of the slice-based
types for a type with a runtime-determined length.)

## `&[T; N]` - fixed-size array by reference

crABI supports passing a fixed-size array by reference. crABI represents this
as a pointer to the element type.

C can represent this as an array (e.g. `uint16_t rgb[3]`) in contexts where
that already implies passing by pointer (such as a function argument or return
value), or translate it explicitly to a pointer (e.g `uint16_t (*rgb)[3]`) in
contexts where just writing the array would imply by-value (such as a struct
field).

## Arbitrary `enum` types

crABI supports arbitrary `enum` types, if declared with `repr(crabi)`. These
are always passed using the same layout that Rust uses for enums with `repr(C)`
and a specified discriminant type:
<https://doc.rust-lang.org/reference/type-layout.html#combining-primitive-representations-of-enums-with-fields-and-reprc>

This layout consists of the discriminant, followed by a union of the
representations of each `enum` variant that has fields.

If an `enum` specifies `repr(crabi)` but does not specify a discriminant type,
the `enum` is guaranteed to use the smallest discriminant type that holds the
maximum discriminant value used by a variant in the `enum`.

If the `enum` has no fields, or no fields with a non-zero size, crABI will
represent the `enum` as only its discriminant.

### Guaranteed niche optimization
[niche]: #niche

As a special case, if an `enum` using `repr(crabi)` has exactly two variants,
one of which has no fields and the other of which has a single field, and the
single field type has a specific type (defined below) with a "niche" value,
then the `enum` is supported in crABI, and the `enum` representation uses
"niche optimization" to have the same size as the field. For instance,
`core::crabi::Option<T>` uses the niche optimization if `T` is one of the types
supporting niche optimization.

As another special case, if an `enum` using `repr(crabi)` has exactly two
variants, both of which have a single field, and one of the two fields has type
`()`, the `enum` is supported in crABI, and crABI applies the niche
optimization to make the enum the same size as the field in the other variant.
For instance, `core::crabi::Result<(), E>` uses the niche optimization if `E`
is one of the types supporting niche optimization, and
`core::crabi::Result<T, ()>` uses the niche optimization if `T` is one of the
types supporting niche optimization.

`core::crabi::Option` and `core::crabi::Result` are analogues to the standard
`Option` and `Result` types, but defined with `repr(crabi)` and guaranteed to
only use niche optimizations specified by crABI. These types support `?` and
convenient bidirectional conversions to/from the Rust `Option` and `Result`
types.

As crABI must specify a precise stable layout, crABI's niche optimization only
applies to the types explicitly listed in the crABI specification, even though
the same type in the Rust ABI may be able to apply niche optimizations in more
cases.

Types that permit niche optimization (using `crabi::Option` as an example and
using the niche value to represent `crabi::None`, both of which are abbreviated
unqualified below for readability):
- `Option<&T>`, `Option<&mut T>`, `Option<NonNull<T>>`, `Option<Box<T>>`, and
  `Option` of any function pointer type are all passed using a null pointer to
  represent `None`.
- `Option<bool>` is passed using a single `u8`, where 0 is `Some(false)`, 1 is
  `Some(true)`, and 2 is `None`.
- `Option<char>` is passed using a single `u32`, where 0 through `0xD7FF` and
  `0xE000` through `0x10FFFF` are possible `char` values, and `0x110000` is
  `None`.
- `Option` of any of the `NonZero*` types is passed using a value of the
  underlying numeric type with 0 as `None`.
- `Option<OwnedFd>` and `Option<BorrowedFd>` are passed using `-1` to represent
  `None`
- `Option` of a `repr(transparent)` type containing one of the above as its
  only non-zero-sized field will use the same representation.

Note again that crABI does not apply niche optimizations in all the cases that
native Rust types are capable of, only in the specific cases listed above.
Niche optimization also does not apply for `enum` types defined with an
explicit `repr` other than `repr(crabi)`, such as `repr(C)`, *or* to `enum`
types defined with an explicit discriminant, such as `repr(crabi, u8)`; such
`enum` types will *always* use a separate discriminant.

### `core::crabi::Option<T>`

crABI supports passing optional values using `core::crabi::Option` (whose
variants are additionally provided as `core::crabi::None` and
`core::crabi::Some`). See above for details on enums in general and the niche
optimization in particular. When not using the niche optimization,
`crabi::Option` is guaranteed to use a `u8` discriminant, with 0 representing
`crabi::None` and 1 representing `crabi::Some`.

### `core::crabi::Result<T, E>`

crABI supports passing success/failure results using
`core::crabi::Result<T, E>` (whose variants are additionally provided as
`core::crabi::Ok` and `core::crabi::Err`). When not using the niche
optimization, `crabi::Result` has a `u8` discriminant, with 0 representing
`crabi::Ok` and 1 representing `crabi::Err`.

Note that crABI 1.0 does not attempt to define a standardized `Error` type.

# Native support for crABI

crABI is designed to allow any language or tool with a C-compatible FFI layer
to interoperate with it. However languages and tools may still want to add
additional support specifically for crABI, to make crABI even more usable.

In particular, some aspects of crABI define translations of types that involve
generics or the generation of various small one-off data structures, and a
language or tool with native support could handle these more conveniently
without requiring the explicit separate definition of as many structures.

Furthermore, native support for crABI could distinguish between owned and
borrowed types, and handle those automatically in a way that makes sense for
the language or tool. (For instance, native support could adopt owned types
into whatever mechanism the language or tool uses to free objects when done
when them.)

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

Guaranteed niche optmization is the most uncertain part of the proposed crABI
specification. crABI either needs to be layout-compatible with Rust `Option`
and `Result` and tuple types, or needs to use its own distinct layout and thus
distinct type for them, in which case they require conversion to/from the Rust
types. However, Rust does not provide stability guarantees for its niche
optimizations, so crABI cannot simultaneously have a documented ABI (defining
an exact set of specific niche optimizations and no others) and maintain layout
compatibility with Rust: Rust could not then do any further niche optimizations
not already specified by crABI, even in an edition (because types and their
layouts are intercompatible between editions). Thus, this crABI proposal
(reluctantly) specifies *separate* `Option`, `Result`, and tuple types, to
avoid this compatibility hazard.

Making a structure compatible with crABI requires declaring it with either
`repr(crabi)` or `repr(C)`. This makes it difficult to adapt a type from
another library, or to add a crABI interface to an existing Rust library while
preserving its data types.

crABI could (instead of or in addition to `repr(crabi)`) provide a mechanism
that "transforms" a type into the `repr(crabi)` equivalent of that type, to avoid
having to define a separate type. However, this would not handle the problem of
translating between the types, which in the general case of a "deep"
translation could require copying arbitrary in-memory data structures. crABI
does not aim to solve that problem.

The translation of slices and similar uses structs containing pointer/length
pairs, rather than inlining the pointer and length as separate arguments.
[As noted above][types], this is typically passed and returned in an efficient
fashion on major targets. However, in some languages, such as C, this will
require separately defining a structure and then using that structure. This
still seems preferable, though, as combining the two into one struct allows for
uniform handling between arguments, return values, and fields, as well as
keeping the pointer and length more strongly associated.

@programmerjake made a
[proposal](https://github.com/rust-lang/rfcs/pull/3470#issuecomment-1674249638)
([sample usage](https://github.com/rust-lang/rfcs/pull/3470#issuecomment-1674265515))
to modify the standard impl of `Drop` for `Box` to allow plugging in an
arbitrary function (via a `BoxDrop` trait), to drop the `Box` as a whole. This
would be generally useful (e.g. for object pooling), and would then permit
crABI to define a `box_drop` function that calls an FFI function to free the
object. If we accepted that proposal, it would make sense to use it to
represent crABI boxes.

# Prior art
[prior-art]: #prior-art

Some potential sources of inspiration:

- WebAssembly Interface Types
- Swift's stable ABI
- The `abi_stable` crate (which aims for Rust-to-Rust stability, not
  cross-language interoperation, but it still serves as a useful reference)
- `stabby`
- UniFFI
- Diplomat
- C++'s various ABIs (and the history of its ABI changes). crABI does not,
  however, aim for compatibility with or supersetting of any particular C++
  ABI.
- Many, many interface description languages (IDLs).
- The [x86-64 psABI](https://gitlab.com/x86-psABIs/x86-64-ABI). While we're not
  specifying the lowering all the way to specific architectures, we can still
  learn from how it handles various types.
- The [aarch64 ABI](https://github.com/ARM-software/abi-aa/).
- The ABIs of other targets.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- Once we have support for extern types (such as via
  https://github.com/rust-lang/rfcs/pull/3396), pointers and references to
  extern types should work in crABI, just like any other opaque pointer.

- Should the `improper_crabi_types` lint be a hard error, instead?

- Is there a better way we can handle tuple types? Having to use a distinct
  syntax like `cr#()` is onerous; one of the primary values of tuples is
  brevity. In the future, if we have variadic generics, we could potentially
  use a named tuple type, but that only helps with orthogonality, not brevity.
  Or we could guarantee the in-memory layout of tuples *in general* in Rust. We
  could also omit support for tuples entirely, but that seems like an
  unfortunate gap between Rust and crABI.

  We might be able to provide a shorthand, where `repr(crabi)` types and
  `extern "crabi"` functions imply the use of crABI tuples in their type
  signatures, but we'd still need a way of naming the distinct type, and the
  shorthand could introduce confusion in the form of distinct types spelled
  identically.

  Would an approach based on "type transformers" help here?

- The handling of `Option` and `Result` seems similarly unfortunate. We have to
  use a distinct type to avoid freezing the layout of these Rust types for all
  time. But could we do better, somehow? Would an approach based on "type
  transformers" help here?

- Should we provide additional guaranteed niche optimizations for `Option` in
  crABI 1.0? In order to keep new minor revisions of crABI compatible with
  crABI 1.0, any existing type that doesn't have a niche optimization in crABI
  1.0 can never switch to having a niche optimization until a new
  (incompatible) major version, and ideally we want new crABI major versions to
  be rare.

  That said, we should probably keep *most* uses of niche optimizations rare,
  as they require special-case handling in other languages.

- Similarly, should we provide additional guaranteed niche optimizations for
  `Result`? Which ones? The same issue applies: any optimizations we want to
  provide for `Result` on existing types should be in crABI 1.0.

- Should we provide *fewer* niche optimizations? Those for `NonZero` and
  reference types provide obvious value; are those for `bool` and `char` really
  useful enough to justify the special case in languages that will have to
  handle them explicitly rather than automatically?

- crABI specifies supports owned pointers via `Box`, but does not specify how
  to free such objects other than passing them back to the caller. Should we
  represent this in the type system explicitly, such as via a special allocator
  (e.g. `NoDeallocate` to require passing back to the module that allocated it,
  or an FFI deallocator parameterized with a `free` function in the type)? Or a
  `ManuallyDrop`? Or should we use a normal `Box`?

- Should crABI support passing owned values that have a non-trivial `Drop`
  implementation, on the assumption that the API will specify that they must be
  passed back to another exported function for freeing (which could call
  `Drop`)? Or should crABI just prohibit types with non-trivial `Drop`
  implementations?

# Future possibilities
[future-possibilities]: #future-possibilities

- crABI makes it easier to define safer interfaces, making it more likely that
  a library can define an interface that doesn't necessarily require `unsafe`
  to call. Thus, crABI would benefit from a mechanism to allow specifying an
  `extern "crabi"` function as explicitly safe, and then not requiring an
  `unsafe` block to call that function. This RFC does not specify such a
  mechanism, but there have been various proposals for such mechanisms for
  `extern "C"`, and those mechanisms should work equivalently for `extern
  "crabi"`.

- Once `extern "C"` supports C-compatible handling of `u128` and `i128`,
  `extern "crabi"` should do the same.

- Extensible enums. To define types that allow for extension, crABI would
  benefit from a means of defining "extensible" enum types, that capture
  unknown enum discriminants.

- Support for pattern-restricted data types, if defined in the future. These
  would allow defining types restricted to a subset of their value range.

- Allow-by-default lints for specific crABI versions. A library working with a
  specific version of another language or runtime that only supports a specific
  version of crABI could use such lints to avoid unintentionally using features
  requiring a newer version of crABI.

- Support for additional datatypes.
  - Support for operating system paths and strings, other than by using `&[u8]`
    or `Box<[u8]>` (or `&[u16]` or `Box<u16>` on Windows or UEFI). Rust's
    `Path` and `PathBuf` types could easily accommodate this on UNIX targets,
    but on Windows these use WTF-8 encoding and require translation to UTF-16
    for use with Windows APIs. Rust's standard library does not provide a type
    that uses *native* path/string representation for every target.
  - Support for range types. The representation of those in Rust
    relies on generics and traits, to have distinct types for `start..end`,
    `start..=end`, `start..`, `..end`, `..=end`, and `..`, which then all
    implement the same trait `RangeBounds`. This implementation makes some uses
    of ranges more *efficient*, but would make usage via a fixed ABI painful to
    the point of unusability. crABI *could* provide a single unified range type
    (and implement `RangeBounds` for it), but that might be inefficient.

- Support for arbitrary objects and methods on those objects.

- Support for trait objects.

- Support for owned pointers to opaque objects. This might be representable as
  a `Box` with a special allocator that makes an FFI call, or as a
  `ManuallyDrop`. The latter requires less effort to specify, while the former
  may allow for automatic cleanup in languages supporting it.

- Support for objects requiring `Drop` cleanup.

- Support for easily referencing crABI types and functions from a Rust crate in
  another Rust crate, without duplicating any type/function declarations. The
  [`export` proposal](https://github.com/rust-lang/rfcs/pull/3435) would enable
  this.

- Support for a simple IDL, making it easier to generate crABI bindings for
  various languages.
  - This simple IDL could have a representation for lifetimes and owned vs
    borrowed types, allowing tools to check for the correct usage.
