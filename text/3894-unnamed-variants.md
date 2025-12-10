- Feature Name: (`unnamed_enum_variants`)
- Start Date: 2025-12-09
- RFC PR: [rust-lang/rfcs#3894](https://github.com/rust-lang/rfcs/pull/3894)
- Rust Issue:
  [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

## Summary

Enable ranges of enum discriminants to be reserved ahead of time, requiring
all users of that enum to consider those values as valid. This includes within
the declaring crate.

`_ = RANGE` is an _unnamed variant_ definition. It specifies that enum
discriminants in `RANGE` are valid. It is sound to construct unnamed variants
with `unsafe`, and to handle them over FFI. If there is no invalid discriminant
for an enum, it becomes an _open enum_. If it is [unit-only], it can then be
`as` cast from its explicit underlying integer.

[unit-only]: https://doc.rust-lang.org/reference/items/enumerations.html#r-items.enum.unit-only

## Motivation

Enums in Rust have a _closed_ representation, meaning the only valid
representations of that type are the variants listed, with any violation of this
being [Undefined Behavior][ub]. This is the right default for Rust, since it
enables niche optimization and ensures values have a known state, limiting
unnecessary or dangerous code paths.

[ub]: https://doc.rust-lang.org/reference/behavior-considered-undefined.html

However, a closed enum is not always the best choice for systems programming.
The issue lies with compatibility between existing binaries. There are many
cases in which code is expected to handle non-yet-known enum values as a
non-error.

Consider a complex system that initially uses this `TaskState` enum to
communicate:

```rust
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
/// `TaskState` v1
pub enum TaskState {
    Stopped = 0,
    Running = 1,
}
```

`non_exhaustive` is specified for forwards compatibility, since it should be a
non-breaking change for variants to be added to `TaskState`. This works by
requiring foreign crates to include a wildcard branch when `match`ing. Once a
new `Paused` variant is added to `TaskState`, any code that previously compiled
when using the `TaskState` will continue to do so. However, if any part of the
system is _not_ recompiled, that old code will see the `Paused` variant as
invalid.

```rust
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
/// `TaskState` v2
pub enum TaskState {
    Stopped = 0,
    Running = 1,
    // A new valid discriminant for `TaskState` has been introduced!
    Paused = 2,
}
```

What if it isn't feasible to recompile **every** part of the system that uses
the enum in order to avoid the breaking change?

```rust
/// `TaskState` v1 reserves discriminants instead of using `non_exhaustive`.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TaskState {
    Stopped = 0,
    Running = 1,
    // There are reserved variants for the rest of the discriminants:
    // The `_` resembles a wildcard seen when `match`ing.
    _ = ..,
}
```

If every binary is using this definition, it is not an breaking change for
existing binaries using this definition to add `Paused = 2`. The `_ = ..` has
required _every_ exhaustive `match` of `TaskState`, including in the defining
crate, to handle the case where it's not one of the currently-named variants.

### Protobuf

Protocol Buffers (Protobuf), a language-neutral serialization mechanism, is
designed to be forwards and backwards compatible when extending a schema.
Initially, it defined all of its enums as closed. However, this caused confusing
and often incorrect behavior with `repeated` enums, and so the `proto3` syntax
[switched to open enums][protobuf-history]. Handling unknown values
transparently comes up often in microservices where incremental rollouts cause
schema version skew.

[protobuf-history]: https://protobuf.dev/programming-guides/enum/#history

Protobuf generates code for target languages from a schema. On C++, it can
directly generate an `enum` - C++ enums are open since it's valid to
`static_cast` an `enum` from its backing integer. However, on Rust, the current
implementation simulates an open enum by using an integer newtype with
associated constants for each variant.

While this allows Protobuf enums in Rust to be used _mostly_ like enums, this is
a suboptimal experience.

#### Newtype integers are bad for enumeration

When the point of a type is to give an integer a set of well-known names (like
in C++), a newtype integer isn't as ergonomic to use as an `enum`:

- It is arduous to read the generated definition - the variants are inside of an
  `impl` instead of next to the name. It hides the type's nature as an enum.
- It's invalid to `use` the pseudo-variants like with `use EnumName::*`.
- The third-party macro ecosystem built around enums can't be used.
- Rust is a systems language that can move data around efficiently, and so
  first-class support for named integers is valuable for embedded programmers.
- Code analysis and lints specific to enums are unavailable.
  - No "fill match arms" in rust-analyzer.
  - The [`non-exhaustive patterns` error][E0004] lists only integer values, and
    cannot suggest the named variants of the enum.
  - The unstable [`non_exhaustive_omitted_patterns`] lint has no easy way to
    work with this enum-alike, even though treating it like a `non_exhaustive`
    enum would be more helpful.
- Generated rustdoc is less clear (the pseudo-enum is grouped with `struct`s).
- In order for a pseudo-variant name to match the normal style for an enum
  variant name, `allow(non_uppercase_globals)` is required.
- `derive`s that work with names are less useful. The built-in `derive(Debug)`
  can't know the variant names to list. The `open-enum` crate, which provides an
  attribute macro to construct newtype integers from an `enum` declaration,
  requires a disctinct `derive` ecosystem for operations like `TryFrom`,
  `Debug`, `IsKnownVariant`, ser/de, etc. - a worse experience than if all
  derives were capable of reading a first-class open `enum` definition.

[E0004]: https://doc.rust-lang.org/stable/error-index.html#E0004

If Protobuf instead declared generated Rust enums with a `_ = ..` variant, users
could have a first-class enum experience with compatible open semantics.

### C interop

A closed `#[repr(C)]` field-less `enum`s is [hazardous][repr-c-field-less] to
use when interoperating with C, mostly because it is so easy to trigger
Undefined Behavior when unknown values appear. In C, it is idiomatic to do an
unchecked cast from integer to enum. So, even if one ensures that the C and Rust
libraries are compiled at the same time, they must also audit the C source to
ensure that unknown values cannot be exposed to Rust.

[repr-c-field-less]: https://doc.rust-lang.org/reference/type-layout.html#reprc-field-less-enums

With unnamed variants, the current guidance surrounding sharing enums with C can
thus be simplified greatly: add a `_ = ..` variant and UB from invalid values
aren't a concern.

`bindgen` has [multiple ways][bindgen-enum-variation] to generate Rust that
correspond to a C enum, the default being to define a series of `const` items.
Its best-effort logic to determine the backing integer type for a C enum does
not always match that of `repr(C)` on a Rust `enum`. A future version of
`bindgen` could use this feature to add a `_ = ..` variant to a Rust `enum`  by
default, instead of a exposing a less-effective `non_exhaustive` attribute.

[bindgen-enum-variation]: https://docs.rs/bindgen/0.72.1/bindgen/enum.EnumVariation.html

### Dynamic Linking

Dynamically linked libraries, Rust or otherwise, are prone to ABI compatibility
breakage.

Ensuring ABI compatibility when extending a library requires extra care. While
`non_exhaustive` grants API compatibility as variants are added, it [does _not_
provide ABI compatibility][non-exhaustive-ub]. By reserving discriminants for
future extensions to an enum, libraries can choose to remain ABI
forwards-compatible as new variants are added.

Projects like Redox and relibc would use this feature for this reason among
others listed.

[non-exhaustive-ub]: https://github.com/rust-lang/rust-bindgen/issues/1763

### Embedded syscalls

TockOS is an embedded OS with a separate user space and kernel space. Its
syscall ABI defines that kernel error codes are between 1 and 1024. It's highly
desirable to keep the `0` niche available for `Result<(), ErrorCode>`, so the
user space library defines an [`ErrorCode` enum][libtock-errorcode] with 14
normal variants and 1010 "reserved" variants that will eventually be renamed.
This has drawbacks:

- It clutters the enum definition.
- rust-analyzer's "Fill match arms" inserts a new match arm for each of the
  reserved names, even though a single wildcard branch would be more
  appropriate.
- Since the reserved discriminants have named variants, there's nothing
  preventing users from using the reserved name. There is no perfect way to
  claim a reserved discriminant without breaking the API.
  - Declaring an associated `const` is the way to prevent an API breakage.
  - Moving a reserved variant name like `N00014` to a `deprecated` associated
    `const` is better for readability, but breaks any user that wrote
    `use ErrorCode::N00014`.
  - Declaring the new variant name as an associated `const` is harder to read,
    doesn't interact with code analyzers, and doesn't let users write
    `use ErrorCode::NewVariant`.

[libtock-errorcode]: https://github.com/tock/libtock-rs/blob/master/platform/src/error_code.rs#L30-L33

### Zero-copy deserialization

A common pattern on embedded systems is to read data structures directly from a
`[u8]`, facilitated by libraries like [`zerocopy`][zerocopy-frombytes-derive] or
[`bytemuck`][bytemuck-checkedbitpattern]. In order to do this, the bytes for an
enum must always be validated to be one of the known discriminants.

This scales poorly for performance and code bloat as more enums and variants are
added to be deserialized in a message. It is more flexible to defer wildcard
branches for unknown discriminants to the point when the enum is `match`ed on,
rather than up-front during deserialization. When these checks are undesirable,
ergonomics must be sacrificed for compatibility and performance by using an
integer newtype.

[bytemuck-checkedbitpattern]: https://docs.rs/bytemuck/latest/bytemuck/checked/trait.CheckedBitPattern.html
[zerocopy-frombytes-derive]: https://docs.rs/zerocopy/0.6.1/zerocopy/derive.FromBytes.html

### Restricted range integers

Unnamed variants can be used to define integers that are statically restricted
to a particular range, including with niches.

```rust
macro_rules! make_ranged_int {
    ($name:ident : $repr:ty; $($range:tt)*) => {
        #[repr($repr)]
        enum $name {
            _ = $($range)*,
        }
        impl TryFrom<$repr> for $name {
            type Error = ();
            fn try_from(val: $repr) -> Result<$name, ()> {
                match val {
                    // SAFETY: `val` is a valid discriminant for `$name`
                    $($range)* => Ok(unsafe { mem::transmute(val) }),
                    _ => Err(()),
                }
            }
        }
        impl From<$name> for $repr {
            fn from(val: $name) -> $repr {
                val as $repr
            }
        }
    };
}
make_ranged_int!(FuelLevel: u32; 0..=100);

assert!(size_of::<FuelLevel>() == size_of::<Option<FuelLevel>>());
assert_eq!(FuelLevel::try_from(10).unwrap() as u32, 10);
assert!(FuelLevel::try_from(21).is_err());
```

With other extensions, this could even be generic:

```rust
trait EnumDiscriminant {
    type Ranged<const RANGE: Range<Self>>;
}

impl EnumDiscriminant for u32 {
    type Ranged<const RANGE: Range<Self>> = RangedU32<RANGE>;
}

#[repr(u32)]
enum RangedU32<const RANGE: Range<u32>> {
    _ = RANGE,
}

type Ranged<T, const RANGE: Range<T>> = <T as EnumDiscriminant>::Ranged<RANGE>;

type FuelLevel = Ranged<u32, 0..=100>;
```

[Pattern types][pattern types] are a more direct way to express this.

[pattern types]: https://github.com/rust-lang/rust/pull/107606

## Guide-level explanation

Enums have a _closed_ representation by default, meaning that any enum value
must be represented by one of the listed variants. Constructing any enum value
with an unassigned discriminant is immediate [Undefined Behavior][ub]:

```rust
#[repr(u32)]     // Fruit is represented with specific discriminants of `u32`.
enum Fruit {
    Apple,       // Apple is represented with 0u32.
    Orange,      // Orange is represented with 1u32.
    Banana = 4,  // Banana is represented with 4u32.
}
// Undefined Behavior: 5 is not a valid discriminant for `Fruit`!
let fruit: Fruit = unsafe { core::mem::transmute(5u32) };

// Rust utilizes these invalid discriminants for compiler-dependent
// optimization:
assert_eq!(mem::transmute(Option::<Fruit>::None, 2u32));
```

However, by declaring an **unnamed variant**, the discriminant `5` is _reserved_
and becomes sound to transmute from.

```rust
#[repr(u32)]     // An explicit repr is required to declare an unnamed variant.
enum Fruit {
    Apple,       // Apple is represented with 0u32.
    Orange,      // Orange is represented with 1u32.
    Banana = 4,  // Banana is represented with 4u32.
    _ = 5,       // Some future variant will be represented with 5u32.
}
// SAFETY: 5 is a reserved discriminant for `Fruit`.
let fruit: Fruit = unsafe { core::mem::transmute(5u32) };

// `fruit` is not any of the named variants.
assert!(!matches!(fruit, Fruit::Apple | Fruit::Orange | Fruit::Banana));

// These are both rejected: an unnamed variant can't construct reserved
// discriminants or patttern match on them.
// assert!(!matches!(fruit, Fruit::_));
// let fruit = Fruit::_;
```

By introducing this special variant, all users of `Fruit` must include a
wildcard branch when `match`ing, including within the declaring crate. Think of
the `_ = 5` as declaring that "discriminant `5` goes in the `_` branch when
`match`ing". There's no safe way to construct a `Fruit` from a `5`, but it can
be `transmute`d or received over FFI.

```rust
match fruit {
    Fruit::Apple | Fruit::Orange | Fruit::Banana => println!("Known fruit"),
    // Must be included, even in the crate that defines `Fruit`.
    x => println!("Unknown fruit: {}", x as u32),
}
```

An unnamed variant accepts a range as its discriminant expression, which ensures
each discriminant in the range is reserved and valid to use.

```rust
#[repr(u32)]     // Fruit is represented with specific discriminants of `u32`.
enum Fruit {
    Apple,       // Apple is represented with 0u32.
    Orange,      // Orange is represented with 1u32.
    Banana = 4,  // Banana is represented with 4u32.
    _ = 3..=10,  // 3 through 10 inclusive are valid discriminants for `Fruit`.
}
// SAFETY: 7 is a reserved discriminant for `Fruit`
let fruit: Fruit = unsafe { core::mem::transmute(7u32) };
```

By using `..` as an unnamed variant range, all bit patterns for the enum become
valid. It is now an _open enum_ and can be constructed from its underlying
representation via `as` cast:

```rust
#[derive(PartialEq, PartialOrd)]
#[repr(u32)]     // Fruit is represented by any `u32` - it is an *open enum*.
enum Fruit {
    Apple,       // Apple is represented with 0u32.
    Orange,      // Orange is represented with 1u32.
    Banana = 4,  // Banana is represented with 4u32.
    _ = ..,      // The rest of the discriminants in `u32` are reserved.
}
// Using an `as` cast from `u32`.
let fruit = 3 as Fruit;

// Does not match any of the known variants.
assert!(!matches!(fruit, Fruit::Apple | Fruit::Orange | Fruit::Banana));

// `fruit` preserves its value casting back to `u32`.
assert_eq!(fruit as u32, 3);

// `derive(PartialOrd, PartialEq)` works by discriminant as usual:
assert!(5 as Fruit > fruit);
assert!(3 as Fruit == fruit);
assert!(1 as Fruit == Fruit::Orange);

// error: incompatible cast: `Fruit` must be cast from a `u32`
// help: to convert from `isize`, perform a conversion to `u32` first:
//         let fruit2 = u32::try_from(5isize).unwrap() as Fruit;
let fruit2 = 5isize as Fruit;
```

This open enum is much like a `struct Fruit(u32)`, except it is treated as an
enum by IDEs and developers.

### Interaction with `#[non_exhaustive]`

An enum declared both `non_exhaustive` and with an unnamed variant is rejected.
On a field-less enum, it is not a breaking change to replace a
`#[non_exhaustive]` declared on the enum with a contained unnamed variant.
Unnamed variants and `#[non_exhaustive]` both declare that future variants of an
enum may be added as the type evolves.

`non_exhaustive` affects API semver compatibility:

- It is flexible in how new variants are represented.
- It does _not_ affect what discriminants are currently valid to represent.
- Crates must be recompiled to use new enum variants.
- It affects _only_ foreign crates.

By contrast, an unnamed variant affects API _and_ ABI semver compatibility:

- It reserves specific ranges of discriminants.
- These reserved discriminants are valid to represent without naming the future
  variants that use them.
- Crates can manipulate these unnamed enum variants without recompilation.
- It affects all crates, including the declaring one.

For enums that have relevant discriminant values, an unnamed variant may be the
better choice. This is often the case for enums declaring an explicit `repr`.

## Reference-level explanation

### Unnamed variants

An **unnamed variant** is an enum variant with `_` declared for its name. It
is assigned to a set of **reserved discriminants**. These discriminants are
valid for the enum, and may be assigned to a named variant in the future. It is
valid to `transmute` to an enum type from a reserved discriminant.

An unnamed variant does not declare an identifier scoped under the enum name,
unlike a named variant. `EnumName::_` remains an invalid expression and pattern.

An unnamed variant may be specified more than once on the same enum. It is valid
to reserve multiple ranges of discriminants. Those ranges may be discontiguous.

An explicit `repr(Int)` is required on an enum to declare an unnamed variant.
`Int` is one of the primitive integers or `C`. If it is `C`, then `Int` below is
`isize`. An unnamed variant must specify a discriminant expression with one of
these types:

- `Int`
  - Reserves a particular discriminant value.
  - The discriminant must not be assigned to another variant of the enum -
    whether named or unnamed.

    ```rust
    // error: discriminant value `1` assigned more than once
    #[repr(u32)]
    enum Color {
        Red,
        Green,
        Blue,
        _ = 1,
    }
    ```

- `start..end` (`core::ops::Range<Int>`) or\
  `start..=end` (`core::ops::RangeInclusive<Int>`)
  - Ensures every discriminant value in the range is reserved.
  - Named variants have higher precedence than unnamed variants when assigning
    discriminants to variants.

    ```rust
    #[repr(u32)]
    enum HttpStatusCode {
        Ok = 200,
        NotFound = 404,
        // Ensures the discriminants in 100..=599 are valid for Self.
        // Actually reserves 100..=199, 201..=403, and 405..=599.
        _ = 100..=599,
    }
    ```

  - The range must not overlap with discriminants assigned to unnamed variants.
    Multiple unnamed variants have equal claim to a discriminant value.

    ```rust
    #[repr(u8)]
    // error: discriminant value `10` assigned more than once
    enum Foo {
        X = 0,
        _ = 1..=10,
        _ = 10,
    }

    // error: discriminant values `10..=14` assigned more than once
    #[repr(u8)]
    enum Bar {
        X = 0,
        _ = 1..20,
        Y = 20,
        _ = 10..15,
    }
    ```

  - The range should be non-empty. A
    [`deny`-by-default lint](#empty-discriminant-ranges) is produced if this is
    violated.
  - There should be at least one discriminant available to reserve in the range.
    A [`warn`-by-default lint](#taken-discriminant-ranges) is produced if this
    is violated.
- `start..` (`core::ops::RangeFrom<Int>`)
  - Equivalent to `start..=Int::MAX`.
- `..end` (`core::ops::RangeTo<Int>`)
  - Equivalent to `Int::MIN..end`.
- `..=end` (`core::ops::RangeToInclusive<Int>`)
  - Equivalent to `Int::MIN..=end`.
- `..` (`core::ops::RangeFull`)
  - Equivalent to `Int::MIN..=Int::MAX`.
  - Reserves the rest of the discriminants for `Int`. This always makes an enum
    open without consideration for named variants' discriminants.
  - Because unnamed variants cannot have conflicting discriminants, this is the
    only unnamed variant allowed on the enum when used. It is called the enum's
    _open variant_.

    ```rust
    // error: discriminant value `1` assigned more than once
    // help: an `_` variant assigned to `..` forbids other `_` variants
    #[repr(u8)]
    enum Foo {
        X = 0,
        _ = 1,
        Y = 2,
        _ = ..,
    }
    ```

#### Type Inference

The discriminant expression for an unnamed variant has its type inferred as if
it were an argument to a generic function accepting the valid types for the
representation integer:

```rust
#[repr(u32)]
enum X {
    // {integer} infers as `u32`, `{integer}..{integer}` as `Range<u32>`, etc.
    _ = validate::<u32, _>(10),
    _ = validate::<u32, _>(10..20),
    _ = validate::<u32, _>(20..=30),
    // ...
}
const fn validate<Int, T: ReserveDiscriminants<Int>>(x: T) -> T { x }
trait ReserveDiscriminants<Int> {}
impl ReserveDiscriminants<u32> for u32 {}
// ... impl ReserveDiscriminants<Int> for Int {} ...
impl<Int> ReserveDiscriminants<Int> for Range<Int> {}
impl<Int> ReserveDiscriminants<Int> for RangeInclusive<Int> {}
impl<Int> ReserveDiscriminants<Int> for RangeFrom<Int> {}
impl<Int> ReserveDiscriminants<Int> for RangeTo<Int> {}
impl<Int> ReserveDiscriminants<Int> for RangeToInclusive<Int> {}
impl<Int> ReserveDiscriminants<Int> for RangeFull {}
```

#### `repr(C)` behavior

`repr(C)` enums have special semantics in Rust because the discriminant
expression type, `isize`, is not the same as the actual backing integer. These
enums are ordinarily backed by a `ffi::c_int`, but if any of the assigned
discriminants cannot fit, a larger backing integer is chosen that can represent
all of them.

> Since this behavior is fraught with ABI mismatches, this is going to change
> to [forbid enums larger than `c_int` or `c_uint`][enum-size-constrain].

Sometimes this is overridden by the system's ABI. On some rarer platforms,
`repr(C)` enums start as small as 1 byte, smaller than the C `int`. The behavior
is otherwise the same.

[enum-size-constrain]: https://github.com/rust-lang/rust/pull/147017

The same rules apply for discriminants assigned to unnamed variants:

```rust
#[repr(C)]
enum Small {
    X = 1,
    _ = 2..10,
}

// Named and unnamed variants can both grow a `repr(C)` enum.
enum Big1 {
    X = 1,
    _ = isize::MAX,
}

enum Big2 {
    X = 1,
    _ = 2,
    Y = isize::MAX,
}

// On x86_64-unknown-linux-gnu.
const _: () = assert!(
    size_of::<Small>() == 4 &&
    size_of::<Big1>() == 8 &&
    size_of::<Big2>() == 8
);
```

The unbounded end of a discriminant range never affects the backing integer of a
`repr(C)` enum. When a range with an unbounded end (`start..`, `..end`,
`..=end`, `..`) is used as an unnamed variant's discriminant expression in a
`repr(C)` enum, the set of discriminants that is reserved by that unbounded end
is dependent on the other variants' discriminants.

```rust
#[repr(C)]
enum SmallNonnegative {
    X = 0,
    // Reserves `1..=c_int::MAX`.
    _ = 1..,
}

#[repr(C)]
enum BigOpen1 {
    X = isize::MAX,
    // Reserves `isize::MIN..isize::MAX`.
    _ = ..,
}

#[repr(C)]
enum BigOpen2 {
    // Reserves `isize::MIN..0`.
    _ = ..0,
    _ = 0..=isize::MAX,
}

// On x86_64-unknown-linux-gnu.
const _: () = assert!(
    size_of::<SmallNonnegative>() == 4 &&
    size_of::<BigOpen1>() == 8 &&
    size_of::<BigOpen2>() == 8
);
```

This behavior means that it is sound to expose a C enum defined like this:

```c
enum Foo {
    Name1 = Value1,
    Name2 = Value2,
    // etc.
};
```

as this Rust enum, regardless of the discriminant values assigned:

```rust
// `allow` effective when there are 256 variants within the `u8`/`i8` range on
// a short-enum platform. Only macros/codegen like bindgen bother with this.
#[allow(taken_discriminant_ranges)]
#[repr(C)]
enum Foo {
    Name1 = Value1,
    Name2 = Value2,
    // etc.

    // The rest of the discriminants for an enum with the named variants
    // are reserved and valid. Unchecked casts can't invoke UB.
    _ = ..,
}
```

#### Grammar changes

[EnumVariant] is extended to allow an underscore instead of a variant's name:

```text
EnumVariant ->
  OuterAttribute* Visibility?
  (IDENTIFIER | `_`) ( EnumVariantTuple | EnumVariantStruct )?
  EnumVariantDiscriminant?
```

[EnumVariant]: https://doc.rust-lang.org/reference/items/enumerations.html#grammar-EnumVariant

#### No field data

This RFC only defines adding unnamed variants to field-less enums, leaving this
as future work.

#### `non_exhaustive`

The `non_exhaustive` attribute on enums and unnamed variants are mutually
exclusive:

```rust
#[non_exhaustive]
#[repr(u8)]
enum Color {
    Red = 0,
    Green = 1,
    // error: An `_` variant cannot be specified on a `non_exhaustive` enum.
    // help: remove the `#[non_exhaustive]`
    _ = 2,
}
```

An unnamed variant is more impactful than `non_exhaustive`, since it affects the
declaring crate - the enum is "universally non-exhaustive".

#### Compatibility

Given enum versions A and B with some change between them:

- A change is forwards-compatible if a library designed for enum version A can
  use A or B.
- A change is backwards-compatible if a library designed for enum version B can
  use A or B.
- A change is fully-compatible if it is both forwards and backwards compatible.
- A change is API compatible if the change does not affect static compilation
  using a single enum source, either A or B.
- A change is ABI compatible if the change does not affect dynamically linked
  libraries compiled using enum versions A and B (with the same Rust compiler).

It is an API and ABI fully-compatibile change to:

- Add a named variant to a field-less enum using a discriminant that was
  previously reserved.
  - When doing the this, removing the last unnamed variant may cause warnings
    for unused code in client libraries, as a wildcard branch is no longer
    required. This can be avoided by then adding `#[non_exhaustive]` to the
    enum.

It is an API fully-compatible and ABI backwards-compatible change to:

- Replace `#[non_exhaustive]` on an enum with an unnamed variant.
  - This may require changes to the defining crate to add wildcard branches.
- Add another reserved discriminant, if an unnamed variant already exists on the
  enum.

It is an API and ABI backwards-compatible change to:

- Add an unnamed variant to an enum without `#[non_exhaustive]` or another
  unnamed variant. The same caveat regarding unused wildcard branches applies.

#### Applicable lints

##### Empty discriminant ranges

`empty_discriminant_ranges` is a new `deny`-by-default lint. It should be
produced if the discriminant range assigned to an unnamed variant is empty.

```rust
#[repr(u8)]
enum Foo {
    X = 0,
    // error: empty range assigned to `_` variant
    // help: variant has discriminant range `0..1`
    _ = Self::X..Self::Y,
    Y = 1,
}

#[repr(isize)]
enum Bar {
    X = 2,
    // error: empty range assigned to `_` variant
    // help: variant has discriminant range `-2..0`
    _ = Self::X..Self::Y,
    Y = 0,
}
```

- It is usually a mistake to specify an empty range.
- An empty or negative range could accidentally cause UB if certain
  discriminants are expected to be reserved but are not due to reversing the
  `start` and `end` of the range.
- If `allow`ed, the unnamed variant declaration has no effect.
- There are rare use cases involving macro or non-literal discriminants
  in which in may be intentional to declare an empty variant in order to
  avoid complex discriminant analysis.

##### Taken discriminant ranges

`taken_discriminant_ranges` is a new `warn`-by-default lint. It should be
produced if every discriminant in the range assigned to an unnamed variant is
already assigned to a named variant. Thus, the unnamed variant does not
introduce any reserved discriminants and has no effect on the enum.

```rust
#[repr(u8)]
enum Foo {
    X,
    Y,
    // warning: all discriminants in range `0..=1` already assigned
    // help: remove the `_` variant; it has no effect
    // help: `0` is assigned here: `X`
    // help: `1` is assigned here: `Y`
    _ = 0..2,
}
```

This warning should thus be produced when specifying an unnamed variant on an
enum that is already open. Any macro or codegen that intends to make an enum
open can ignore this lint when adding `_ = ..`:

```rust
// Say bindgen generated this from a C enum.
// It shouldn't have to count the number of variants and compare that against
// the `repr` to know if the enum's already open and must avoid placing the
// `_ = ..`. It can just allow the warning.
#[allow(taken_discriminant_ranges)]
#[repr(u8)]
enum NamedU8 {
    James = 0,
    Fernando = 1,
    Sally = 2,
    // ... Named variant for every other u8 ...
    Jolene = 255,

    _ = ..,
}
```

##### Truncatable ranges

`overlong_discriminant_ranges` is a new `warn`-by-default lint. It should be
produced if an unnamed variant's discriminant range can be shortened to avoid
overlapping with named variants.

Let `start..=end` be the range of discriminants that an unnamed variant
definition is assigned to, regardless of the actual range type used. An
`overlong_discriminant_ranges` lint should be produced if all of the below are
true:

- The bound is specified as a range expression in the variant's discriminant
  expression, and not as an identifier or block.
- Every discriminant in some prefix or suffix of the range is already assigned.
  That is, there exists some `n ≥ 0` such that the sub-range
  `start..=(start + n)` or `(end - n)..=end` has every discriminant in that
  range assigned to a named variant. Let either sub-range for which this is
  true be called an "overlong side".
- An overlong side is specified with a literal integer, and not implicitly
  defined by an unbounded range.
- The prefix is an overlong side _or_ the following variant, if any, has an
  explicit discriminant.
- The `taken_discriminant_ranges` lint is not produced for this unnamed variant.

```rust
#[repr(u32)]
enum LeftSide {
    X,
    Y,
    Z,
    // warning: discriminant range for variant can be shortened
    // help: shorten the range: `3..`
    // note: `#[warn(overlong_discriminant_ranges)]` on by default
    _ = 0..,
}
#[repr(u32)]
enum RightSide {
    X,
    Y = 9,
    Z,
    // warning: discriminant range for variant can be shortened
    // help: shorten the range: `(Self::X as u32)..9`
    // note: `#[warn(overlong_discriminant_ranges)]` on by default
    _ = (Self::X as u32)..10,
}
#[repr(u32)]
enum BothSides {
    X,
    Y,
    Z = 10,
    // warning: discriminant range for variant can be shortened
    // help: shorten the range: `2..=9`
    // note: `#[warn(overlong_discriminant_ranges)]` on by default
    _ = 0..=10,
}
#[repr(u32)]
enum NonLiteralOverlongSide {
    X,
    // A warning is not produced, as the overlong side is a non-literal.
    // This was likely intended.
    _ = (Self::X as u32)..10,
}
#[repr(u32)]
enum UnboundedOverlongSide {
    X = 0,
    // A warning is not produced, as the overlong side is unbounded.
    _ = ..10,
}
#[repr(u32)]
enum ImplicitNextDiscriminant {
    // A warning is not produced, as the following named variant depends on the
    // overlong side's discriminant.
    _ = 5..=10,
    X,  // 11
    Y = 10,
}
```

##### Gap of length one caused by an exclusive range

The existing [`non_contiguous_range_endpoints`] lint should be produced if:

[`non_contiguous_range_endpoints`]: https://doc.rust-lang.org/stable/nightly-rustc/rustc_lint_defs/builtin/static.NON_CONTIGUOUS_RANGE_ENDPOINTS.html

- There exists some unnamed variant assigned to a `start..end` or `..end`
  discriminant expression, and
- `end` is not a valid discriminant for the enum, and
- `end + 1` is a valid discriminant for the enum.

```rust
#[repr(u32)]
enum Foo {
    // warning: multiple ranges are one apart
    // help: this range doesn't match `100` because `..` is an exclusive range
    // help: use an inclusive range instead: `80..=100`
    _ = 80..100,
    X = 101,
    // ^ this could appear to continue range `0..100`, but `100` isn't included
    //   by either of them

}

#[repr(u32)]
enum Bar {
    // warning: multiple ranges are one apart
    // help: this range doesn't match `99` because `..` is an exclusive range
    // help: use an inclusive range instead: `..=99`
    _ = ..99,
    _ = 100..200,
    // ^ this could appear to continue range `..99`, but `99` isn't included
    //   by either of them
}
```

##### Forgot to mention a named variant

The unstable [`non_exhaustive_omitted_patterns`] `allow`-by-default lint should
be produced if a `match` on an enum with reserved discriminants mentions some,
but not all, of the named variants.

[`non_exhaustive_omitted_patterns`]: https://doc.rust-lang.org/stable/nightly-rustc/rustc_lint_defs/builtin/static.NON_EXHAUSTIVE_OMITTED_PATTERNS.html

This uses the same name as the similar lint for `non_exhaustive` because it is
burdensome to require developers to remember two different lints for such
similar use cases. This requires updating the documentation of the lint to
reference unnamed variants as well as `non_exhaustive`.

It may also be prudent to rename the lint before stabilization to include
unnamed variants.

```rust
#[repr(u32)]
enum Bar {
    A,
    B,
    _ = ..,
}
let b = Bar::A;

// warning: some variants are not matched explicitly
//          pattern `Bar::B` not covered
// help: ensure that all variants are matched explicitly by adding the
//       suggested match arms
// note: the matched value is of type `Bar` and the
//       `non_exhaustive_omitted_patterns` attribute was found
#[warn(non_exhaustive_omitted_patterns)]
let name = match b {
    Bar::A => "A",
    _ => "unknown",
};
```

#### Next variant's implicit discriminant

When a named variant without an implicit discriminant follows an unnamed
variant, the assigned implicit discriminant is the next integer after the
declared discriminant range for that unnamed variant. If the unnamed variant is
assigned to an integer, it is the next integer.

```rust
#[repr(u32)]
enum Foo {
    X,
    _ = 5,
    Y,
}
assert_eq!(Foo::Y as u32, 6);

#[repr(u32)]
enum Bar {
    _ = ..10,
    X,
    Y = 9,
}
assert_eq!(Bar::X as u32, 10);

#[repr(u32)]
enum Baz {
    _ = 2..=10,
    X,
}
assert_eq!(Baz::X as u32, 11);

#[repr(u8)]
enum Overflow {
    _ = 10..,
    // error: enum discriminant overflowed
    // overflowed on value after 255
    X,
}
```

#### Non-literal discriminant expression

A non-literal range or integer is allowed for an unnamed variant.

```rust
const VALID_FOO: Range<u32> = 10..100;

#[repr(u32)]
enum Foo {
    X = 10,
    Y = 20,
    Z = 30,
    _ = VALID_FOO,
}
// SAFETY: `15` is a valid discriminant in range `VALID_FOO`.
let _: Foo = unsafe { mem::transmute(15u32) };
```

#### Only variant

An unnamed variant may be the only variant for an enum. In this case, an `as`
cast or `transmute` is the only way to construct an enum value.

```rust
#[repr(u32)]
#[derive(PartialEq, PartialOrd)]
enum NothingYet { _ = .. }
(10 as NothingYet > 5 as NothingYet)
```

### Open enum conversion

An _open enum_ is defined as an `enum` for which every value of its backing
integer is a valid discriminant.

- An open enum always has an explicit `repr` backing integer, or is `repr(C)`.
- An enum is open if every discriminant value for that integer is associated
  with a named variant or is reserved with an unnamed variant.
  - For a field-less enum, this means every initialized bit pattern is valid.
- A [unit-only] open enum may be `as` cast from its backing integer:
  `2u8 as Color`. See below for `repr(C)` behavior.
  - Casting from other integer types is rejected.
- If an expression with the `{integer}` inference variable type is used as the
  source for an `as` cast to an open enum, it is uniquely constrained to the
  explicit backing integer type. This excludes `repr(C)`; see below.

    ```rust
    #[repr(u8)]
    enum Foo { _ = .. }
    let x = 10;

    // `x` must be a `u8` to be cast to `Foo`
    let _ = x as Foo;

    // error: mismatched types, expected `u32`, found `u8`
    // let _: u32 = x;
    ```

#### `repr(C)` open enum behavior

The actual backing integer type for a `repr(C)` enum changes based on the
variants' numeric discriminant values as described above.

A `repr(C)` unit-only open enum may be `as` cast from:

- `const` expressions of type `isize`. This is so a `repr(C)` enum may always
  be `as` cast from the same discriminant expression assigned to a variant.
- Any primitive explicit-width integer that is capable of representing all
  variants' discriminants and does not exceed the size of the enum for the
  platform. Thus any signedness cast performed to the backing integer has no
  visible effect.
  - This means that authors who don't know or care about short-enum platforms
    can cast from `c_int` and `c_uint` to most `repr(C)` open enums, while
    preventing unexpected truncations when necessary.

```rust
const TEN: isize = 10;

// Must be able to represent `u8::MAX`: `u8` or `c_int` or `c_uint`.
#[repr(C)]
enum SmallUnsigned {
    X = 0,
    Y = TEN,
    Z = 255,
    _ = ..,
}

// May be backed by `c_int` or `c_uint` or `i8` or `u8`.
#[repr(C)]
enum Small {
    X = 0,
    Y = 10,
    _ = ..,
}

// Must be able to represent negative numbers: `i8` or `c_int`.
#[repr(C)]
enum SmallSigned {
    X = 0,
    Y = 10,
    Z = -10,
    _ = ..,
}

// Must be able to hold `isize::MIN..=isize::MAX` which may exceed `c_int`.
#[repr(C)]
enum Big {
    X = 0,
    Y = TEN,
    Z = 255,
    _ = isize::MIN..=isize::MAX,
}

assert!(matches!(TEN as Big, Big::Y));
assert!(matches!(TEN as SmallUnsigned, SmallUnsigned::Y));
assert!(matches!(TEN as SmallSigned, SmallSigned::Y));
assert!(matches!(TEN as Small, Small::Y));

let zero: c_int = 0;
assert!(matches!(zero as Big, Big::X));
// On thumbv7m-none-eabi:
// error: truncating cast to `repr(C)` open enum
// note: `SmallUnsigned` is backed by `u8`, which fallibly converts
//       from `i32`
// help: try converting to `u8` first:
//       `u8::try_from(zero).unwrap() as SmallUnsigned`
assert!(matches!(zero as SmallUnsigned, SmallUnsigned::X));

let ten: isize = 10;
assert!(matches!(ten as Big, Big::Y));
// On x86_64-unknown-linux-gnu:
// error: truncating cast to `repr(C)` open enum
// note: `SmallUnsigned` is backed by `i32`, which fallibly converts
//       from `isize`
// note: a `repr(C)` open enum may be cast from constant `isize`
// help: try converting to `i32` first:
//       i32::try_from(ten).unwrap() as SmallUnsigned
assert!(matches!(ten as SmallUnsigned, SmallUnsigned::Y));

let byte: u8 = 255;
assert!(matches!(byte as Big, Big::Z));
assert!(matches!(byte as SmallUnsigned, SmallUnsigned::Z));
_ = byte as Small;
// On thumbv7m-none-eabi:
// error: truncating cast to `repr(C)` open enum
// note: `SmallSigned` is backed by `i8`, which fallibly converts
//       from `u8`
// help: try converting to `i8` first:
//       `i8::try_from(byte).unwrap() as SmallSigned`
_ = byte as SmallSigned;

let signed_byte: i8 = 10;
assert!(matches!(signed_byte as Big, Big::Y));
// On thumbv7m-none-eabi:
// error: truncating cast to `repr(C)` open enum
// note: `SmallUnsigned` is backed by `u8`, which fallibly converts
//       from `i8`
// help: try converting to `u8` first:
//       `u8::try_from(signed_byte).unwrap() as SmallUnsigned`
assert!(matches!(signed_byte as SmallUnsigned, SmallUnsigned::Y));
_ = signed_byte as Small;
_ = signed_byte as SmallSigned;
```

### Interaction with the standard library

- `derive(Debug)` formats as `EnumName(X)` when `X` is a reserved discriminant.
  A `Debug` format changing is not considering an API-breaking change.
- `Default` forbids `#[default]` from being specified on an unnamed variant,
   but this may change in the future.
- The derives `Clone`, `Copy`, `Eq`, `Hash`, `Ord`, `PartialEq`, and
  `PartialOrd` are unaffected by unnamed variants on a field-less enum.
  They all operate on discriminants, and this includes reserved discriminants.
- `mem::Discriminant` continues to operate as before, always treating
  field-less enum values with the same discriminant integers as equal and
  those with different discriminant integers as non-equal.

## Drawbacks

- The mutual-exclusion with `non_exhaustive` despite having similar motivations
  could be confusing to explain to new users.
- Every new feauture in Rust is another thing to maintain and for users to
  learn.
- Rust has not put significant efforts towards ABI compatibility in language
  constructs in the past.

### Flag enums

It is possible to define `bitflags` style enums using `enum` syntax with
unnamed variants. However, if `BitOr` is defined on such an enum, then, rather
confusingly, `!matches!(Enum::A | Enum::B, Enum::A | Enum::B)`. This problem
exists for `bitflags` or integer newtypes that `derive(PartialEq)` today, which
is why the library defines a `bitflags_match!` macro that avoids it.

As future work, a lint could trigger when `|` is used in a pattern with a
non-integer type that defines `BitOr` and has structural equality.

## Rationale and alternatives

Unnamed variants enable a large range of discriminants to be reserved for an
enum, whether it's all or some of them. `NonZero`, and an `enum` spelling out
each discriminant are the only other ways to achieve this in stable Rust today.

The open enum conversion from backing integer is an ergonomic benefit that is
made possible by unnamed variants.

### Do nothing

Why not just use an integer newtype or macro?

The best way to write a field-less open enum in Rust today is the "newtype enum"
pattern that uses associated constants for variants. So, to make this enum open:

```rust
enum Color {
    Red,
    Blue,
    Black,
}
```

the author can write this:

```rust
#[repr(transparent)]  // Optional, but often useful
#[derive(PartialEq, Eq)]  // In order to work in a `match`
struct Color(pub i8);  // Alternatively, make the inner private and `impl From`

#[allow(non_upper_case_globals)]  // Enum variants are CamelCase
impl Color {
    pub const Red: Color = Color(0);
    pub const Blue: Color = Color(1);
    pub const Black: Color = Color(2);
}
```

With this syntax, users of an open enum can use these variant names inside a
`match` with _mostly_ the same syntax as they would with a regular closed enum,
except there must _always_ be a wildcard branch for handling unknown values.
This syntax also provides grouping of related values and associated methods, an
advantage over module-level `const` items.

However, this pattern has some distinct disadvantages when used to emulate
an open enum, as described in the
[Motivation](#newtype-integers-are-bad-for-enumeration) section above.

[Pattern types][pattern types] can constrain the valid values for an integer
newtype, but do not help with the enum ergonomics issue.

### Attribute to improve diagnostic behavior for associated `const`

Newtype integers could improve the ergonomics for a "fill match arms" analyzer
capabilities and other diagnostics with an attribute placed on pseudo-variants:

```rust
#[repr(transparent)]
#[derive(PartialEq, Eq)]
struct Color(pub i8);

#[allow(non_upper_case_globals)]
impl Color {
    // Tells rust-analyzer "this is like an enum variant"
    #[diagnostic::enum_variant]
    pub const Red: Color = Color(0);

    #[diagnostic::enum_variant]
    pub const Blue: Color = Color(1);

    #[diagnostic::enum_variant]
    pub const Black: Color = Color(2);
}
```

However:

- Open enums require even more typing for the desired semantics.
- `derive`s cannot be easily written with enum variant names. In order to avoid
  duplicating the names, a `derive` macro must directly inter-operate with
  another macro that generates these pseudo-variants like `open-enum`.
- This is less discoverable than a user trying to `as` cast to an enum and
  having the compiler inform them of `_ = ..` as an option.
- It's not clear how this would relate to the functionality of the
  [`non_exhaustive_omitted_patterns`] lint.

### As an `enum` attribute

An enum could be made open by specifying it as part of its `repr`:

```rust
#[repr(open, u8)]  // requires an explicit `repr(Int)`
enum Color {
    Red,
    Blue,
    Black
}
use Color::*;
// or an unsafe `transmute`
assert!(!matches!(3u8 as Color, Red | Blue | Black));
```

This has the same interaction with `#[non_exhaustive]`. The drawbacks:

- It's not as clear what the attribute does, in contrast to the `_ = ..` syntax
  mirroring known concepts: we're introducing new valid values, `_` means
  "unnamed/wildcard", and `..` means "the rest" as the discriminants.
- It is not clear why a `repr` would affect `match`/`as` behavior, even though
  this does affect how it is valid to represent the type.
  - There are many alternative syntaxes for this, such as
    `#[non_exhaustive(repr)]` or `[open]` / `#[open(Range)]`. All should require
    a `repr(Int)` be specified.
- Allowing for a reservation of particular ranges instead of a full opening
  could be done with a pattern-type-like syntax, but this is less discoverable:

  ```rust
  #[repr(u8 in 1..=100)]
  pub enum NonZeroU8 {
      One = 1,
      Two = 2,
  }
  ```

- Unnamed variants meld well with [unnamed fields] in `struct`/`union` for ABI
  stability, if that is ever stabilized.
- An `#[repr(u8)] enum E { A, B }` has two possible values, but an open enum
  would instead have 256. Attributes are not typically used to adjust a type's
  validity to this degree. `#[non_exhaustive]` is barely an exception; it merely
  prevents exhaustive matches. Therefore, something stronger than an attribute
  should be required to open an enum.

### Unbounded ranges select discriminants based on surrounding variants

```rust
#[repr(u32)]
enum Foo {
    X,
    // Reserves `1..=4`.
    _ = ..,
    Y = 5,
    // Reserves `6..=10`.
    _ = ..=10,
}

enum Bar {
    _ = ..
    X,
    _ = ..,
    Y = 5,
}
```

- This prevents the highly desirable one-line declaration that every
  discriminant is valid.
- Ordinarily a variant with an explicit discriminant expression is not sensitive
  to the discriminants of surrounding variants.

Consider this enum being processed by a derive macro:

```rust
// How does a derive-macro make this enum have no niches?
#[repr(u8)]
enum Foo {
    X = CONST1,  // non-literal expressions defined elsewhere
    Y = CONST2,
}
```

How does that macro make the `Foo` enum open? The macro developer might try to
surround the variants with `_ = ..`:

```rust
#[repr(u8)]
enum Foo {
    _ = ..,
    X = CONST1,  // non-literal expressions defined elsewhere
    _ = ..,
    Y = CONST2,
    _ = ..,
}
```

But what if `CONST1 > CONST2`? If this compiles then the range of discriminants
`(CONST2 + 1)..CONST1` are invalid and it's not an open enum! If it errors out,
then there's no clear way one is supposed to write the opening-macro.
Complicating the macro further can make it work, so long as empty discriminant
ranges are allowed:

```rust
#[repr(u8)]
enum Foo {
    _ = ..CONST1,
    X = CONST1,

    // You need to provide your *own* `max` and `min` since it's unstable
    // in `const`.
    _ = min(CONST1, CONST2)..=max(CONST1, CONST2),

    Y = CONST2,
    _ = ..,
}
```

### Declare niches instead of reserving values

If an enum selects its discriminants such that a desirable niche exists, like
`0`, perhaps it is better to declare ranges of niches rather than reserving
discriminants?

It can be very confusing to mix positive and negative assertions, and this would
be doing that for enum discriminants in likely a different syntax than variant
declaration.

Unnamed variants use the same syntax to assign discriminants, except they do not
have to have a name and thus can be assigned to discontiguous ranges.

### Discriminant ranges for named variants instead of unnamed variants

What if instead this were valid?

```rust
enum IpProto {
    Tcp = 6,
    Udp = 17,
    Other = ..,
}
```

This is not mutually exclusive with unnamed variants, but this RFC chooses to
leave reserved ranges of discriminants as anonymous to keep the feature simple.

- It is ambiguous what value should be chosen when `IpProto::Other` is used in
  an expression.
- Even with an arbitrary rule to choose a discriminant, a consistently
  performant `derive(PartialEq)` that compares discriminants instead of ranges
  of discriminants will result in
  `matches!(o, IpProto::Other) && o != IpProto::Other`.
  - A reasonable but less useful alternative is to reject expression usage as
    well as `derive(PartialEq)`.
- If discontiguous ranges are allowed as above, the performance of
  `matches!(o, EnumName::Variant)` gets worse as the number of variants grows.
- Adding an `Icmp = 1` variant affects `matches!(1 as IpProto, IpProto::Other)`:
  it is an API-breaking change.
- A `derive` can be used to determine whether an enum's discriminant is assigned
  to a named variant.
- Anonymous discriminant values are useful on their own for enum evolution.

This can be left as future work for the language.

### `..` at the end

```rust
#[repr(u8)]
enum IpProto {
    Tcp = 6,
    Udp = 17,

    // "the rest of the variants exist"
    ..
}
```

- This is less flexible than `_ = ..`, and is awkward to restrict to smaller or
  discontiguous ranges.
- This resembles the [rest pattern] more than the [full range expression] that
  discriminants are assigned to and the [wildcard pattern] that it requires.

[full range expression]: https://doc.rust-lang.org/reference/expressions/range-expr.html#grammar-RangeFullExpr
[rest pattern]: https://doc.rust-lang.org/reference/patterns.html?#rest-patterns
[wildcard pattern]: https://doc.rust-lang.org/reference/patterns.html?#wildcard-pattern

### An "other" variant carries unknown discriminants like a tuple variant

An alternative way to specify a field-less open enum could be to write this:

```rust
#[repr(u32)]
enum IpProto {
    Tcp = 6,
    Udp = 17,

    // bikeshed syntax
    Other(0..6 | 7..17 | 18..=u32::MAX),
}
```

This would mean that the `Other` variant is a named way to refer to unlisted
values and works in pattern matching naturally, while being a zero-cost
representation:

```rust
if let IpProto::Other(x) = proto {
    // `proto` was *not* `Tcp` or `Udp`; its integer value is in `x`.
}
```

However, this has some problems. For one, it's peculiar for a tuple variant
syntax to not carry a payload, but a discriminant. It is also possible to
build the variant with a discriminant value, which means that it would need
to be constrained by a [pattern type][pattern types] - one that may end up
far more complicated if it overlaps with named variants. It is also an API
breaking change to move the discriminant `2` to a new named variant, since
it breaks anyone passing `2` into an `IpProto::Other` expression.

```rust
if let IpProto::Other(x) = IpProto::Other(6) {
    // This branch is not taken, since it's actually an `IpProto::Tcp`!
}
```

Instead, to get this behavior with this RFC's proposed syntax, the
author could use a third-party derive to check against the named variants,
and an `unsafe` transmute or `as` cast to construct the enum value from
integer. This makes it clear that declaring a new named variant with an
unnamed variant's discriminant will affect the method's return value.

```rust
#[repr(transparent, u32)]
#[derive(IsNamedVariant)]
enum IpProto {
    Tcp = 6,
    Udp = 17,
    _ = ..,
}

assert!(!(3u32 as IpProto).is_named_variant());
assert!((6u32 as IpProto).is_named_variant());
```

### Forbid unnamed variants' discriminants from overlapping named ones

```rust
#[repr(u32)]
// error: discriminant `200` assigned more than once
enum HttpStatusCode {
    Ok = 200,
    _ = 100..=599,
}
```

This makes it entirely unambiguous which discriminant is assigned to which
variant, without precedence rules. However, `_ = ..` to "make it open" is still
desirable.

- Forbidding named variant overlaps with `_ = ..` makes it nearly useless, since
  it then must be the only variant for the enum.
- Giving `..` special behavior to reserve "the rest" of the variants is then
  inconsistent with other ranges' behavior.
  - There is precedent for `..` acting differently than other ranges, such as
    when `match`ing a number or `char`. This `..`, however, is an expression and
    not a pattern.
  - It cannot be reasonably be equivalent to `Int::MIN..=Int::MAX` without that
    range allowing named variant overlap.

### Require an unnamed variant reserve at least one discriminant

It is a desirable property for an unnamed variant to always introduce a reserved
discriminant.

This would mean that an unnamed variant's presence in an enum always requires a
wildcard branch when `match`ing. Otherwise, a peculiar situation is possible in
which an enum definition declares an unnamed variant, but does not have any
reserved discriminants and thus no wildcard branch is needed.

However, upholding this requirement prevents `_ = ..` from always working to
mean "ensure this enum is open". In order for macros or codegen like `bindgen`
to ensure an enum is open, they would need to handle the particular edge case of
an enum with 256 variants and an 8-bit discriminant and leave out the variant.
Instead, the lints can be `allow`ed for carefully-considered macros/codegen.

### Require `non_exhaustive`, don't forbid it

Perhaps an unnamed variant could _require_ `#[non_exhaustive]`, rather than
forbid it? This RFC opts against that, with the following considerations:

Pros:

- `non_exhaustive` already implies adding another wildcard branch. This could
  make it easier to explain to new users by fitting the idea of "needs wildcard
  branch" into one mental bucket.
- This would make the unstable allow-by-default
  `non_exhaustive_omitted_patterns` lint more obviously correct to apply to
  enums with unnamed variants.

Cons:

- It expands the scope of `non_exhaustive`: the wildcard branch required by
  unnamed variants apply to the defining crate as well as foreign crates. This
  could make it harder to explain to newer users.
- The variant name being an underscore _already_ implies that a wildcard branch
  is needed.
- It always requires two lines to achieve ABI non-exhaustiveness.
- Consider this enum:

  ```rust
  #[repr(u8)]
  #[non_exhaustive]
  enum OpenEnum {
      X000 = 0,
      X001 = 1,
      // XNNN = N,
      X254 = 254,
      _ = 255,
  }
  ```

  When adding `X255`, the `non_exhaustive` _should_ also be removed, but as of
  today, an open enum gives no warning if it is `non_exhaustive`. This is even
  though it would necessarily be an API and ABI-breaking change to add a new
  variant by changing the `repr`. This is non-obvious and can be avoided by
  forbidding `non_exhaustive` when a valid unnamed variant exists.

## Prior art

_Open_ and _closed_ enums are [pre-existing industry terms][acord-xml].

### Enum openness in other languages

- C++'s [scoped enumerations][cpp-scoped-enums] and C enums are both open
  enums.
- C## uses [open enums][cs-open-enums], with a [proposal][cs-closed-enums] to
  add closed enums for guaranteed exhaustiveness.
- Java uses closed enums.
- [Protobuf][protobuf-enum] uses closed enums with the `proto2` syntax, treating
  unlisted enum values as unknown fields, and changed the semantics to open
  enums with the `proto3` syntax. This was in part because of lessons learned
  from protocol evolution and service deployment as described above.
- Swift uses both closed and open enums, based on if it is `@frozen`. An
  `@unknown default` branch is required for open enums, the `@unknown` being
  another way to achieve the design goals of the
  `non_exhaustive_omitted_range_patterns` lint.

[acord-xml]: https://docs.oracle.com/cd/B40099_02/books/ConnACORDFINS/ConnACORDFINSApp_DataType10.html
[cpp-scoped-enums]: https://en.cppreference.com/w/cpp/language/enum#Scoped_enumerations
[cs-open-enums]: https://learn.microsoft.com/en-us/dotnet/csharp/language-reference/builtin-types/enum#conversions
[cs-closed-enums]: https://github.com/dotnet/csharplang/issues/3179
[protobuf-enum]: https://developers.google.com/protocol-buffers/docs/reference/cpp-generated#enum

### Other crates that use open enums

Users today are simulating open enums with other language constructs, but it's
a suboptimal experience:

- [open-enum], written by the author of this RFC. It's a procedural
  macro which converts any field-less `enum` definition to an equivalent
  newtype integer with associated constants.
- Bindgen is [aware of the problem][bindgen-ub] with FFI and closed enums, and
  avoids creating Rust enums from C/C++ enums because of this. It provides an
  option for newtype enums directly.
- ICU4X uses newtype enums for [certain properties][icu4x-props] which must be
  forwards compatible with future versions of the enum.
- OpenTitan's [`with_unknown!`] macro also uses this pattern to create
  "C-like enums".
- `winapi-rs` defines an [`ENUM`][winapi-enum] macro which generates plain
  integers for simple `enum` definitions.

The [`newtype-enum` crate][newtype-enum-crate] is an entirely different pattern
than what is described here.

[bindgen-ub]: https://github.com/rust-lang/rust/issues/36927
[icu4x-props]: https://github.com/unicode-org/icu4x/blob/ff1d4b370b834281e3524118fb41883341a7e2bd/components/properties/src/props.rs#L56-L106
[newtype-enum-crate]: https://crates.io/crates/newtype-enum
[open-enum]: https://crates.io/crates/open-enum
[`with_unknown!`]: https://github.com/lowRISC/opentitan/blob/06584dc620c633e88631f97f1fc1e22c1980c21c/sw/host/ot_hal/src/util/unknown.rs#L7-L48
[winapi-enum]: https://github.com/retep998/winapi-rs/blob/77426a9776f4328d2842175038327c586d5111fd/src/macros.rs#L358-L380

### `bitflags`

The bitflags crate also uses [an unnamed value][bitflags-unnamed] with `_` to
specify valid bits without assigning a name to them.

[bitflags-unnamed]: https://docs.rs/bitflags/latest/bitflags/macro.bitflags.html#named-and-unnamed-flags

### `abi_stable`

[`abi_stable::NonExhaustive`] uses an associated type to hold a typed raw
discriminant for an enum. It is not ergonomic to `match` on discriminant values
directly, but another macro could improve this.

[`abi_stable::NonExhaustive`]: https://docs.rs/abi_stable/0.11.3/abi_stable/nonexhaustive_enum/struct.NonExhaustive.html

### Unnamed fields

The [unnamed fields] RFC reserves space for future extension in a `struct` or
`union` for FFI purposes, allowing ABI to be planned ahead of time. Unnamed
variants have similar motivations, but no great workaround. The future work
proposed below to allow `_(payload) = discriminants` further unifies these
concepts by reserving space for `payload` to be held in the enum.

[unnamed fields]: https://github.com/rust-lang/rfcs/blob/master/text/2102-unnamed-fields.md

### `repr(open)` RFC

There's an [unmerged RFC][enum-repr-open] that defines a `repr(open)` syntax as
described in the Alternatives section above.

[enum-repr-open]: https://github.com/madsmtm/rfcs/blob/enum-repr-no-niches/text/3803-enum-repr-open.md

## Unresolved questions

None.

## Future possibilities

### Discriminant ranges for named variants

A future extension could allow for named variants to specify ranges as
discriminants. This bikeshed syntax avoids many of the drawbacks in the
related Alternatives section above.

```rust
#[repr(u8)]
enum Color {
    Red = 0,
    Green = 1,
    // Must specify a non-overlapping range,
    // including if `..` is the discriminant.
    Unknown = 2..=50,
}

// This is fine.
assert_eq!(Color::Red as u8, 0);

// error: ambiguous discriminant for `Color::Unknown`
// help: specify a discriminant with `2 as Color::Unknown`
// let c = Color::Unknown;

// Use an `as` cast to construct `Color::Unknown` safely.
let c = 3 as Color::Unknown;
assert_eq!(c as u8, 3);

// error: invalid discriminant for `Color::Unknown`
// help: `Color::Unknown` has the discriminant range `2..=u8::MAX`
// let c = 0 as Color::Unknown;

let d = 10u8;
// error: non-constant expression used for enum ranged variant cast
// let c = d as Color::Unknown;

// This is fine.
let c = const { 1 + 1 } as Color::Unknown;

// Pattern types could extend this further:
let e = match d {
    x @ 2..=50 => x as Color::Unknown,
    _ => unreachable!(),
};
assert_eq!(e as u8, 10);
```

### Unnamed variants on enums with field data

Unnamed variants on enums with field data would allow library authors to plan
for future ABI compatibility by reserving discriminants and data space for an
enum. This requires significantly more documentation and care regarding ABI
stability before this can be stabilized.

For example:

```rust
#[repr(u32)]
pub enum Shape {
    Circle { radius: f32 } = 0,
    Rectangle { width: f32, height: f32 } = 1,
    _ = 2..=10,
}
```

- This reserves discriminants `2..=10` as valid for the `Shape` enum. It's not
  an ABI-breaking change to add new variants with data to `Shape` using these
  discriminants, so long as it doesn't affect the layout of the `Shape`.
- `Drop` glue is forbidden for field data (for a similar reason as `union`).
- The payload bytes of `Shape` are treated as opaque and never as padding.

By putting field data in an unnamed variant, `Shape` can specifically
reserve the size and alignment needed to hold future variants' fields:

```rust
#[repr(u32)]
pub enum Shape {
    Circle { radius: f32 } = 0,
    Rectangle { width: f32, height: f32 } = 1,

    // This reserves discriminants `2..=10` and the layout to hold a
    // thin pointer without breaking ABI. It's as if there were a variant
    // for `&'static ()` in the enum's internal `union`.
    _(&'static ()) = 2..=10,

    // Because of the above, it's not an ABI-breaking change to add this
    // variant since the layout won't be affected:
    // FromInfo { name: &'static ShapeInfo } = 2,
}
```

### Tuple-like syntax for `repr` enums

A very useful thing this RFC enables is that replacing this:

```rust
// The "newtype integer enum" pattern.
#[derive(PartialEq, Eq)]
pub struct Color(u32);
impl Color {
    const Red: Color = Color(0);
    const Blue: Color = Color(1);
    const Green: Color = Color(2);
}
```

with this:

```rust
#[derive(PartialEq, Eq)]
#[repr(u32)]
pub enum Color {
    Red,
    Blue,
    Green,
    _ = ..,
}
```

is a non-breaking change for client crates.

However, if the library initially exposed the discriminant field as `pub`, as
`bindgen`, `icu4x`, and `open-enum` do, then the migration to an open `enum`
requires that `Color(discriminant)` and `color.0` also function as originally.

These each have their own independent utility:

#### Tuple constructor

The enum name is a constructor `fn(Repr) -> Enum`:

```rust
assert_eq!(Color(1), Color::Blue);
assert!(
    [0, 3, 2].map(Color),
    matches!([Color::Red, _, Color::Green])
)
```

- This is valid for any open enum, the same as the `as` cast from integer.
- This mirrors the `derive(Debug)` format, is ergonomic, and is clear at
  callsite. Thus it may be worth adding to Rust even if `.0` isn't.
- When should one prefer the constructor over the `as` cast? Always?

#### Discriminant field access

`.0` provides direct access to the discriminant value:

```rust
let mut c = Color::Blue;
assert_eq!(c.0, 1);
c.0 += 1;
assert!(matches!(c, Color::Green));
assert_eq!(c.0, 2);
```

This is subjectively ugly and undiscoverable syntax to access the discriminant
of an `enum`. One possibility: when introduced, treat as deprecated and throw a
warning to recommend a better syntax than `.0` but still allow the desired
non-breaking migration.

There are a few distinct advantages compared to `as` casting:

- It is possible to get a reference directly to the discriminant, which can be
  useful when performing lifetime-constrained zero-copy serialization.
- The type of `.0` is exactly the `repr`, and doesn't require the user specify a
  type to `as` cast to and possibly truncate. Currently, there's no language
  feature in Rust that does this - it requires a macro or codegen to guarantee.
  This can cause subtle bugs, especially for `repr(C)`:

  ```rust
  #[repr(C)]
  enum Oops {
      // On any platform where this is more than `c_int::MAX`.
      TooBig = 2_147_483_649,
  }
  assert_eq!(Oops::TooBig as core::ffi::c_int, -2_147_483_647);
  ```

  Instead, `.0` accesses the discriminant without fear of truncation:

  ```rust
  assert_eq!(Oops::TooBig.0, 2_147_483_649);
  // mismatched types, expected `i32`, got `i64`
  // let _: c_int = X::V.0;
  ```

This could be supported for _any_ enum with an explicit `repr(Int)` by having
closed enums be `unsafe` to mutate through `.0` - it's an [unsafe field].

```rust
#[repr(u32)]
enum X {
    A = 0,
    B = 1,
}
let mut x = X::A;
assert_eq!(x.0, 0);

// SAFETY: 1 is a valid discriminant for `X`
unsafe { x.0 += 1; }

assert!(matches!(x, X::B));
```

[unsafe field]: https://rust-lang.github.io/rust-project-goals/2025h2/unsafe-fields.html

### Extracting the integer value of the discriminant for fielded enums

A fielded enum with `#[repr(Int)]` and/or `#[repr(C)]` is guaranteed to have its
discriminant values starting from 0. However, for any given value of that enum,
there's no built-in way to extract what the integer value of the discriminant is
safely. The unsafe mechanism is `(&thenum as *const _ as *const Int).read()`.
For open fielded enums, this would be even more valuable, since the discriminant
could be entirely unknown and the programmer may want to know its value.

Perhaps this uses the same `.0` syntax as above, or an extension to
`mem::Discriminant`?

### `match` on ranges of enums

```rust
#[repr(u32)]
enum HttpStatusCode {
    Ok = 200,
    NoContent = 204,
    Internal = 500,
    Unavailable = 503,
    _ = 100..=599,
}
let code = unsafe { transmute(301u32) };
let name = match code {
    HttpStatusCode::Ok => "ok",
    HttpStatusCode::NotFound => "not found",

    // Matches on discriminants 500..=503.
    HttpStatusCode::Internal..=HttpStatusCode::Unavailable =>
        "lower server error",

    // Explicit `repr` allows matching on the discriminant value.
    100..=199 => "info",
    200..=299 => "success",
    300..=399 => "redirection",
    400..=499 => "client error",
    500..=599 => "server error",

    // Exhaustive match, no wildcard branch needed.
}
```
