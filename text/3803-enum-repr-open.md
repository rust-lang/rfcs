- Feature Name: `enum_repr_open`
- Start Date: 2025-04-21
- RFC PR: [rust-lang/rfcs#3803](https://github.com/rust-lang/rfcs/pull/3803)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)


# Summary
[summary]: #summary

Allow `#[repr(_, open)]` on field-less (C-style) enums to allow the enum to contain unknown variants, which enables using the enum directly in FFI.

Enums with the `open` modifier cannot be matched on exhaustively without the use of a wildcard arm. This feature is similar but distinct from the `#[non_exhaustive]` attribute, since it works on an ABI level and applies in the defining module as well.

Example:

```rust
#[repr(u8, open)] // ABI stable and allowed in FFI
pub enum Version {
    Http0_9,
    Http1_0,
    Http1_1,
    Http2_0,
    Http3_0,
}

match version {
    Version::Http0_9 | Version::Http1_0 | Version::Http1_1 => println!("good old"),
    Version::Http2_0 | Version::Http3_0 => println!("ooh, fancy!"),
    // Fallback arm is required, even in the same crate.
    _ => println!("from the future!"),
}
```


# Motivation
[motivation]: #motivation

## Enums in Foreign Function Interfaces (FFI)

When using C enums in Rust across FFI, the common workaround is to wrap them in a newtype `struct` and use associated constants for the enum variants as follows (see the equivalent C code in [the guide-level explanation][guide-level-explanation]):

```rust
#[repr(transparent)]
pub struct Weather(pub u8);

impl Weather {
    pub const SUNNY: Self = Self(0);
    pub const WINDY: Self = Self(1);
    pub const RAINY: Self = Self(2);
}
```

This works, but looses a lot of the things that make Rust enums great, including:
- `rust-analyzer` is no longer able to fill out `match` statements automatically, and its suggestions are generally worse.
- It is unclear how to name the enum variants. SCREAMING_SNAKE_CASE (because they're technically `const`s) or CamelCase (because they're semantically enum variants)?
- Generated `rustdoc` documentation is less clear (the enum is documented in the "structs" section).
- Match exhaustiveness checking doesn't work, so it's hard to know if you've matched on all variants.
- You cannot (yet?) import associated constants like you can for enum variants.
- Derived impls like `Debug` are a lot less helpful.

Another common option is to expose the enum as constants in a `-sys` crate, and then re-expose it in a wrapper crate as a Rust enum, like the following:

```rust
// in *-sys crate

pub type Weather = u8;

pub const WEATHER_SUNNY: Weather = 0;
pub const WEATHER_WINDY: Weather = 1;
pub const WEATHER_RAINY: Weather = 2;

// in wrapper crate.

pub enum Weather {
    Sunny = ffi::WEATHER_SUNNY,
    Windy = ffi::WEATHER_WINDY,
    Rainy = ffi::WEATHER_RAINY,
}

pub struct UnknownWeather;

impl TryFrom<u8> for Weather {
    type Error = UnknownWeather;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match u8 {
            ffi::WEATHER_SUNNY => Ok(Self::Sunny),
            ffi::WEATHER_WINDY => Ok(Self::Windy),
            ffi::WEATHER_RAINY => Ok(Self::Rainy),
            _ => Err(UnknownWeather),
        }
    }
}
```

This also has several disadvantages, including:
- It is very cumbersome (you have to maintain enum bindings twice, along with conversions).
- It quickly becomes out of date with the upstream C library, so older bindings are often slightly incorrect.
- Panicking is a common choice for handling the unknown variant (even if one is going to match non-exhaustively, since `match { Ok(Weather::Sunny) => ... }` tend to be quite verbose).
- It is a bit inefficient (wrapper crates often eagerly check the variants even if the user did not need to check them).

## Binary format serialization/deserialization

When performing zero-copy deserialization of binary data formats, it is often desirable to map a set of bytes to a set of known values, while still handling unknown values. For example, when parsing the Mach-O `LC_BUILD_VERSION` load command, it is desirable to handle the `platform` field as an enum with [certain known values](https://docs.rs/goblin/0.9.3/src/goblin/mach/load_command.rs.html#1336-1347).

This is similar to the case above, but shows that there is value in this feature outside FFI and C interop.

## Greater flexibility for library authors

Winit, a popular GUI creation library in the Rust ecosystem, provides a [`MouseButton`](https://docs.rs/winit/0.30.9/winit/event/enum.MouseButton.html) enum with a few variants like `Left` and `Right`, and a catch-all `Other(u16)` for buttons that do not have a classical semantic meaning.

This is slightly wrong though, since the value `0` actually represents `MouseButton::Left`, but can now be incorrectly represented in the enum as `MouseButton::Other(0)` too. Besides, the enum is slightly larger than it needs to be, which is inefficient.

Winit could have used the `struct` + `const`s pattern, but other mouse button mappings are comparatively rare that the cost of doing this is too high.

The `bitflags` crate might also benefit from marking their enums as `#[repr(_, open)]`.


# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Imagine you are using a C library that queries the current weather, perhaps from humidity and pressure sensors on the user's device. The library provides the result as a C enumeration.

The author of the library has helpfully provided the following C header:

```c
// weather.h
// A service for guessing the current weather.

// The different kinds of weather.
typedef enum weather : uint8_t { // C23 syntax
    weather_sunny = 0,
    weather_windy = 1,
    weather_rainy = 2,
} weather_t;

// Guess the current weather.
weather_t weather_current(void);
```

This API could be translated to Rust as follows:

```rust
//! Bindings to a service for guessing the current weather.

/// The different kinds of weather.
#[repr(u8, open)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Weather {
    Sunny = 0,
    Windy = 1,
    Rainy = 2,
}

#[link(name = "weather")]
// SAFETY: The function signature is correct (return type `weather_t` == `Weather`).
unsafe extern "C" {
    // SAFETY: No preconditions for using this API.
    safe fn weather_current() -> Weather;
}

impl Weather {
    /// Guess the current weather.
    pub fn current() -> Self {
        weather_current()
    }
}
```

The `#[repr(u8)]` makes it so that the `Weather` enum has the same size as `weather_t` (matches the `uint8_t`), and the `open` modifier is required because C enums are non-exhaustive and can have values added to them at any time in an API- and ABI-compatible update.

An example usage of this binding might be:

```rust
match Weather::current() {
    Weather::Sunny => println!("Nice an sunny!"),
    Weather::Windy => println!("A bit breezy."),
    Weather::Rainy => println!("Let's go inside and grab a cup of tea."),
    weather => eprintln!("Unable to determine the weather, got enum value {}.", weather as u8),
}
```

With the last match arm being required because the enum might gain more variants over time. Omitting the match arm, the error message might look something like the following (similar to `#[non_exhaustive]`):

```
error: non-exhaustive patterns: `_` not covered
 --> example.rs:4:11
  |
  |     match Weather::current() {
  |           ^^^^^^^^^^^^^^^^^^ pattern `_` not covered
  |
  = note: `Weather` may have unknown variants, so a wildcard `_` is necessary to match exhaustively
```

If the author of the weather library were to release a new version which added the capability to determine whether it's snowing too:

```diff
 typedef enum weather : uint8_t {
     weather_sunny = 0,
     weather_windy = 1,
     weather_rainy = 2,
+    weather_snowy = 3,
 } weather_t;
```

Then ~you suddenly start encountering weird crashes where an `Option<Weather>` inexplicably turn into a `None`~ everything continues to work as expected, even without recompiling your Rust code, because this variant was already assumed to be present and thus already handled.


# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## Syntax

The syntax is not completely decided on, see [the unresolved questions section][unresolved-questions].

## Restrictions on the modifier

The `open` modifier can be used inside `#[repr]` attributes on enums.

The enum must have an explicit representation, that is, either a [primitive representation](https://doc.rust-lang.org/1.86.0/reference/type-layout.html#primitive-representations) or the [`C` representation](https://doc.rust-lang.org/1.86.0/reference/type-layout.html#the-c-representation).

For the initial implementation, we allow only field-less enums ["without explicit discriminants, or where only unit variants are explicit"](https://doc.rust-lang.org/1.86.0/reference/items/enumerations.html#r-items.enum.discriminant.coercion.fieldless) (i.e. all enums that can be `as`-casted). [Extending this to enums with fields][enums-with-fields] is left as a future possibility.

## Semantic operation

Adding the `open` modifier acts as-if the compiler were to insert extra un-nameable enum variants, such that the enum is able to exhaustively represent every bit-pattern of the underlying integer type.

```rust
#[repr(u8, open)]
pub enum Weather {
    Sunny = 0,
    Windy = 1,
    Rainy = 2,
}

// Becomes internally:

#[repr(u8)]
pub enum Weather {
    Sunny = 0,
    Windy = 1,
    Rainy = 2,
    _3 = 3,
    _4 = 4,
    // ...
    _254 = 254,
    _255 = 255,
}
```

A more complex example could be:

```rust
#[repr(i16, open)]
pub enum Foo {
    A = 10,
    B = -3,
}

// Becomes internally:

#[repr(i16)]
pub enum Foo {
    _Min = -32768,
    // ...
    _Neg5 = -5,
    _Neg4 = -4,
    B = -3,
    _Neg2 = -2,
    _Neg1 = -1,
    // ...
    _8 = 8,
    _9 = 9,
    A = 10,
    _11 = 11,
    _12 = 12,
    // ...
    _Max = 32767,
}
```

## ABI

The above change has a profound impact on the ABI / the validity invariant of the enum: every bit pattern of the underlying integer type is now a valid bit pattern of the enum discriminant itself.

Stated alternatively, the niche layout optimization on the enum discriminant is disabled.

This allows you to safely transmute from the underlying bit-pattern to the enum itself, as shown in the following example:

```rust
impl From<u8> for Weather {
    fn from(value: u8) -> Self {
        // SAFETY: The enum is `open` and fieldless, and
        // can thus represent all bit-patterns of `u8`.
        unsafe { core::mem::transmute::<u8, Weather>(value) }
    }
}

assert_eq!(Weather::from(0), Weather::Sunny);
assert_eq!(Weather::from(1), Weather::Windy);
assert_eq!(Weather::from(2), Weather::Rainy);

let snowy = Weather::from(3);
assert_ne!(snowy, Weather::Sunny);
assert_ne!(snowy, Weather::Windy);
assert_ne!(snowy, Weather::Rainy);

assert_eq!(size_of::<Weather>(), size_of::<u8>());
// No niches for the `None` to fit in.
assert_ne!(size_of::<Weather>(), size_of::<Option<Weather>>());
```

And thus, using the enum at the FFI boundary is similarly possible.

## `as`-casting

Casting from the enum to the underlying integer continues to access the discriminant [as described in the reference](https://doc.rust-lang.org/1.86.0/reference/items/enumerations.html#casting). Building on the previous example:

```rust
assert_eq!(Weather::Sunny as u8, 0);
assert_eq!(snowy as u8, 3);
```

Note that this RFC does not propose a way to safely construct enums of the unknown variants, see [the future possibilities section][future-possibilities] for that.

## Discriminant

The `core::mem::Discriminant` of the enum exposed to the user should differ based on the actual underlying value, regardless of the variants not being available in the source code. Building on the previous example:

```rust
use core::mem::discriminant;
assert_eq!(discriminant(&snowy), discriminant(&snowy));
assert_ne!(discriminant(&snowy), discriminant(&Weather::Sunny));
assert_ne!(discriminant(&snowy), discriminant(&Weather::Windy));
assert_ne!(discriminant(&snowy), discriminant(&Weather::Rainy));
assert_ne!(discriminant(&snowy), discriminant(&Weather::from(42)));
```

## Usage in pattern matching

Adding the `open` modifier affects exhaustiveness-checking, since it adds extra variants.

Unlike `#[non_exhaustive]`, the check is enabled everywhere, regardless of visibility attributes and crate boundaries (because it works on the ABI level).

Whether exhaustiveness-checking is affected by `open` enums that already has enough variants to represent every bit-pattern of the underlying integer type depends on the final syntax, see [the unresolved questions section][unresolved-questions].

## Derives

`#[derive(PartialEq, Eq, PartialOrd, Ord, Hash)]` compares the discriminant, regardless of the source-ordering (like they do today).

`#[derive(Default)]` continues to require that a variant is marked with `#[default]`.

The exact output of `#[derive(Debug)]` is [an unresolved question][derivedebug].

User-defined derives and proc-macros will have to know when to emit the extra match arm (so most user-defined derive macros probably won't support `#[repr(_, open)]` initially). Proc-macros could either choose to inspect the `repr`, or to defensively emit an extra `#[allow(unreachable_patterns)] _ => {}` match arm.

Adding a crate for making such semantic inspections easier [is a future possibility][helper-crate-for-macro-authors].

## Handling unknown variants

Matching on unknown variants is possible:

```rust
const SNOWY: Weather = unsafe { core::mem::transmute::<u8, Weather>(3) };

match Weather::current() {
    Weather::Sunny => println!("Nice an sunny!"),
    Weather::Windy => println!("A bit breezy."),
    Weather::Rainy => println!("Let's go inside and grab a cup of tea."),
    SNOWY => println!("Do you wanna build a snowman?"),
    weather => eprintln!("Unable to determine the weather, got enum value {}.", weather as u8),
}
```

And would continue to work even if the `Weather::Snowy` variant was added in the future.


# Drawbacks
[drawbacks]: #drawbacks

This adds surface area to the language for something that could be considered a relic from the C past.

It may be confusing when to use this over `#[non_exhaustive]` (see also the syntax discussion in [the unresolved questions section][unresolved-questions]).


# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

## Do nothing

We could do nothing, and continue to recommend the `struct` + `const`s approach.

## Add struct attribute

We could add a helper attribute `#[diagnostics::enum_like]` on newtype `struct`s (or maybe on the `impl`) that could allow providing a user-experience more similar to what you get from regular enums.

## Wait for pattern types, and use that in public API instead

A common suggestion is to wait for [pattern types](https://internals.rust-lang.org/t/thoughts-on-pattern-types-and-subtyping/17675), define their interaction with niches, and instead suggest people write:

```rust
#[repr(bikeshed_guarantee_abi(u8))]
pub enum Weather {
    Sunny = 0,
    Windy = 1,
    Rainy = 2,
    Unknown(u8 in 3..=255),
}
```

Usage could then be:

```rust
match Weather::current() {
    Weather::Sunny => println!("Nice an sunny!"),
    Weather::Windy => println!("A bit breezy."),
    Weather::Rainy => println!("Let's go inside and grab a cup of tea."),
    Weather::Unknown(value) => eprintln!("Unable to determine the weather, got enum value {value}"),
}
```

However, this is problematic, since the unknown variant can be matched on explicitly. Consider e.g.:

```rust
const SNOWY: Weather = Weather::Unknown(4);

if let SNOWY = Weather::current() {
    println!("Do you wanna build a snowman?");
}
```

If the author of the Rust binding were to add `Weather::Snowy`, that would be a breaking change, regardless of the presence of `#[non_exhaustive]` (which makes it hard to use in bindings). The `Unknown` variant would have to be `#[doc(hidden)]` for that to work (and then, how would you then get the actual value out?), but avoiding such hacks was the motivation for adding `#[non_exhaustive]` in the first place.

This is also a lot more error-prone, as you'd have to ensure that the niches in the pattern type does not overlap with the other variants. Consider e.g. the following innocuous change:

```diff
 #[repr(bikeshed_guarantee_abi(u8))]
 pub enum Weather {
     Sunny = 0,
     Windy = 1,
     Rainy = 2,
+    Snowy = 3,
     Unknown(u8 in 3..=255),
 }
```

This would be a fairly large bug, since the `Weather` enum suddenly grew to 2 bytes in size (instead, the `Unknown` variant should also have been updated to `u8 in 4..=255`).

(That's not to say that pattern types are completely useless here, they could in fact have a very sensible interaction with this feature, see the [pattern types in `repr`][pattern-types-in-repr] future possibility).

## Define semantics in terms of pattern types

Similar to above, but instead only define the semantics in terms of pattern types:

```rust
#[repr(u8, open)]
pub enum Weather {
    Sunny = 0,
    Windy = 1,
    Rainy = 2,
}

// Becomes internally:

#[repr(u8)]
pub enum Weather {
    Sunny = 0,
    Windy = 1,
    Rainy = 2,
    _Unknown(u8 in 3..=255),
}
```

This clashes with Rust's existing concept of enum discriminants though, and thus is unclear what `core::mem::discriminant` would return for the `_Unknown` variant. It is also unclear how `as` casts would work (since those only work on field-less enums).

But if these issues were resolved, we could in the future consider `#[repr(_, open)]` to simply be a shorthand for having a hidden variant that uses pattern types.

## `std` macro

This cannot feasibly be implemented as a helper macro in the standard library, as filling out the enum with fake variants very quickly grows to impossible amounts of data that would have to be sent to the compiler.

## Use unions

An alternative would be to use `union`s together with some modifications to how `match` works to soundly look through certain kinds of unions such that one could write (without `unsafe`):

```rust
#[repr(u8)]
enum KnownWeather {
    Sunny = 0,
    Windy = 1,
    Rainy = 2,
}

union Weather {
   known: KnownWeather,
   unknown: u8,
}

const SNOWY: u8 = 4;

match Weather::current() {
   Weather { known: KnownWeather::Sunny } => {...}
   Weather { known: KnownWeather::Windy } => {...}
   Weather { known: KnownWeather::Rainy } => {...}
   Weather { unknown: SNOWY } => {...}
   _ => {...}
}
```

Advantages:
- Would allow us to avoid adding any new syntax.
- The known variants and unknown variants are clearly separate types.

Disadvantages:
- It is quite verbose, and would thus still push library authors towards avoiding exposing the union in favour of instead panicking on unknown variants.
- You'd have to ensure to match the two `u8`s, and it is unclear what type would you use for `#[repr(C)]`.
- Modifying the rules around `union` and `unsafe` might be difficult to do soundly.


# Prior art
[prior-art]: #prior-art

C enums are "open" / ABI-stable non-exhaustive by default, and only some implementations allow opting out of that (such as Clang's [`enum_extensibility(closed)`](https://clang.llvm.org/docs/AttributeReference.html#enum-extensibility) attribute, although this does not come with any niche guarantees).

Zig [non-exhaustive enums](https://ziglang.org/documentation/0.14.0/#Non-exhaustive-enum) are "open" in the same sense as C.

Swift enums are "closed" by default, but a library developer can enable [library-evolution mode](https://www.swift.org/blog/library-evolution/) to mark all enums as "open", and then later selectively mark enums as "closed" with [the `@frozen` attribute](https://docs.swift.org/swift-book/documentation/the-swift-programming-language/attributes#frozen).

In the Rust ecosystem, [there](https://docs.rs/fake-enum) [are](https://docs.rs/c-enum) [several](https://docs.rs/open-enum) [crates](https://docs.rs/ffi-enum) for translating what is syntactically an `enum` to a `struct` + `const`s. Even `bindgen` has implemented [a feature](https://docs.rs/bindgen/0.71.1/bindgen/enum.EnumVariation.html#variant.Rust) for users to just generate unsound Rust enums instead, mainly because this is such a pain point.


# Unresolved questions
[unresolved-questions]: #unresolved-questions

## Syntax

Current options and sub-options:
1. Use `#[repr(_, something_here)]`.

    Advantage: The feature semantically modifies the layout/representation.

    1. `#[repr(_, open)]`

        Advantage: Matches Clang's existing naming.

        Disadvantage: Might be unclear what "open" means. Open to what?

    2. `#[repr(_, no_niches)]`.

        Advantage: Very precise in what it does.

        Disadvantages: Uses "niche" which is a niche term.
        Poor framing because it disables optimizations instead of allowing something new.

    3. `#[repr(_, non_exhaustive)]`.

        Advantage: Clear that it implies `#[non_exhaustive]`.

        Disadvantage: Might be confusing for users which kind of `non_exhaustive` they should use.

    4. `#[repr(_, abi_stable)]`.
    5. `#[repr(_, all_bits_valid)]`.
    6. `#[repr(_, allow_undeclared_variants)]`.
    7. `#[repr(_, niches(on))]` (to match `niches(off)` if we wanted that).
    8. `#[repr(_, discriminant = open)]` to mirror [RFC 3659](https://github.com/rust-lang/rfcs/pull/3659).
2. Use `#[non_exhaustive(something_here)]`.

    Advantage: Might be easier to explain the effect to users ("this works just like `#[non_exhaustive]`, except stronger").

    Advantage: Might align better with future additions to `#[non_exhaustive]`, such as [`#[non_exhaustive(pub)]`](https://internals.rust-lang.org/t/pre-rfc-relaxed-non-exhaustive-structs/11977).

    1. `#[non_exhaustive(open)]`.
    2. `#[non_exhaustive(abi)]`.
    3. `#[non_exhaustive(repr)]`.
    4. `#[non_exhaustive(layout)]`.
3. New attribute `#[open]`, `#[abi_stable]`, `#[really_non_exhaustive]` or similar.
4. New keyword like `open_enum Weather { ... }`.

We have a kind of decision tree here, where some unresolved questions depend on the syntax. The exact syntax does not need to be decided before accepting the RFC, though we should choose one of the main "branches".

The RFC author himself is undecided on the syntax.

## Should `#[non_exhaustive]` be required

If we choose syntax 1, it might be desirable to require the `#[non_exhaustive]` attribute as well for clarity?

```rust
#[repr(u8, open)]
#[non_exhaustive] // Maybe required?
enum Weather {
    Sunny = 0,
    Windy = 1,
    Rainy = 2,
}
```

If we don't require it, what are the semantic differences between the two cases? Do we warn if both are supplied? Or do we give different error messages when not matching exhaustively based on if the enum uses `#[non_exhaustive]` on `#[repr(_, open)]` vs. not using it?

See also the [exhaustive match of known values][exhaustive-match-of-known-values] future possibility.

## Are full enums `#[non_exhaustive]`?

Choosing syntax 2 seems to imply that `#[non_exhaustive]` is semantically present.

This has implications for the following corner-case:

```rust
#[repr(u8, open)]
pub enum AllVariantsPresent {
    X00 = 0x00,
    X01 = 0x01,
    X02 = 0x02,
    // ...
    XFE = 0xfe,
    XFF = 0xff,
}

match AllVariantsPresent::X00 {
    AllVariantsPresent::X00 => (),
    AllVariantsPresent::X01 => (),
    AllVariantsPresent::XAC => (),
    // ...
    AllVariantsPresent::XFE => (),
    AllVariantsPresent::XFF => (),

    // Should we require this match arm? It can never be reached ABI-wise,
    // because every bit-pattern of the underlying u8 is exhausted, but a
    // `#[non_exhaustive]` enum would require it because it (intentionally)
    // doesn't concern itself with the implementation/contents of an enum.
    _ => (),
}
```

## `#[derive(Debug)]`
[derivedebug]: #derivedebug

`#[derive(Debug)]` could expand to something like:

```rust
impl Debug for Weather {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Sunny => write!(f, "Sunny"),
            Self::Windy => write!(f, "Windy"),
            Self::Rainy => write!(f, "Rainy"),
            value => write!(f, "{}", value as u8),
        }
    }
}
```

How the unknown variants should be represented is not immediately clear, though it should probably be something that makes sense to come after the string `"Weather::"`.

Ideas:
- `{value}` (e.g. `Weather::3`).
- `_{value}` (e.g. `Weather::_3`).
- `#unknown({value})` (e.g. `Weather::#unknown(3)`).

Regardless, this is probably `T-libs` purview, and can be decided in a FCP during the implementation phase.

## Opposite modifier

There are a few future possibilities below where it makes sense to have the opposite modifier, something like `#[repr(_, closed)]`.

We should consider before stabilization whether we should add such a modifier at the same time, to allow for better forwards compatibility.


# Future possibilities
[future-possibilities]: #future-possibilities

## Construction

It would be nice to have some way to safely convert from the underlying integer type to both known and unknown variants. Possibly `3u8 as Weather`? Or if `as` is being deprecated, derives for `From` like proposed in [RFC 3604](https://github.com/rust-lang/rfcs/pull/3604)?

## FFI Diagnostics

A warning could be emitted when using raw `enum` in C FFI as a return type, or as a parameter type behind a pointer (e.g. an out pointer `&mut Weather`). This would have notified the developer in the "guide level explanation" section of the unsoundness immediately, instead of later.

This might need some way to suppress the warning on the enum itself (`#[repr(_, closed)]`? `#[exhaustive]`?), since certain enums _are_ valid to use across FFI. Examples of this include C99's `_Bool` if it were an enum, enums marked `__attribute__((enum_extensibility(closed)))`, enums such as `CompassDirection` (there are only ever four cardinal directions) and enums otherwise documented as exhaustive.

## Enums with fields
[enums-with-fields]: #enums-with-fields

The `open` modifier apply specifically to the enum discriminant. This could be extended to [`#[repr(_)]` enums with fields](https://doc.rust-lang.org/1.86.0/reference/type-layout.html#reprc-enums-with-fields), and would translate to the `open` option being present on the internal discriminant enum.

Building on the reference link:

```rust
#[repr(C, open)] // `open` was added
enum MyEnum {
    A(u32),
    B(f32, u64),
    C { x: u32, y: u8 },
    D,
}

// Would have this discriminant enum.
#[repr(C, open)]
enum MyEnumDiscriminant { A, B, C, D }
```

## Zero-variant enums

The reference [states](https://doc.rust-lang.org/1.86.0/reference/type-layout.html#r-layout.repr.primitive.constraint):
> It is an error for zero-variant enums to have a primitive representation

We could consider relaxing this for `open` enums, since these are actually inhabited:

```rust
#[repr(u8)] // Disallowed.
pub enum Never {}
let never = unsafe { core::mem::transmute::<u8, Never>(42) }; // Unsound.

#[repr(u8, open)] // Could be allowed.
pub enum Empty {}
let empty = unsafe { core::mem::transmute::<u8, Empty>(42) }; // Sound.
```

Though when to use this over `pub type Empty = u8;` or `pub struct Empty(u8);` is yet unclear.

## Further progress stable Rust ABI

Rust does not have a stable ABI, but there are [thoughts](https://faultlore.com/blah/swift-abi/) [on](https://docs.rs/abi_stable) [improving](https://docs.rs/stabby) [that](https://github.com/rust-lang/rfcs/pull/3470), and having a way to define an enum's discriminants as ABI-stable could be a useful stepping stone.

If Rust intends to make big strides here, it might make sense to tailor the syntax of this feature towards that? Perhaps something like an `#[abi_stable]` attribute that would also be usable on `struct`s and `trait`s? But it probably shouldn't block this RFC, as it can always be added later.

## Changing the default

Making `#[repr(C)]` (and maybe `#[repr(uXX)]`?) enums have the `open` modifier by default across an edition boundary could make interoperability with C easier and even further remove the footgun (anecdotally, I've seen several people surprised that `#[repr(C)] enum` isn't a good idea for interfacing with C enums).

Would perhaps need some way to re-opt-in to niche optimizations, maybe `#[repr(C, closed)]`?

## Pattern types in `repr`
[pattern-types-in-repr]: #pattern-types-in-repr

This feature could work nicely with having [pattern types](https://internals.rust-lang.org/t/thoughts-on-pattern-types-and-subtyping/17675) in the enum `repr`, since it would allow defining very precisely the actual valid bit-patterns of the enum's discriminant.

```rust
#[repr(u8 in 1..=255, open)]
pub enum NonZeroU8 {
    One = 1,
    Two = 2,
    Max = 255,
}
```

## Exhaustive match of known values
[exhaustive-match-of-known-values]: #exhaustive-match-of-known-values

We could add a way to allow users to ensure that they're handling all _currently_ known variants.

This would need some way to differentiate between a `_` pattern that is used as a fallback for unknown variants, and a `_` pattern that is used as a general fallback.

Swift does this with an [`@unknown default`](https://docs.swift.org/swift-book/documentation/the-swift-programming-language/statements/#Switching-Over-Future-Enumeration-Cases) case instead of just a normal `default` case.

An example in Rust could be:

```rust
match Weather::current() {
    Weather::Sunny => println!("Nice an sunny!"),
    Weather::Windy => println!("A bit breezy."),
    Weather::Rainy => println!("Let's go inside and grab a cup of tea."),
    #[unknown] weather => eprintln!("Unable to determine the weather, got enum value {}", weather as u8),
}
```

From a SemVer perspective, this would separate ABI from API, so we could say that adding `Weather::Snowy` (without `#[non_exhaustive]` on the enum) would be an API or source-breaking change (even though it wouldn't be ABI-breaking).

## Match diagnostics

Similar to above, it might be desirable for diagnostics to differ between `#[repr(_, open)]` and `#[non_exhaustive]`.

So e.g. for something like the following:

```rust
match Weather::current() {
    Weather::Sunny => println!("Nice an sunny!"),
    Weather::Windy => println!("A bit breezy."),
}
```

The diagnostic might suggest to add both `Weather::Rainy` and the `_` blanket pattern.

And maybe `match` statements like `match Weather::current() { _ => {} }` would warn by default about possibly unhandled cases?

See also the [`non_exhaustive_omitted_patterns`](https://doc.rust-lang.org/rustc/lints/listing/allowed-by-default.html#non-exhaustive-omitted-patterns) lint.

## Helper crate for macro authors
[helper-crate-for-macro-authors]: #helper-crate-for-macro-authors

It might be useful to publish a helper crate that could help with parsing the `open` modifier.

An alternative would be to explicitly document how macro authors are expected to deal with open enums.
