# Safer Transmute RFC

- Feature Name: `safer_transmute`
- Start Date: 2020-08-31
- RFC PR: [rust-lang/rfcs#2981](https://github.com/rust-lang/rfcs/pull/2981)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)


# Summary
[summary]: #summary

We propose traits, namely `TransmuteFrom`, that are implemented *automatically* for combinations of types that may be safely transmuted. In other words, this RFC makes safe transmutation *as easy as 1..., 2..., `repr(C)`!*
```rust
#[repr(C)]
pub struct Foo(pub u8, pub u16);
//                    ^ there's a padding byte here, between these fields

// Transmute fearlessly!
let _ : Foo = transmute!(64u32); // Alchemy Achieved!

let _ : u32 = transmute!(Foo(16, 12)); // Compile Error!

// error[E0277]: the trait bound `u32: TransmuteFrom<Foo, _>` is not satisfied
//   --> src/demo.rs:7:27
//    |
//  7 | let _ : u32 = transmute!(Foo(16, 12)); // Compile Error!
//    |                          ^^^^^^^^^^^ the trait `TransmuteFrom<Foo, _, _>` is not implemented for `u32`
//    |
//   = note: byte 8 of the source type may be uninitialized; byte 8 of the destination type cannot be uninitialized.
```


# Motivation
[motivation]: #motivation

Byte-reinterpretation conversions (such as those performed by `mem::transmute`, `mem::transmute_copy`, pointer casts, and `union`s) are invaluable in high performance contexts, are `unsafe`, and easy to get wrong. This RFC provides mechanisms that make many currently-unsafe transmutations entirely safe. For transmutations that are not entirely safe, this RFC's mechanisms make mistakes harder to make.

This RFC's comprehensive approach provides additional benefits beyond the mere act of transmutation; namely:
 - [authoritatively codifies language layout guarantees](#codifying-language-layout-guarantees)
 - [allows crate authors to codify their abstractions' layout requirements](#expressing-layout-requirements)

Given the expressive foundation provided by this RFC, we also envision a range of future possibilities that will *not* require additional compiler support, including:
 - [safe slice and `Vec` casting][0000-ext-container-casting.md]
 - [a unified, generic `Atomic<T>` type][0000-ext-generic-atomic.md]
 - [a safe, generic alternative to `include_bytes!`][0000-ext-include-data.md]
 - [traits for asserting the size and alignment relationships of types][0000-ext-layout-traits.md]
 - [zerocopy-style traits for safe initialization][0000-ext-byte-transmutation.md]
 - [bytemuck-style mechanisms for fallible reference casting][ext-ref-casting]


## Codifying Language Layout Guarantees
Documentation of Rust's layout guarantees for a type are often spread across countless issues, pull requests, RFCs and various official resources. It can be very difficult to get a straight answer. When transmutation is involved, users must reason about the *combined* layout properties of the source and destination types.

This RFC proposes mechanisms that programmers will use to confidently answer such questions—by checking whether the `TransmuteFrom` trait is implemented.

## Expressing Layout Requirements
Similarly, there is no canonical way for crate authors to declare the layout requirements of generic abstractions over types that have certain layout properties. 

For instance, a common bit-packing technique involves using the relationship between allocations and alignment. If a type is aligned to 2<sup>n</sup>, then the *n* least significant bits of pointers to that type will equal `0`. These known-zero bits can be packed with data. Since alignment cannot be currently reasoned about at the type-level, it's currently impossible to bound instantiations of a generic parameter based on minimum alignment.

The mechanisms proposed by the RFC enable this, see [here][0000-ext-layout-traits.md].

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

## Terminology & Concepts

### 📖 Transmutation
**Transmutation** is the act of reinterpreting the bytes corresponding to a value of one type as if they corresponded to a different type. Concretely, we mean the behavior of this function:
```rust
#[inline(always)]
unsafe fn transmute<Src, Dst>(src: Src) -> Dst
{
    #[repr(C)]
    union Transmute<Src, Dst> {
        src: ManuallyDrop<Src>,
        dst: ManuallyDrop<Dst>,
    }

    ManuallyDrop::into_inner(Transmute { src: ManuallyDrop::new(src) }.dst)
}
```

### 📖 Safer Transmutation
By **safer transmutation** we mean: *what `where` bound could we add to `transmute` restricts its type parameters `Src` and `Dst` in ways that statically limit the function's misuse?* Our answer to this question will ensure that transmutations are, by default, *well-defined* and *safe*.

### 📖 Well-Definedness
A transmutation is ***well-defined*** if the mere act of transmuting a value from one type to another is not unspecified or undefined behavior.

### 📖 Safety
A well-defined transmutation is ***safe*** if *using* the transmuted value cannot violate memory safety.

## Concepts in Depth

***Disclaimer:** While the high-level definitions of transmutation well-definedness and safety is a core component of this RFC, the detailed rules and examples in this section are **not**. We expect that the initial implementation of `TransmuteFrom` may initially be considerably less sophisticated than the examples in this section (and thus forbid valid transmutations). Nonetheless, this section explores nuanced cases of transmutation well-definedness and safety to demonstrate that the APIs we propose can grow to handle that nuance.*


### 📖 When is a transmutation well-defined?
[sound transmutation]: #-when-is-a-transmutation-well-defined
A transmutation is ***well-defined*** if the mere act of transmuting a value from one type to another is not unspecified or undefined behavior.

#### Well-Defined Representation
[`u8`]: core::u8
[`f32`]: core::f32

Transmutation is ill-defined if it occurs between types with unspecified representations.

Most of Rust's primitive types have specified representations. That is, the precise layout characteristics of [`u8`], [`f32`] is a documented and guaranteed aspect of those types.

In contrast, most `struct` and `enum` types defined without an explicit `#[repr(C)]` attribute do ***not*** have well-specified layout characteristics.

To ensure that types you've define are transmutable, you almost always (with very few exceptions) must mark them with the `#[repr(C)]` attribute.

#### Requirements on Owned Values
[transmute-owned]: #requirements-on-owned-values

Transmutations involving owned values must adhere to two rules to be well-defined. They must:
 * [preserve or broaden the bit validity][owned-validity], and
 * [preserve or shrink the size][owned-size].

##### Preserve or Broaden Bit Validity
[owned-validity]: #Preserve-or-Broaden-Bit-Validity
[`NonZeroU8`]: https://doc.rust-lang.org/beta/core/num/struct.NonZeroU8.html

The bits of any valid instance of the source type must be a bit-valid instance of the destination type.

For example, we are permitted to transmute a `Bool` into a [`u8`]:
```rust
#[repr(u8)]
enum Bool {
    True = 1,
    False = 0,
}

let _ : u8 = transmute!(Bool::True);
let _ : u8 = transmute!(Bool::False);
```

...because all possible instances of `Bool` are also valid instances of [`u8`]. However, transmuting a [`u8`] into a `Bool` is forbidden:
```rust
/* ⚠️ This example intentionally does not compile. */
let _ : Bool = transmute!(u8::default()); // Compile Error!
```
...because not all instances of [`u8`] are valid instances of `Bool`.

Another example: While laying out certain types, Rust may insert padding bytes between the layouts of fields. In the below example `Padded` has two padding bytes, while `Packed` has none:
```rust
#[repr(C)]
#[derive(Default)]
struct Padded(pub u8, pub u16, pub u8);

#[repr(C)]
#[derive(Default)]
struct Packed(pub u16, pub u16, pub u16);

assert_eq!(mem::size_of::<Packed>(), mem::size_of::<Padded>());
```

We may safely transmute from `Packed` to `Padded`:
```rust
let _ : Padded = transmute!(Packed::default());
```
...but not from `Padded` to `Packed`:
```rust
/* ⚠️ This example intentionally does not compile. */
let _ : Packed = transmute!(Padded::default()); // Compile Error!
```
...because doing so would expose two uninitialized padding bytes in `Padded` as if they were initialized bytes in `Packed`.

##### Preserve or Shrink Size
[owned-size]: #Preserve-or-Shrink-Size

It's well-defined to transmute into a type with fewer bytes than the source type; e.g.:
```rust
let _ : [u8; 16] = transmute!([u8; 32]::default());
```
This transmute truncates away the final sixteen bytes of the `[u8; 32]` value.

A value may ***not*** be transmuted into a type of greater size, if doing so would expose uninitialized bytes as initialized:
```rust
/* ⚠️ This example intentionally does not compile. */
let _ : [u8; 32] = transmute!([u8; 16]::default()); // Compile Error!
```

A `differing_sizes` lint reports warnings for invocations of `transmute!()` where the source and destination types are different sizes.

#### Requirements on References
[transmute-references]: #requirements-on-references

The [restrictions above that apply to transmuting owned values][transmute-owned] also apply to transmuting references. However, references carry a few *additional* restrictions.

A [well-defined transmutation] of references must:
 - [preserve or shrink size][reference-size],
 - [preserve or relax alignment][reference-alignment],
 - [preserve or shrink lifetimes][reference-lifetimes],
 - [preserve or shrink uniqueness][reference-mutability], and
 - and if the destination type is a mutate-able reference, [preserve validity][reference-validity].

##### Preserve or Shrink Size
[reference-size]: #Preserve-or-Shrink-Size

You may preserve or decrease the size of the referent type via transmutation:
```rust
let _: &[u8; 3] = transmute!(&[0u8; 9]);
```

However, you may **not**, under any circumstances, *increase* the size of the referent type:
```rust
/* ⚠️ This example intentionally does not compile. */
let _: &[u8; 9] = transmute!(&[0u8; 3]); // Compile Error!
```
##### Preserve or Relax Alignment
[reference-alignment]: #Preserve-or-Relax-Alignment

Unaligned loads are undefined behavior. You may transmute a reference into reference of more relaxed alignment:
```rust
let _: &[u8; 0] = transmute!(&[0u16; 0]);
```

However, you may **not** transmute a reference into a reference of more-restrictive alignment:
```rust
/* ⚠️ This example intentionally does not compile. */
let _: &[u16; 0] = transmute!(&[0u8; 0]); // Compile Error!
```

##### Preserve or Shrink Lifetimes
[reference-lifetimes]: #Preserve-or-Shrink-Lifetimes

You may transmute a reference into a reference of lesser lifetime:
```rust
fn shrink<'a>() -> &'a u8 {
    static long : &'static u8 = &16;
    transmute!(long)
}
```

However, you may **not** transmute a reference into a reference of greater lifetime:
```rust
/* ⚠️ This example intentionally does not compile. */
fn extend<'a>(short: &'a u8) -> &'static u8 {
    transmute!(short) // Compile Error!
}
```

##### Preserve or Shrink Uniqueness
[reference-mutability]: #Preserve-or-Shrink-Uniqueness

You may preserve or decrease the uniqueness of a reference through transmutation:
```rust
let _: &u8 = transmute!(&42u8);
let _: &u8 = transmute!(&mut 42u8);
```

However, you may **not** transmute a shared reference into a unique reference:
```rust
/* ⚠️ This example intentionally does not compile. */
let _: &mut u8 = transmute!(&42u8); // Compile Error!
```

##### Mutate-able References Must Preserve Validity
[reference-validity]: #Mutate-able-References-Must-Preserve-Validity

A mutate-able reference is:
- all unique (i.e., `&mut T`) references
- all shared (i.e., `&T`) references whose referent type contain any bytes produced by the contents of `UnsafeCell`.

Unlike transmutations of owned values, the transmutation of a mutate-able reference may also not expand the bit-validity of the referenced type. For instance:
```rust
/* ⚠️ This example intentionally does not compile. */
let mut x = NonZeroU8::new(42).unwrap();
{
    let y : &mut u8 = transmute!(&mut x); // Compile Error!
    *y = 0;
}

let z : NonZeroU8 = x;
```
If this example did not produce a compile error, the value of `z` would not be a bit-valid instance of its type, [`NonZeroU8`].



### 📖 When is a transmutation safe?
A well-defined transmutation is ***safe*** if *using* the transmuted value safely cannot violate memory safety. Whereas well-definedness solely concerns the act of transmutation, *safety* is concerned with what might happen with a value *after* transmutation occurs. A well-defined transmutation must be safe if the involved types are *implicitly constructable*.

#### Implicit Constructability
A struct or enum variant is *fully implicitly constructable* at a given location only if, at that location, that type can be instantiated via its *implicit constructor*, and its fields are also *implicitly constructable*.

The *implicit constructor* of a struct or enum variant is the constructor Rust creates implicitly from its definition; e.g.:
```rust
struct Point<T> {
    x: T,
    y: T,
}

let p = Point { x: 4, y: 2 };
     // ^^^^^^^^^^^^^^^^^^^^ An instance of `Point` is created here, via its implicit constructor.
```

Limiting implicit constructability is the fundamental mechanism with which type authors build safe abstractions for `unsafe` code, whose soundness is dependent on preserving invariants on fields. Usually, this takes the form of restricting the visibility of fields. For instance, consider the type `NonEmptySlice`, which enforces a validity constraint on its fields via its constructor:

```rust
pub mod crate_a {

    #[repr(C)]
    pub struct NonEmptySlice<'a, T> {
        data: *const T,
        len: usize,
        lifetime: core::marker::PhantomData<&'a ()>,
    }

    impl<'a, T> NonEmptySlice<'a, T> {
        pub fn from_array<const N: usize>(arr: &'a [T; N], len: usize) -> Self {
            assert!(len <= N);
            assert!(len > 0);
            Self {
                data: arr as *const T,
                len,
                lifetime: core::marker::PhantomData,
            }
        }

        pub fn first(&self) -> &'a T {
            unsafe { &*self.data }
        }
    }

}
```
It is sound for `first` to be a *safe* method is because the `from_array` constructor ensures that `data` is safe to dereference, and because `from_array` is the *only* way to safely initialize `NonEmptySlice` outside of `crate_a` (note that `NonEmptySlice`'s fields are *not* `pub`). As a rule: any field that is not marked `pub` should be assumed to be private *because* it is subject to safety invariants.

Unfortunately, field visibility modifiers are not a surefire indicator of whether a type is *fully* implicitly constructable. A type author may restrict the implicit constructability of a type even in situations where all fields of that type (*and all fields of those fields*) are `pub`; consider:
```rust
pub mod crate_a {

    #[repr(C)]
    pub struct NonEmptySlice<'a, T>(pub private::NonEmptySliceInner<'a, T>);

    impl<'a, T> NonEmptySlice<'a, T> {
        pub fn from_array<const N: usize>(arr: &'a [T; N], len: usize) -> Self {
            assert!(len <= N && len > 0);
            Self(
                private::NonEmptySliceInner {
                    data: arr as *const T,
                    len,
                    lifetime: core::marker::PhantomData,
                }
            )
        }

        pub fn first(&self) -> &'a T {
            unsafe { &*self.0.data }
        }
    }

    // introduce a private module to avoid `private_in_public` error (E0446):
    pub(crate) mod private {
        #[repr(C)]
        pub struct NonEmptySliceInner<'a, T> {
            pub data: *const T,
            pub len: usize,
            pub lifetime: core::marker::PhantomData<&'a ()>,
        }
    }

}
```
In the above example, the definitions of both `NonEmptySlice` and its field `NonEmptySliceInner` are marked `pub`, and all fields of these types are marked `pub`. However, `NonEmptySlice` is *not* fully implicitly constructible outside of `crate_a`, because the module containing `NonEmptySliceInner` is not visibile outside of `crate_a`.

#### Constructability and Transmutation
Transmutation supplies a mechanism for constructing instances of a type *without* invoking its implicit constructor, nor any constructors defined by the type's author. In the previous examples, it would be *unsafe* to transmute `[usize; 2]` into `NonEmptySlice` outside `crate_a`, because subsequent *safe* use of that value (namely, calling `first`) would violate memory safety:
```rust
/* ⚠️ This example intentionally does not compile. */
// [usize; 2] ⟶ NonEmptySlice
let _: NonEmptySlice<'static, u8> = transmute!([0usize; 2]); // Compile Error: `NonEmptySlice<_, _>` is not constructible here.
```
If a field is private, then instantiating or modifying it via transmutation is not, generally speaking, safe.

For transmutations where the destination type involves mutate-able references, the constructability of the *source* type is also relevant. Consider:
```rust
/* ⚠️ This example intentionally does not compile. */
let arr = [0u8, 1u8, 2u8];
let mut x = NonEmptySlice::from_array(&arr, 2);
{
    // &mut NonEmptySlice ⟶ &mut [usize; 2]
    let y : &mut u128 = transmute!(&mut x) // Compile Error! `&mut NonEmptySlice` is not constructible here.
    *y[0] = 0;
    *y[1] = 0;
}

let z : NonEmptySlice<u8> = x;
```
If this example did not produce a compile error, the value of `z` would not be a safe instance of its type, `NonEmptySlice`, because `z.first()` would dereference a null pointer.

#### Constructability and Scope
Whether a type is fully implicitly constructable will depends on the *scope* in which that question is asked. Consider:
```rust
pub mod a {

    #[repr(C)] pub struct Foo(private::Bar);

    mod private {
        #[repr(C)] pub struct Bar;
    }

    // `Foo` is fully implicitly constructible in this module.
    const _: Foo = Foo { private::Bar };

    // Thus, `Foo` is transmutable in this module!
    const _: Foo = transmute!(());
}

pub mod b {
    use super::a;

    // `Foo` is NOT fully implicitly constructible in this module.
    const _: Foo = a::Foo { a::private::Bar }; // Compile Error: the module `a::private` is private.

    // Thus, `Foo` is NOT transmutable in this module:
    const _: Foo = transmute!(()); // Compile Error: `Foo` is not constructible here.
}
```

The `transmute!` macro provides a shorthand for safely transmuting a value using its invocation scope as its reference frame:
```rust
pub macro transmute($expr: expr) {
    TransmuteFrom::<_, Here!()>::transmute_from($expr)
    //              ┯  ━━━┯━━━
    //              │     ┕ check constructability from `transmute!`'s invocation scope
    //              ┕ the destination type of the transmute (`_` used to infer the type from context)
}
```
The `Here!()` macro produces a type that uniquely identifies its invocation scope.

This explicit `Scope` parameter of `TransmuteFrom` makes possible the creation of generic abstractions over it. For instance, consider a hypothetical `FromZeros` trait that indicates whether `Self` is safely initializable from a a sufficiently large buffer of zero-initialized bytes:
```rust
pub mod zerocopy {
    pub unsafe trait FromZeros {
        /// Safely initialize `Self` from zeroed bytes.
        fn zeroed() -> Self;
    }

    #[derive(Copy, Clone)]
    #[repr(u8)]
    enum Zero {
        Zero = 0u8
    }

    unsafe impl<Dst> FromZeros<Neglect> for Dst
    where
        Dst: TransmuteFrom<[Zero; usize::MAX], ??? >,
    {
        fn zeroed() -> Self {
            [Zero; size_of::<Self>].transmute_into()
        }
    }
}
```
The above definition leaves ambiguous (`???`) the scope in which the constructability of `Dst` is checked: is it from the perspective of where this trait is defined, or where this trait is *used*? You probably do *not* intend for this trait to *only* be usable with `Dst` types that are defined in the same scope as the `FromZeros` trait!

Adding an explicit `Scope` parameter to `FromZeros` makes this unambiguous; the transmutability of `Dst` should be assessed from where the trait is used, *not* where it is defined:
```rust
pub unsafe trait FromZeros<Scope> {
    /// Safely initialize `Self` from zeroed bytes.
    fn zeroed() -> Self;
}

unsafe impl<Dst, Scope> FromZeros<Scope> for Dst
where
    Dst: TransmuteFrom<[Zero; usize::MAX], Scope>
{
    fn zeroed() -> Self {
        [Zero; size_of::<Self>].transmute_into()
    }
}
```

A thid-party could then use `FromZeros` like so:
```rust
use zerocopy::FromZeros;

#[repr(C)]
struct Foo {
    ...
}

// Initialize `Foo` from zero-initialized bytes.
let _: Foo = FromZeros::<_, Here!()>::zeroed();
```


## Mechanisms of Transmutation

The `TransmuteFrom` trait provides the fundamental mechanism checking the transmutability of types:
```rust
// this trait is implemented automagically by the compiler
#[lang = "transmute_from"]
pub unsafe trait TransmuteFrom<Src: ?Sized, Scope, Neglect = ()>
where
    Neglect: TransmuteOptions,
{
    #[inline(always)]
    fn transmute_from(src: Src) -> Self
    where
        Src: Sized,
        Self: Sized,
        Neglect: SafeTransmuteOptions,
    {
        unsafe { Self::unsafe_transmute_from(src) }
    }

    #[inline(always)]
    unsafe fn unsafe_transmute_from(src: Src) -> Self
    where
        Src: Sized,
        Self: Sized,
        Neglect: TransmuteOptions,
    {
        use core::mem::ManuallyDrop;

        #[repr(C)]
        union Transmute<Src, Dst> {
            src: ManuallyDrop<Src>,
            dst: ManuallyDrop<Dst>,
        }

        unsafe {
            ManuallyDrop::into_inner(Transmute { src: ManuallyDrop::new(src) }.dst)
        }
    }
}
```

In the above definitions, `Src` represents the source type of the transmutation, `Dst` represents the destination type of the transmutation, and `Neglect` is a parameter that [encodes][options] which static checks the compiler ought to neglect when considering if a transmutation is valid. The default value of `Neglect` is `()`, which reflects that, by default, the compiler does not neglect *any* static checks.

### Neglecting Static Checks
[options]: #Neglecting-Static-Checks

The default value of the `Neglect` parameter, `()`, statically forbids transmutes that are ill-defined or unsafe. However, you may explicitly opt-out of some static checks; e.g.:

| Transmute Option    | Usable With                                             |
|---------------------|---------------------------------------------------------|
| `NeglectAlignment`  | `unsafe_transmute_{from,into}`                          |
| `NeglectValidity`   | `unsafe_transmute_{from,into}`                          |

The selection of multiple options is encoded by grouping them as a tuple; e.g., `(NeglectAlignment, NeglectValidity)` is a selection of both the `NeglectAlignment` and `NeglectValidity` options.

We introduce two marker traits which serve to group together the options that may be used with safe transmutes, and those which may be used with `unsafe` transmutes:
```rust
pub trait SafeTransmuteOptions: private::Sealed
{}

pub trait TransmuteOptions: SafeTransmuteOptions
{}

impl SafeTransmuteOptions for () {}
impl TransmuteOptions for () {}
```

#### `NeglectAlignment`
[ext-ref-casting]: #NeglectAlignment

By default, `TransmuteFrom`'s methods require that, when transmuting references, the minimum alignment of the destination's referent type is no greater than the minimum alignment of the source's referent type. The `NeglectAlignment` option disables this requirement.
```rust
pub struct NeglectAlignment;

impl TransmuteOptions for NeglectAlignment {}
```

By using the `NeglectAlignment` option, you are committing to ensure that the transmuted reference satisfies the alignment requirements of the destination's referent type. For instance:
```rust
/// Try to convert a `&T` into `&U`.
///
/// This produces `None` if the referent isn't appropriately
/// aligned, as required by the destination type.
pub fn try_cast_ref<'t, 'u, T, U, Scope>(src: &'t T) -> Option<&'u U>
where
    &'t T: TransmuteFrom<&'u U, Scope, NeglectAlignment>,
{
    if (src as *const T as usize) % align_of::<U>() != 0 {
        None
    } else {
        // Safe because we dynamically enforce the alignment
        // requirement, whose static check we chose to neglect.
        Some(unsafe { TransmuteFrom::unsafe_transmute_from(src) })
    }
}
```

#### `NeglectValidity`
By default, `TransmuteFrom`'s methods require that all instantiations of the source type are guaranteed to be valid instantiations of the destination type. This precludes transmutations which *might* be valid depending on the source value:
```rust
#[repr(u8)]
enum Bool {
    True = 1,
    False = 0,
}

/* ⚠️ This example intentionally does not compile. */
let _ : Bool  = transmute!(some_u8_value); // Compile Error!
```
The `NeglectValidity` option disables this check.
```rust
pub struct NeglectValidity;

impl TransmuteOptions for NeglectValidity {}
```

By using the `NeglectValidity` option, you are committing to ensure dynamically source value is a valid instance of the destination type. For instance:
```rust
#[repr(u8)]
enum Bool {
    True = 1,
    False = 0,
}

pub trait TryIntoBool<Scope>
{
    fn try_into_bool(self) -> Option<Bool>;
}

impl<T, Scope> TryIntoBool<Scope> for T
where
    u8: TransmuteFrom<T, Scope>,
    Bool: TransmuteFrom<u8, Scope, NeglectValidity>
{
    fn try_into_bool(self) -> Option<Bool> {
        let val: u8 = TransmuteFrom::transmute_from(self);

        if val > 1 {
            None
        } else {
            // Safe, because we've first verified that
            // `val` is a bit-valid instance of a boolean.
            Some(unsafe {TransmuteFrom::unsafe_transmute_from(val)})
        }
    }
}
```

Even with `NeglectValidity`, the compiler will statically reject transmutations that cannot possibly be valid:
```rust
#[repr(C)] enum Foo { A = 24 }

#[repr(C)] enum Bar { Z = 42 }

let _ = <Bar as TransmuteFrom<Foo, Here!(), NeglectValidity>::unsafe_transmute_from(Foo::N) // Compile error!
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## Implementation Guidance
Three items in this RFC require special compiler support:
  - `Here!()`
  - `Constructible` (a private implementation detail of `TransmuteFrom`)
  - `TransmuteFrom`

### Implementing `Here!()`
The `Here!` macro produces a type that uniquely identifies its invocation scope. For instance:
```rust
use static_assertions::*;

type A = Here!();
type B = Here!();

/* Types A and !::B are equal */
assert_type_eq_all!(A, B);

trait Foo {
    type C;
    const CONST: ();
}

impl Foo for ! {
    type C = Here!();

    const CONST: () = {
        type D = Here!();

        /* A, B and !::C are equal */
        assert_type_eq_all!(A, B, !::C);

        /* C and D are NOT equal; C and D inhabit different scopes */
        assert_type_ne_all!(C, D);
    };
}
```
Scope types should (as much as possible) pretty print in compiler error messages as their definition path.

These scope types should generated with `pub(self)` visibility. We are not currently aware of any reason why publicly re-exporting a scope type via a type alias would be a good idea; restricting the visibility of these types will warn users against doing so. If compelling use-cases for re-exported scope types are discovered in the future, a broader visibility could be used instead without breaking backwards compatibility.

### Implementing `Constructible`
The compiler implements `Constructible<Scope>` for `T` if `T` is fully implicitly constructible in the scope uniquely identified by the type `Scope`.

A type `T` is fully implicitly constructible in a particular scope if:
  - `T`'s implicit constructor is reachable from the scope, and either:
    - `T` has no fields, or
    - `T`'s fields are fully implicitly constructible from the scope.

The `Constructible` trait does not ever need to be made stable, or even visible (à la the virtual `Freeze` trait). It is merely a useful device for implementing `TransmuteFrom`.

### Implementing `TransmuteFrom`
The implementation of `TransmuteFrom` is completely internal to the compiler (à la [`Sized`](https://doc.rust-lang.org/nightly/nightly-rustc/rustc_middle/ty/struct.TyS.html#method.is_sized) and [`Freeze`](https://doc.rust-lang.org/nightly/nightly-rustc/rustc_middle/ty/struct.TyS.html#method.is_freeze)).

#### Constructability and Transmutation
Unless `NeglectConstructability` is used as `Neglect` option, a `Src` is *safely* transmutable into `Dst` in a given `Scope` if:
  1. `Dst: Constructible<Scope>`
  2. `Dst: TransmuteFrom<Src, Scope, Neglect>`

If `Src` is a mutatable reference, then additionally:
  1. `Src: Constructible<Scope>`

### Implementing `differing_sizes`
The `differing_sizes` lint reports a compiler warning when the source and destination types of a `transmute!()`, `transmute_into` or `transmute_from` invocation differ. This lint shall be warn-by-default.

### Minimal Useful Stabilization Surface
Stabilizing *only* this subset of the Initial Smart Implementation will cover many use-cases:
  - `transmute!()`

To define traits that generically abstract over `TransmuteFrom`, these items must be stabilized:
  - `Here!()`
  - `TransmuteFrom`
  - `TransmuteOptions` and `SafeTransmuteOptions`


### Complete API Surface
[minimal-impl]: #Listing-for-Initial-Minimal-Implementation
This listing is the **canonical specification** of this RFC's API surface ([playground](https://play.rust-lang.org/?version=nightly&mode=debug&edition=2018&gist=0a4dda2760244110bcf829c298c45c34)):
```rust
#![feature(untagged_unions,const_fn,const_fn_union)] // for the impl of unsafe_transmute_from
#![feature(decl_macro)] // for stub implementations of derives
#![allow(warnings)]

/// Transmutation conversions.
// suggested location: `core::convert`
pub mod transmute {

    use options::*;

    /// Safely transmute $expr
    pub macro transmute($expr: expr) {
        TransmuteFrom::<_, Here!()>::transmute_from($expr)
    }

    /// `Self: TransmuteFrom<Src, Neglect`, if the compiler accepts
    /// the safety of transmuting `Src` into `Self`, notwithstanding
    /// a given set of static checks to `Neglect`.
    pub unsafe trait TransmuteFrom<Src: ?Sized, Scope, Neglect = ()>
    where
        Neglect: TransmuteOptions,
    {
        /// Reinterpret the bits of a value of one type as another type, safely.
        #[inline(always)]
        fn transmute_from(src: Src) -> Self
        where
            Src: Sized,
            Self: Sized,
            Neglect: SafeTransmuteOptions,
        {
            unsafe { Self::unsafe_transmute_from(src) }
        }

        /// Reinterpret the bits of a value of one type as another type, potentially unsafely.
        ///
        /// The onus is on you to ensure that calling this function is safe.
        #[inline(always)]
        unsafe fn unsafe_transmute_from(src: Src) -> Self
        where
            Src: Sized,
            Self: Sized,
            Neglect: TransmuteOptions,
        {
            use core::mem::ManuallyDrop;

            #[repr(C)]
            union Transmute<Src, Dst> {
                src: ManuallyDrop<Src>,
                dst: ManuallyDrop<Dst>,
            }
    
            unsafe {
                ManuallyDrop::into_inner(Transmute { src: ManuallyDrop::new(src) }.dst)
            }
        }
    }

    /// A type is always transmutable from itself.
    // This impl will be replaced with a compiler-supported for arbitrary source and destination types.
    unsafe impl<T: ?Sized, Scope, Neglect> TransmuteFrom<T, Scope, Neglect> for T
    where
        Neglect: TransmuteOptions
    {}

    /// Static checks that may be neglected when determining if a type is `TransmuteFrom` some other type.
    pub mod options {

        /// Options that may be used with safe transmutations.
        pub trait SafeTransmuteOptions: TransmuteOptions {}

        /// Options that may be used with unsafe transmutations.
        pub trait TransmuteOptions: private::Sealed {}

        impl SafeTransmuteOptions for () {}
        impl TransmuteOptions for () {}

        /// Neglect the alignment check of `TransmuteFrom`.
        pub struct NeglectAlignment;

        /// Neglect the validity check of `TransmuteFrom`.
        pub struct NeglectValidity;

        /* additional options */

        impl TransmuteOptions for NeglectAlignment {}
        impl TransmuteOptions for NeglectValidity {}

        impl TransmuteOptions for (NeglectAlignment, NeglectValidity) {}

        // prevent third-party implementations of `TransmuteOptions`
        mod private {
            use super::*;

            pub trait Sealed {}

            impl Sealed for () {}
            impl Sealed for NeglectAlignment {}
            impl Sealed for NeglectValidity {}
            impl Sealed for (NeglectAlignment, NeglectValidity) {}
        }
    }
}
```


# Drawbacks
[drawbacks]: #drawbacks

TODO


# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

## Rationale: `TransmuteFrom`

### Why support arbitrary transmutation?
Some [prior art][prior-art], especially in the crate ecosystem, provides an API that [only supports transmutations involving particular types](#Source-and-Destination-Types-Supported) (e.g., from/into bytes). As we discuss in the [prior art][prior-art] section, we believe that the inflexibility of such approaches make them a poor basis of a language proposal. In particular, these restrictive approaches don't leave room to grow: supporting additional transmutations requires additional traits.

The API advocated by this proposal is unopinionated about what transmutations users might wish to do, and what transmutations the compiler is able to reason about. The implementation of this RFC may be initially very simple (and perhaps support no more than the restrictive approaches allow for), but then subsequently grow in sophistication—*without* necessitating public API changes.

## Alternative: Implementing this RFC in a Crate

This RFC builds on ample [prior art][prior-art] in the crate ecosystem, but these efforts strain against the fundamental limitations of crates. Fundamentally, safe transmutation efforts use traits to expose layout information to the type system. The burden of ensuring safety [is usually either placed entirely on the end-user, or assumed by complex, incomplete proc-macro `derives`][mechanism-manual].

An exception to this rule is the [typic][crate-typic] crate, which utilizes complex, type-level programming to emulate a compiler-supported, "smart" `TransmuteFrom` trait (like the one proposed in this RFC). Nonetheless, [typic][crate-typic] is fundamentally limited: since Rust does not provide a type-level mechanism for reflecting over the structure of arbitrary types, even [typic][crate-typic] cannot judge the safety of a transmutation without special user-added annotations on type definitions. Although [typic][crate-typic] succeeds as a proof-of-concept, its maintainability is questionable, and the error messages it produces are [lovecraftian](https://en.wikipedia.org/wiki/Lovecraftian_horror).

The development approaches like [typic][crate-typic]'s could, perhaps, be eased by stabilizing [frunk](https://crates.io/crates/frunk)-like structural reflection, or (better yet) by stabilizing a compiler plugin API for registering "smart" traits like `TransmuteFrom`. However, we suspect that such features would be drastically harder to design and stabilize. 

Regardless of approach, almost all [prior art][prior-art] attempts to reproduce knowledge *already* possessed by `rustc` during the compilation process (i.e., the layout qualities of a concrete type). Emulating the process of layout computation to any degree is an error-prone duplication of effort between `rustc` and the crate, in a domain where correctness is crucial.

Finally, community-led, crate-based approaches are, inescapably, unauthoritative. These approaches are incapable of fulfilling our motivating goal of providing a *standard* mechanism for programmers to statically ensure that a transmutation is well-defined or safe.

# Prior art
[prior-art]: #prior-art

[crate-plain]: https://crates.io/crates/plain
[crate-bytemuck]: https://crates.io/crates/bytemuck
[crate-dataview]: https://crates.io/crates/dataview
[crate-safe-transmute]: https://crates.io/crates/safe-transmute
[crate-pod]: https://crates.io/crates/pod
[crate-uncon]: https://crates.io/crates/uncon
[crate-typic]: https://crates.io/crates/typic
[crate-zerocopy]: https://crates.io/crates/zerocopy
[crate-convute]: https://crates.io/crates/convute
[crate-byterepr]: https://crates.io/crates/byterepr

[2017-02]: https://internals.rust-lang.org/t/pre-rfc-safe-coercions/4823
[2018-03]: https://internals.rust-lang.org/t/pre-rfc-frombits-intobits/7071
[2018-03-18]: https://internals.rust-lang.org/t/pre-rfc-frombits-intobits/7071/23
[2018-05-18]: https://internals.rust-lang.org/t/pre-rfc-trait-for-deserializing-untrusted-input/7519
[2018-05-23]: https://github.com/joshlf/rfcs/blob/joshlf/from-bytes/text/0000-from-bytes.md
[2019-09]: https://internals.rust-lang.org/t/specifying-a-set-of-transmutes-from-struct-t-to-struct-u-which-are-not-ub/10917
[2019-11]: https://internals.rust-lang.org/t/pre-rfc-safe-transmute/11347
[2019-12-05-gnzlbg]: https://gist.github.com/gnzlbg/4ee5a49cc3053d8d20fddb04bc546000
[2019-12-05-v2]: https://internals.rust-lang.org/t/pre-rfc-v2-safe-transmute/11431
[2020-07]: https://internals.rust-lang.org/t/pre-rfc-explicit-opt-in-oibit-for-truly-pod-data-and-safe-transmutes/2361

## Prior Art: Rust
[prior-art-rust]: #prior-art-rust

A handful of dimensions of variation characterize the distinctions between prior art in Rust:
  - conversion complexity
  - conversion fallibility
  - source and destination types supported
  - implementation mechanism
  - stability hazards

We review each of these dimensions in turn, along with this proposal's location along these dimensions:

### Conversion Complexity
Prior work differs in whether it supports complex conversions, or only simple transmutation. [*Pre-RFC FromBits/IntoBits*][2018-03]'s proposed traits include conversion methods that are implemented by type authors. Because end-users provide their own definitions of these methods, they can be defined to do more than just transmutation (e.g., slice casting). (This approach is similar to the [uncon][crate-uncon] crate's [`FromUnchecked`](https://docs.rs/uncon/1.*/uncon/trait.FromUnchecked.html) and [`IntoUnchecked`](https://docs.rs/uncon/1.*/uncon/trait.IntoUnchecked.html) traits, which provide unsafe conversions between types. These traits are safe to implement, but their conversion methods are not.)

In contrast, our RFC focuses only on transmutation. Our `TransmutableFrom` and `TransmutableInto` traits serve as both a marker *and* a mechanism: if `Dst: TransmuteFrom<Src>`, it is safe to transmute from `Dst` into `Src` using `mem::transmute`. However, these traits *also* provide transmutation methods that are guaranteed to compile into nothing more complex than a `memcpy`. These methods cannot be overridden by end-users to implement more complex behavior.

The signal and transmutability and mechanism are, in principle, separable. The [convute][crate-convute] crate's [`Transmute<T>`](https://docs.rs/convute/0.2.0/convute/marker/trait.Transmute.html) trait is an unsafe marker trait representing types that can be transmuted into `T`. This is *just* a marker trait; the actual conversion mechanisms are provided by a [separate suite](https://docs.rs/convute/0.2.0/convute/convert/index.html) of traits and functions. Our RFC combines marker with mechanism because we feel that separating these aspects introduces additional complexity with little added value. 

### Conversion Fallibility
Prior work differs in whether it supports only infallible conversions, or fallible conversions, too. The [convute][crate-convute] crate's [`TryTransmute<T>`](https://docs.rs/convute/0.2.0/convute/marker/trait.TryTransmute.html) trait provides a method, `can_transmute`, that returns true a transmutation from `Self` to `T` is valid for a particular value of `&self`. An early version of [typic][crate-typic] abstracted a similar mechanism into an [`Invariants`](https://docs.rs/typic/0.1.0/typic/transmute/trait.Invariants.html) trait, with additional facilities for error reporting. [*Draft-RFC: `Compatible`/`TryCompatible`*][2019-12-05-gnzlbg] employs a similar mechanism to typic.

Typic removed support for fallible transmutation after reckoning with several challenges:
- The causes of uncertain failure could be language-imposed (e.g., alignment or validity requirements), or library imposed (i.e., invariants placed on a structure's private fields).
- The points of uncertain failures could be arbitrarily 'deep' into the fields of a type.
- Error reporting incurs a runtime cost commensurate with the detail of the reporting, but the detail of reporting required by end-user depends on use-case, not just type. For instance: for some use-cases it may be necessary to know where and why a byte was not a valid `bool`; in others it may be sufficient to know simply *whether* an error occurred.

Finally, we observed that the mechanisms of fallible transmutation were basically separable from the mechanisms of infallible transmutation, and thus these challenges could be addressed at a later date. For these reasons, our RFC *only* addresses infallible transmutation.

While this RFC does not provide a grand, all-encompassing mechanism for fallible transmutation, the fundamental mechanisms of our RFC are useful for constructing safer, purpose-built fallible conversion mechanisms; e.g.:
```rust
/// Try to convert a `&T` into `&U`.
///
/// This produces `None` if the referent isn't appropriately
/// aligned, as required by the destination type.
pub fn try_cast_ref<'t, 'u, T, U, Scope>(src: &'t T) -> Option<&'u U>
where
    &'t T: TransmuteFrom<&'u U, Scope, NeglectAlignment>,
{
    if (src as *const T as usize) % align_of::<U>() != 0 {
        None
    } else {
        // Safe because we dynamically enforce the alignment
        // requirement, whose static check we chose to neglect.
        Some(unsafe { TransmuteFrom::unsafe_transmute_from(src) })
    }
}
```
In this approach, our RFC is joined by crates such as [plain](https://docs.rs/plain/0.2.3/plain/#functions), [bytemuck](https://docs.rs/bytemuck/1.*/bytemuck/#functions), [dataview](https://docs.rs/dataview/0.1.1/dataview/struct.DataView.html#methods), [safe-transmute](https://docs.rs/safe-transmute/0.11.0/safe_transmute/fn.transmute_one.html), [zerocopy](https://docs.rs/zerocopy/0.3.0/zerocopy/struct.LayoutVerified.html#methods), and [byterepr](https://docs.rs/byterepr/0.1.0/byterepr/trait.ByteRepr.html#provided-methods), and several pre-RFCs (such as [this][2018-05-18] and [this](https://github.com/joshlf/rfcs/blob/joshlf/from-bits/text/0000-from-bits.md#library-functions)). The ubiquity of these mechanisms makes a strong case for their inclusion in libcore.

### Source and Destination Types Supported
Prior work differs in whether its API surface is flexible enough to support transmutation between arbitrary types, or something less.

#### Arbitrary Types
Approaches supporting transmutations between arbitrary types invariably define traits akin to either or both: 
```rust
/// Indicates that `Self` may be transmuted into `Dst`.
pub unsafe trait TransmuteInto<Dst>
{ ... }

/// Indicates that `Self` may be transmuted from `Dst`.
pub unsafe trait TransmuteFrom<Src>
{ ... }
```
This approach, taken by our RFC, is used by at least two crates:
- The [convute][crate-convute] crate's [`Transmute<T>`](https://docs.rs/convute/0.2.0/convute/marker/trait.Transmute.html) trait is akin to the above definition of `TransmuteInto`.
- The [typic][crate-typic] crate's [`TransmuteInto`](https://docs.rs/typic/0.3.0/typic/transmute/trait.TransmuteInto.html) and [`TransmuteFrom`](https://docs.rs/typic/0.3.0/typic/transmute/trait.TransmuteFrom.html) traits almost exactly mirror the above definitions.

...and several proposals:
- [*Pre-RFC: Safe coercions*][2017-02] proposes a `Coercible<A, B>` trait that is implemented if `A` is safely transmutable into `B`.
- [*Pre-RFC: `FromBits`/`IntoBits`*][2018-03] proposes the traits `IntoBits<U>` and `FromBits<T>.`
- [*Draft-RFC: `FromBytes`*][2018-05-23] proposes the traits `IntoBytes<U>` and `FromBytes<T>.`
- [*Draft-RFC: `Compatible`/`TryCompatible`*][2019-12-05-gnzlbg] proposes the trait `Compatible<U>`, akin to the above definition of `TransmuteInto`.

##### From/Into Bytes Transmutations
Other approaches adopt an API that only supports transmutation of a type into initialized bytes, and from initialized bytes. These approaches invariably define traits akin to:
```rust
/// Indicates that a type may be transmuted into an appropriately-sized array of bytes.
pub unsafe trait IntoBytes
{}

/// Indicates that a type may be transmuted from an appropriately-sized array of bytes.
pub unsafe trait FromBytes
{}
```
This is the approach taken by the [zerocopy][crate-zerocopy] crate, and the [*Pre-RFC: Safe Transmute*][2019-11] and [*Pre-RFC: Safe Transmute v2*][2019-12-05-v2] proposals.

This approach is strictly less flexible than an API supporting transmutation between arbitrary types. It is incapable of representing transmutations of bytes into types with validity constraints, and incapable of representing transmutations of types with padding bytes into bytes.

Supporting additional transmutation source and destination types requires a commensurate addition of conversion traits. For instance, some of [zerocopy][crate-zerocopy]'s users [require](https://fuchsia-review.googlesource.com/c/fuchsia/+/306036/2#message-a1a0c9cf16e3dec24e7b0548e3c09382f63783f0) a trait that reflects types which can be transmuted from a buffer of zeroed bytes. This would require introducing an additional trait, `FromZeros`.

An advantage of this API is that it gives descriptive names to perhaps the two most common transmutations. However, an API providing transmutation between arbitrary types can encode `FromBytes` and `IntoBytes`:
```rust
// `Dst` is `FromBytes` if it can be safely transmuted *from* an
// equivalently sized array of `u8`.
unsafe impl<Dst> FromBytes for Dst
where
    Dst: TransmuteFrom<[u8; size_of::<Dst>()]>,
{}

// `Src` is `IntoBytes` if it can be safely transmuted *into* an
// equivalently sized array of `u8`.
unsafe impl<Src> IntoBytes for Src
where
    Src: TransmuteInto<[u8; size_of::<Src>()]>,
{}
```
For these reasons, we argue that a `FromBytes`/`ToBytes` style API is a poor foundation for in-language safe transmutation.

##### Bytes-to-Bytes Transmutations (aka "Plain Old Data")
Finally, many approaches (especially crates) supply a marker trait that represents "plain old data"; e.g.:
```rust
/// Implemented by types that are "plain old data":
pub unsafe trait PlainOldData
{}
```
This sort of trait is present in crates such as [plain](https://docs.rs/plain/0.2.3/plain/trait.Plain.html), [bytemuck](https://docs.rs/bytemuck/1.*/bytemuck/trait.Pod.html), [dataview](https://docs.rs/dataview/0.1.1/dataview/trait.Pod.html), [safe-transmute](https://docs.rs/safe-transmute/0.11.0/safe_transmute/trivial/trait.TriviallyTransmutable.html), and [pod](https://docs.rs/pod/0.5.0/pod/trait.Pod.html), and at least two language proposals ([here][2018-05-18] and [here][2020-07]).

The exact definition of what constitutes "plain old data" varies between crates. One simple definition is that a type `T` is "plain old data" if it can be transmuted both from and into initialized bytes; i.e.:
```rust
unsafe impl<T> PlainOldData for T
where
    T: FromBytes + IntoBytes,
{}
```

This definition precludes useful transmutations. For instance, `MaybeUninit<u8>` is transmutable from a `u8`, but not *into* a `u8`.

Given this inflexibility, we argue that this approach is a poor foundation for in-language safe transmutation.


### Implementation Mechanism
Not only does prior work differ in which traits are used to encode valid transmutations, they differ in the level of user intervention required to take advantage of the traits. 

#### Manual
[mechanism-manual]: #Manual
Fully manual approaches require type authors to implement the transmutation traits manually. The involved traits are `unsafe`, so it is up to type authors to verify for themselves that their hand-written implementations are sound. This is the approach taken by crates such as [plain][crate-plain], [bytemuck][crate-bytemuck], [safe-transmute][crate-safe-transmute], and [pod][crate-pod], and at least one language proposal: [*Pre-RFC: Safe Transmute*][2019-12-05-v2] (which advocates for a "plain old data" API).

In semi-manual approaches, type authors simply `derive` the applicable traits, using `derive` macros that produce a compile-error if the implementation is not sound. This approach is realized by crates such as ([zerocopy](https://docs.rs/zerocopy/0.3.0/zerocopy/#derives), [zeroable](https://docs.rs/zeroable/0.2.0/zeroable/) and [dataview](https://docs.rs/dataview/0.1.1/dataview/derive.Pod.html)) and advocated by at least two language proposals: [*Pre-RFC: Safe Transmute v2*][2019-12-05-v2] (which advocates for a `FromBytes`/`IntoBytes`-style API), and [*Pre-RFC FromBits/IntoBits*][2018-03] (which advocates for a general-transmutation API).

We believe that the implementation burden these approaches place on end-users, and their inflexibility, make them a poor foundation for in-language safe transmutation:
- These approaches require authors to implement and, potentially, verify a large number of `unsafe` traits, ranging from *O(n)* implementations for plain-old-data trait approaches, to potentially [*many* more](https://internals.rust-lang.org/t/pre-rfc-frombits-intobits/7071/28).
- These approaches are generally impractical for APIs that permit truly general transmutation, as type authors can only construct implementations of the transmutation traits for types they have at their disposal.
- These approaches conflate transmutation stability with transmutation safety. An end-user wishing to transmute a type for which its author has *not* manually implemented the applicable traits must resort to the wildly unsafe `mem::transmute`.


#### Automatic
Automatic approaches implement the transmutation traits without user intervention, whenever it is sound to do so. This is the approach taken by our RFC. Automatic mechanisms appear in at least four prior language proposals:
- [*Pre-RFC: Safe coercions*][2017-02]
- [*Draft-RFC: `from_bytes`*][2018-05-23]
- [*Pre-RFC: Trait for deserializing untrusted input*][2018-05-18]
- [*Draft-RFC: `compatible_trait`*][2019-12-05-gnzlbg]

The [typic][crate-typic] crate mocks a fully-automatic approach: its `TransmuteFrom` trait is usable with any types that are `repr(C)`, or otherwise have a well-defined memory layout. (In practice, since Rust lacks reflection over type definitions, `repr(C)` annotations much be changed to `typic::repr(C)`.)

### Safety Hazards
Fully automatic approaches introduce, at the very least, a safety hazard: they supply a safe constructor for types, without the consent of those types' authors. If a type author hid the internals of their type because they do not wish for its implementation details to become a part of the type's API for SemVer for safety purposes, an automatic transmutation mechanism subverts that intent.

No attempt to avoid this hazard is made by most of the proposals featuring automatic mechanisms; e.g.:
- [*Draft-RFC: `from_bytes`*][2018-05-23]
- [*Pre-RFC: Trait for deserializing untrusted input*][2018-05-18]
- [*Draft-RFC: `compatible_trait`*][2019-12-05-gnzlbg]

#### Hazard-Avoidant
The automatic mechanism proposed by [*Pre-RFC: Safe coercions*][2017-02] exploits field visibility, requiring that all fields that have different types in `Src` and `Dst` are visible at the location where the coercion is made. This approach falls short in three respects:
1. Confining the visibility requirement only to fields of *different* types is insufficient; two different types with identical field types may subject those fields to different invariants. 
2. The 'location' where the coercion is made is ill-defined; the presence of the proposed `Coercible` trait may be far-removed from the location of the actual conversion (if any conversion occurs at all).
3. Field visibility stabilizes the structure of a type, but *not* its layout (e.e., its size).

Our RFC, [typic][crate-typic], and Haskell exploit the related concept of *constructability*. Typic uses a simplified, scope-unaware formulation of constructability that suffers from a soundness hole induced by the pub-in-priv trick.


## Prior Art: Haskell

Haskell's [`Coercible`](https://hackage.haskell.org/package/base-4.14.0.0/docs/Data-Coerce.html#t:Coercible) typeclass is implemented for all types `A` and `B` when the compiler can infer that they have the same representation. As with our proposal's `TransmuteFrom` trait, instances of this typeclass are created "on-the-fly" by the compiler. `Coercible` primarily provides a safe means to convert to-and-from newtypes, and does not seek to answer, for instance, if two `u8`s are interchangeable with a `u16`.

Haskell takes an algebraic approach to this problem, reasoning at the level of type definitions, not type layouts. However, not all type parameters have an impact on1 a type's layout; for instance:
```rust
#[repr(C)]
struct Bar<U>(PhantomData<U>);

#[repr(transparent)]
struct Foo<T, U>(T, Bar<U>);
```
`Foo`'s layout is impacted solely by `T`, not `U`, but this isn't necessarily clear by looking at the definition of `Foo`. To reason about these scenarios, Haskell introduces the concept of type parameter [*roles*](https://gitlab.haskell.org/ghc/ghc/-/wikis/roles)—labels that denote the relationship of a type parameter to coercibility.

Our RFC does not need the concept of roles, because it does not attempt to abstractly reason about type definitions. Rather, it reasons about type *layouts*. This example, for instance, does not pose a challenge to our proposal:
```rust
trait SomeTrait { type AssociatedType; }

#[repr(C)]
struct MyStruct<T: SomeTrait>(pub T, pub T::AssociatedType);
``` 
For a *particular* `T`, `MyStruct<T>` will have a *particular* layout. Our proposed `TransmuteFrom` trait reasons about the 
*layouts* of types (which are fully concrete), not the *definitions* (which may be somewhat abstract).


# Unresolved questions
[unresolved-questions]: #unresolved-questions

### Questions To Be Resolved Before RFC Acceptance
The following unresolved questions should be resolved during the RFC process:

##### Unhandled Use-Cases?
We endeavored to design an API surface with ([nearly][drawbacks]) zero compromises. However, if you have a use-case that you believe is neither satisfied outright by our proposal, nor [aided][future-possibilities] by our proposal, we would *urgently* like to hear of it.

##### Extensions for Inclusion?
In [*Future Possibilities*][future-possibilities], we propose a number of additional abstractions that are aided by this RFC. Some of these abstractions are commonplace in [prior art][prior-art] and should perhaps be included with this RFC. Some of our proposed extensions could begin their crates that work on stable Rust; others, such as [generic atomics][future-possibility-generic-atomics], require nightly-only intrinsics.

### Questions To Be Resolved Before Feature Stabilization
The following unresolved questions should be resolved before feature stabilization:

### Questions Out of Scope
We consider the following unresolved questions to be out-of-scope of *this* RFC process:

# Future possibilities
[future-possibilities]: #future-possibilities

## Extension: Layout Property Traits
[0000-ext-layout-traits.md]: https://github.com/rust-lang/project-safe-transmute/blob/master/rfcs/0000-ext-layout-traits.md

See [here][0000-ext-layout-traits.md].

## Extension: Byte Transmutation Traits and Safe Initialization
[extension-zerocopy]: #extension-byte-transmutation-traits-and-safe-initialization
[0000-ext-byte-transmutation.md]: https://github.com/rust-lang/project-safe-transmute/blob/master/rfcs/0000-ext-byte-transmutation.md

See [here][0000-ext-byte-transmutation.md].


## Extension: Slice and `Vec` Casting
[ext-slice-casting]: #extension-slice-and-vec-casting
[ext-vec-casting]: #extension-slice-and-vec-casting
[0000-ext-container-casting.md]: https://github.com/rust-lang/project-safe-transmute/blob/master/rfcs/0000-ext-container-casting.md

See [here][0000-ext-container-casting.md].


## Extension: `include_data!`
[future-possibility-include_data]: #Extension-include_data
[0000-ext-include-data.md]: https://github.com/rust-lang/project-safe-transmute/blob/master/rfcs/0000-ext-include-data.md

See [here][0000-ext-include-data.md].


## Extension: Generic Atomics
[future-possibility-generic-atomics]: #extension-generic-atomics
[0000-ext-generic-atomic.md]: https://github.com/rust-lang/project-safe-transmute/blob/master/rfcs/0000-ext-generic-atomic.md

See [here][0000-ext-generic-atomic.md].
