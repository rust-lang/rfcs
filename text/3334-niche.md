- Feature Name: `niche`
- Start Date: 2022-10-16
- RFC PR: [rust-lang/rfcs#3334](https://github.com/rust-lang/rfcs/pull/3334)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Provide a stable attribute to define "niche" values of a type. The type cannot
store these values, allowing the compiler to use them to optimize the
representation of containing types such as `Option<Type>`.

# Motivation
[motivation]: #motivation

Rust makes extensive use of types like `Option`, and many programs benefit from
the efficient storage of such types. Many programs also interface with others
via FFI, via interfaces that provide data and a sentinel value (such as for
errors or missing data) within the same bits.

The Rust compiler already provides support for this via "niche" optimizations,
and various types providing guarantees of such optimizations, including
references, `bool`, `char`, and the `NonZero` family of types. However, Rust
does not provide any stable means of defining new types with niches, reserving
this mechanism for the standard library. This puts pressure on the standard
library to provide additional families of types with niches, while preventing
the broader crate ecosystem from experimenting with such types.

Past efforts to define a stable niche mechanism stalled out due to scope creep:
alignment niches, null-page niches, multiple niches, structures with multiple
fields, and many other valid but challenging ideas (documented in the "Future
possibilities" section). This RFC defines a *simple* mechanism for defining one
common type of niche, while leaving room for future extension.

Defining a niche mechanism allows libraries to build arbitrary types containing
niches, and simplifies handling of space-efficient data structures in Rust
without manual bit-twiddling.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

When defining a struct containing exactly one field, you can attach a `niche`
attribute to the struct to declare a specific value or range of values for that
field as invalid. This promises the compiler that you will never store those
values in that field, which allows the compiler to use those in-memory
representations for different purposes, such as the representation of `None` in
a containing `Option`.

```rust
use std::mem::size_of;

#[niche(value = 42)]
struct MeaninglessNumber(u64);

assert_eq!(size_of::<MeaninglessNumber>(), 8);
assert_eq!(size_of::<Option<MeaninglessNumber>>(), 8);

#[niche(range = 2..)]
struct Bit(u8);

assert_eq!(size_of::<Bit>(), 1);
assert_eq!(size_of::<Option<Option<Option<Bit>>>>(), 1);
```

Constructing a structure with a niche value, or writing to the field of such a
structure, or obtaining a mutable reference to such a field, requires `unsafe`
code. Causing a type with a niche to contain an invalid value (whether by
construction, writing, or transmuting) results in undefined behavior.

If a type `T` contains only a single niche value, `Option<T>` (and other enums
isomorphic to it, with one variant containing `T` and one nullary variant) will
use that value to represent `None` (the nullary variant). If such a `T` is
additionally `repr(transparent)` or `repr(C)` or otherwise permitted in FFI,
`Option<T>` will likewise be permitted in FFI, with the niche value mapping
bidirectionally to `None` across the FFI boundary.

If a type contains multiple niche values, Rust does not guarantee any
particular mapping at this time, but may in the future.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The niche attribute may either contain `value = N` where `N` is an unsigned
integer, or `range = R` where R is a range expression whose endpoints are both
unsigned integers. The unsigned integers may use any integer base
representation (decimal, hex, binary, octal), but must not have a type suffix.
The unsigned integers are interpreted as the bit patterns in memory
corresponding to the representation of the field. For instance, a struct with a
float field could specify one or more NaN values as a niche using the integer
representation of those values.

The attribute `#[niche]` may only appear on a struct declaration. The struct
must contain exactly one field.

The field must have one of a restricted set of types:
- A built-in integer type (iN or uN).
- A built-in floating-point type (fN). (The niche must still be specified using
  the integer representation.)
- A `char`. (The niche uses the integer representation, and gets merged with
  the built-in niches of `char`; if the result after merging would have
  multiple discontiguous niches, the compiler need not take all of them into
  account.)
- A raw pointer. (This allows user-defined types to store a properly typed
  pointer while taking advantage of known-invalid pointer values.)
- A fieldless enum with a `repr` of a primitive integer type.

Declaring a niche on a struct whose field type does not meet these restrictions
results in an error.

Declaring a niche on any item other than a struct declaration results in an
error.

Declaring a niche on a struct containing more or less than one field results in
an error.

Declaring multiple `niche` attributes on a single item, or multiple key-value
pairs within a single `niche` attribute, results in an error.

Declaring a niche on a struct that has any generic parameters results in an
error.

Declaring a range niche with an empty range (e.g. `0..0`) results in a
warn-by-default lint. As with many lints, this lint should be automatically
suppressed for code expanded from a macro.

Declaring a range niche with an invalid range (e.g. `5..0`) results in an
error.

Declaring a niche using a negative value or a negative range endpoint results
in an error. The representation of negative values depends on the size of the
type, and the compiler may not have that information at the time it handles
attributes such as `niche`. The text of the error should suggest the
appropriate two's-complement unsigned equivalent to use. The compiler may
support this in the future.

Declaring a range niche with an open start (`..3`) results in an error, for
forwards-compatibility with support for negative values.

Declaring a niche using a non-literal value (e.g. `usize::MAX`) results in an
error. Constants can use compile-time evaluation, and compile-time evaluation
does not occur early enough for attributes such as niche declarations.

If a type `T` contains multiple niche values (e.g. `#[niche(range = 8..16)]`),
the compiler does not guarantee any particular usage of those niche values in
the representation of types containing `T`. In particular, the
compiler does not commit to making use of all the invalid values of the niche,
even if it otherwise could have.

However, multiple instances of the same identical type (e.g. `Option<T>` and
`Option<T>`) will use an identical representation (whether the type contains a
niche or not). This permits a round-trip between such a value and a byte
representation.

Adding a niche to a type does not change the storage size of the type, even if
the niche might otherwise allow storing fewer bytes. The type still allows
obtaining mutable references to the field, which requires storing valid values
using the same representation as those values would have had without the niche.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

We could allow defining *either* valid or invalid ranges. For instance,
`niche(invalid_range(0..=3))` or `niche(valid_range(4..))`. Different types
could use whichever of the two proved simpler for a given use case. However, in
addition to adding gratuitous complexity and requiring longer names
(`invalid_range` vs `range`), this would double the number of cases when
defining other kinds of niches in the future. For instance, a future syntax for
bit-pattern niches would need to provide both `valid` and `invalid` variants as
well. We could introduce another level of nesting to make this orthogonal, such
as `niche(invalid(range(...)))` and `niche(invalid(range(...)))`, but that
further increases complexity.

Rather than defining the range of *invalid* values, the attribute could define
the range of *valid* values. Different types may find one or the other case
simpler. This RFC chooses to define the range of *invalid* values for three
reasons:
- As an arbitrary choice, because we need to pick one or the other (see above).
- The most common case will be a single invalid value, for which defining
  invalid values results in simpler code.
- This mechanism commonly goes by the name `niche`, and `niche` also refers to
  the invalid value. So, an attribute defining the niche of a type most
  naturally refers to the invalid value.

Note that the compiler already supports having a niche in the middle of a
type's possible values; internally, the compiler represents this by defining a
valid range that wraps around the type's possible values. For instance,
`#[niche(value = 42)]` gets represented internally in the compiler as a valid
range starting at 43 and ending at 41.

We could define *only* single-value niches, not ranges. However, the compiler
already supports ranges internally, and the standard library already makes use
of multi-value ranges, so this seems like an artificial limitation.

We could define only ranges, not single-value niches, and users could express
single-value niches via ranges, such as `0..=0`. However, that makes
single-value niches more verbose to define, and makes mistakes such as `0..0`
more likely. (This RFC suggests a lint to catch such cases, but the syntax
should still attempt to guide users away from that mistake.)

We could guarantee more usage of niches than just a single value; however, this
would constrain the compiler in areas that still see active development.

We could avoid guaranteeing the use of a single-value niche for `Option`;
however, this would eliminate one of the primary user goals for such niches.

We could require types to opt into the guaranteed use of a niche, separately
from declaring a niche. This seems unnecessarily verbose, as well as limiting:
we can't yet provide a full guarantee of all *future* uses we may want to
guarantee, only of the limited single-value uses.

We could implement niches using a lang-item type that uses const generics (e.g.
`Niche<T, const RANGE: std::ops::Range<T>>`. This type would be useful
regardless, and we should likely provide it if we can. However, this RFC
advocates (eventually) building such a type on an underlying language-level
building block like `niche`, and providing the underlying building blocks to
the ecosystem as well.

We could implement niches using a trait `Niche` implemented for a type, with
associated consts for invalid values. If we chose to do this in the future, the
`#[niche(...)]` attribute could become forward-compatible with this, by
generating the trait impl.

We could use a syntax based on patterns, such as `struct S(u8 is 0..=32);` or
`struct S(MyEnum is MyEnum::A | MyEnum::B)`.

# Prior art
[prior-art]: #prior-art

The Rust compiler has supported niches for types like `Option` in various forms
since versions prior to Rust 1.0. In particular, Rust 1.0 already guaranteed
that `Option<&T>` has the same size as `&T`. Rust has had many additional
niche-related optimizations since then.

The Rust compiler already supports user-defined niches via the unstable
attributes `rustc_layout_scalar_valid_range_start` and
`rustc_layout_scalar_valid_range_end`.

Bit-twiddling tricks to store information compactly have seen widespread use
and innovation since computing antiquity.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

Could we support niches on generic types? For instance, could we support
declaring a niche of `0` on a generic structure with a single field?

Could we support negative numbers in a niche attribute, at least for fields of
concrete primitive type? That would provide a much more friendly interface, but
would require the compiler to better understand the type and its size.

Will something go wrong if applying a niche to a struct whose field is itself a
struct containing multiple fields? Do we need to restrict niches to structs
containing primitive types, or similar?

Do we need to make `niche` mutually exclusive with `packed`? What about other
attributes?

# Future possibilities
[future-possibilities]: #future-possibilities

Niches offer possibilities as vast, rich, clever, and depraved as the
collective ingenuity of bit-twiddlers everywhere. This section includes many
possibilities that have come up in the past. This RFC deliberately excludes all
of these possibilities from the scope of the initial version, choosing to
specify only behavior that the Rust compiler already implements.

New types of niches can use the same `niche` attribute, adding new key-values
within the attribute.

- **Signed values**: This RFC requires the use of unsigned values when defining
  niches. A future version could permit the use of signed values, to avoid
  having to manually perform the twos-complement conversion. This may
  require either making the compiler's implementation smarter, or using a
  syntax that defines the size of the integer type (e.g. `-1isize`).
- **Limited constant evaluation**: This RFC excludes the possibility of using
  constants in the range expression, because doing so simplifies the
  implementation. Ideally, a future version would allow ranges to use at least
  *simple* numeric constants, such as `usize::MAX`. Full constant evaluation
  may be much harder to support.
- **Alignment niches**: If a pointer requires a certain alignment, any bit pattern
  corresponding to an unaligned pointer could serve as a niche. This provides
  an automatic mechanism for handling "tagged pointers" using the low bits.
- **Null-page niches**: If a target treats the entire null page as invalid,
  pointers on that target could have a niche corresponding to that entire page,
  rather than just the null value. This would allow defining niches spanning a
  large swath of the value space. However, this would either require extensive
  use of `cfg_attr` for various targets, or a new mechanism for obtaining the
  valid range from the compiler. In addition, for some targets the valid range
  may vary based on environment, even for the same target; in such cases, the
  compiler would need to provide a mechanism for the user to supply the valid
  range *to* the compiler.
- **Invalid-pointer niches**: On targets where certain pointer values cannot
  represent a valid pointer in a given context (such as on x86-64 where the
  upper half of the address space represents kernel-space address and the lower
  half represents userspace addresses), types containing such pointers could use
  a large swathe of values as a niche.
- **Pointer high-bit niches**: On targets that don't permit addresses with some of
  the high bits set (such as implicitly on historical x86 or ARM platforms, or
  explicitly defined via ARM's "top-byte ignore" or AMD's "upper address
  ignore" or Intel's "Linear Address Masking"), types containing pointers could
  potentially use values with those high bits set as a niche. This would likely
  require compile-time configuration.
- **Multiple niches**: A type could define multiple niches, rather than just a
  single range.
- **Other bit-pattern niches**: A type could define niches via a bit pattern,
  rather than a range.
- **Per-field niches**: A structure containing multiple fields could have a
  niche on a specific field, rather than the whole structure.
- **structs with ZST fields**: A struct could contain fields with zero-sized
  types (e.g. `PhantomData`) and still have a niche.
- **Fields of reference type**: In addition to allowing raw pointers, structs
  with niches could allow references. In practice, if the references have a
  lifetime other than `'static`, this will also require at least some support
  for generic parameters.
- **Non-primitive fields**: A struct could contain fields of non-primitive
  types, such as tuples, arrays, or other structs (including structs with
  niches themselves). This should wait until after niches support providing
  values with the type of the field, rather than as an unsigned integer.
- **Whole-structure niches**: A structure containing multiple non-zero-sized
  fields could have a niche of invalid values for the whole structure.
- **Union niches**: A union could have a niche.
- **Enum niches**: An enum or an enum variant could have a niche.
- **Specified mappings into niches**: Users may want to rely on mappings of
  multiple values into a multi-value niche. For instance, users could define a
  type with a niche containing a range of integer values, and a range of
  integer error codes, and rely on `Result<T, E>` assigning specific niche
  values to specific error codes, in order to match a specific ABI (such as the
  Linux kernel's `ERR_PTR`).
- **Safety**: The attribute specified in this RFC requires an unsafe block to
  set the field. Future extensions could allow safely setting the field, after
  verifying in a compiler-visible manner that the value works. For instance:
- **`derive(TryInto)`**: Rust could support deriving `TryInto` from the
  contained type to the structure. The implementation could explicitly check
  the range, and return an error if not in-range. This would avoid the need to
  write explicit `unsafe` code, and many uses may be able to elide or coalesce
  the check if the compiler can prove the range of a value at compile time.
- **Lints**: Multiple lints may help users define niches, or detect usages of
  niches that may be better expressed via other means. For instance, a lint
  could detect a newtype whose constructor maintains a range invariant, and
  suggest adding a niche.
- **Range types**: Rust (or libraries built atop Rust) could provide integer
  types with associated valid ranges, along with operations that
  expand/contract/propagate those ranges as appropriate.
- **`unsafe` fields**: If in the future Rust introduces `unsafe` fields,
  declaring a niche could internally mark the field as unsafe, taking advantage
  of the same machinery.
- **read-only fields**: If in the future Rust introduces read-only fields,
  types with a niche may wish to provide read-only access to the value they
  contain, rather than just providing conversion methods or traits.
- **Move types, or types that don't support references**: Rust currently
  requires that all values of a given type have the same representation no
  matter where they get stored, to allow taking references to such types and
  passing them to contexts that don't know about any relevant storage quirks
  such as niches. Given a mechanism for disallowing references to a type and
  requiring users to copy or move it rather than referencing it in-place, Rust
  could more aggressively optimize storage layout, such as by renumbering enum
  values and translating them back when read, or by storing fields using fewer
  bytes if their valid range requires fewer bytes to fully represent.
