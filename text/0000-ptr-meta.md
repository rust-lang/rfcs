- Feature Name: `vtable`
- Start Date: 2018-10-24
- RFC PR:
- Rust Issue:

# Summary
[summary]: #summary

Add a `DynTrait` trait and a `VTable` type to provide
access to the components of a fat pointer to a trait object `dyn SomeTrait`,
and the ability to reconstruct such a pointer from its components.


# Background
[background]: #background

## Dynamically-sized types (DSTs)

Any Rust type `T` is either statically-sized or dynamically-sized.

All values of a statically-sized types occupy the same size in memory.
This size is known at compile-time.
Raw pointers or references `*const T`, `*mut T`, `&T`, `&mut T` to such types
are represented in memory as a single pointer.

Different values of a same DST (dynamically-sized type) can have a different size
which can be queried at runtime.
Raw pointers and references to DSTs are “fat” pointers made of one data pointer to the value
together with some metadata.
In Rust 1.30, a DST is always one of:

* A struct whose last field is a DST (and is the only DST field),
  where the metadata is the same as for a reference to that field.
* A slice type like `[U]`, where the metadata is the length in items,
* The string slice type `str`, where the metadata is the length in bytes,
* Or a trait object like `dyn SomeTrait`,
  where the metadata is a pointer to a vtable (virtutal call table).
  A vtable contains the size and required alignment of the concrete type,
  a function pointer to the destructor if any,
  and pointers for the implementations of the trait’s methods.

## Fat pointer components

Typical high-level code doesn’t need to worry about this,
a reference `&Foo` “just works” wether or not `Foo` is a DST.
But unsafe code such as a custom collection library may want to access a fat pointer’s
components separately.

In Rust 1.11 we *removed* a [`std::raw::Repr`] trait and a [`std::raw::Slice`] type
from the standard library.
`Slice` could be `transmute`d to a `&[U]` or `&mut [U]` reference to a slice
as it was guaranteed to have the same memory layout.
This was replaced with more specific and less wildly unsafe
`std::slice::from_raw_parts` and `std::slice::from_raw_parts_mut` functions,
together with `as_ptr` and `len` methods that extract each fat pointer component separatly.

The `str` type is taken care of by APIs to convert to and from `[u8]`.

This leaves trait objects, where we still have an unstable `std::raw::TraitObjet` type
that can only be used with `transmute`:

```rust
#[repr(C)]
pub struct TraitObject {
    pub data: *mut (),
    pub vtable: *mut (),
}
```

[`std::raw::Repr`]: https://doc.rust-lang.org/1.10.0/std/raw/trait.Repr.html
[`std::raw::Slice`]: https://doc.rust-lang.org/1.10.0/std/raw/struct.Slice.html
[`std::raw::TraitObjet`]: https://doc.rust-lang.org/1.30.0/std/raw/struct.TraitObject.html

## Trait objects of multiple traits

A trait object type can refer to multiple traits like `dyn A + B`, `dyn A + B + C`, etc.
Currently all traits after the first must be [auto traits].
Since auto traits cannot have methods, they don’t require additional data in the vtable.

Lifting this restriction is desirable, and has been discussed in
[RFC issue #2035][2035] and [Internals thread #6617][6617].
Two related open questions with this is how to represent the vtable of such a trait object,
and how to support upcasting:
converting for example from `Box<dyn A + B + C>` to `Box<dyn C + A>`.

One possibility is having “super-fat” pointers
whose metadata is made of multiple pointers to separate vtables.
However making the size of pointers grow linearly with the number of traits involved
is a serious downside.

[auto traits]: https://doc.rust-lang.org/1.30.0/reference/special-types-and-traits.html#auto-traits
[2035]: https://github.com/rust-lang/rfcs/issues/2035
[6617]: https://internals.rust-lang.org/t/wheres-the-catch-with-box-read-write/6617


# Motivation
[motivation]: #motivation

We now have APIs in Stable Rust to let unsafe code freely and reliably manipulate slices,
accessing the separate components of a fat pointers and then re-assembling them.
However `std::raw::TraitObject` is still unstable,
but it’s probably not the style of API that we’ll want to stabilize
as at encourages dangerous `transmute` calls.
This is a “hole” in available APIs to manipulate existing Rust types.

For example [this library][lib] stores multiple trait objects of varying size
in contiguous memory together with their vtable pointers,
and during iteration recreates fat pointers from separate data and vtable pointers.

[lib]: https://play.rust-lang.org/?version=nightly&mode=debug&edition=2015&gist=bbeecccc025f5a7a0ad06086678e13f3


# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

For low-level manipulation of trait objects in unsafe code,
the `DynTrait` trait allows accessing the components of a fat pointer:

```rust
use std::dyn_trait::{DynTrait, VTable};

fn callback_into_raw_parts(f: Box<Fn()>) -> (*const (), &'static VTable) {
    let raw = Box::into_raw(f);
    (DynTrait::data_ptr(raw), DynTrait::vtable(raw))
}
```

… and assembling it back again:

```rust
fn callback_from_raw_parts(data: *const (), vtable: &'static VTable) -> Box<Fn()> {
    let raw: *const Fn() = DynTrait::from_raw_parts(new_data, vtable);
    Box::from_raw(raw as *mut Fn())
}
```

`VTable` also provides enough information to manage memory allocation:

```rust
// Equivalent to just letting `Box<Fn()>` go out of scope to have its destructor run,
// this only demonstrates some APIs.
fn drop_callback(f: Box<Fn()>) {
    let raw = Box::into_raw(f);
    let vtable = DynTrait::vtable(raw);
    unsafe {
        // `DynTrait::data` is equivalent to casting to a thin pointer with `as`
        vtable.drop_in_place(raw as *mut ());
        std::alloc::dealloc(raw as *mut u8, vtable.layout());
    }
}
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

A new `core::dyn_trait` module is added and re-exported as `std::dyn_trait`.
Its contains a `DynTrait` trait and `VTable` pointer whose definitions are given below.

`DynTrait` is automatically implement by the language / compiler
(similar to `std::marker::Unsize`, with explicit impls causing a E0328 error)
for all existing trait object types (DSTs)
like `dyn SomeTrait`, `dyn SomeTrait + SomeAutoTrait`, etc.

If in the future “super-fat” pointers (with multiple separate vtable pointers as DST metadata)
are added to the language to refer to trait objects with multiple non-auto traits,
`DynTrait` will **not** be implemented for such trait object types.

`std::raw::TraitObject` and `std::raw` are deprecated and eventually removed.

```rust
/// A trait implemented by any type that is a trait object with a single vtable.
///
/// This allows generic code to constrain itself to trait-objects, and subsequently
/// enables code to access the vtable directly and store trait-object values inline.
///
/// For instance `dyn Display` implements `Trait`.
#[lang = "dyn_trait"]
pub trait DynTrait: ?Sized {
    /// Extracts the data pointer from a trait object.
    fn data_ptr(obj: *const Self) -> *const () { obj as _ }

    /// Extracts the vtable pointer from a trait object.
    fn vtable(obj: *const Self) -> &'static VTable;

    /// Creates a trait object from its data and vtable pointers.
    ///
    /// Undefined Behaviour will almost certainly result if the data pointed
    /// to isn’t a valid instance of the type the vtable is associated with.
    unsafe fn from_raw_parts(data: *const (), vtable: &'static VTable) -> *const Self;
}

/// The vtable for a trait object.
///
/// A vtable (virtual call table) represents all the necessary information
/// to manipulate the concrete type stored inside a trait object.
/// It notably it contains:
///
/// * type size
/// * type alignment
/// * a pointer to the type’s `drop_in_place` impl (may be a no-op for plain-old-data)
/// * pointers to all the methods for the type’s implementation of the trait
///
/// Note that the first three are special because they’re necessary to allocate, drop,
/// and deallocate any trait object.
///
/// The layout of vtables is still unspecified, so this type is one a more-type-safe
/// convenience for accessing those 3 special values. Note however that `VTable` does
/// not actually know the trait it’s associated with, indicating that, at very least,
/// the location of `size`, `align`, and `drop_in_place` is identical for all
/// trait object vtables in a single program.
pub struct VTable {
    _priv: (),
}

impl VTable {
    /// Returns the size of the type associated with this vtable.
    pub fn size(&self) -> usize { ... }

    /// Returns the alignment of the type associated with this vtable.
    pub fn align(&self) -> usize { ... }

    /// Returns the size and alignment together as a `Layout`
    pub fn layout(&self) -> alloc::Layout {
        unsafe {
            alloc::Layout::from_size_align_unchecked(self.size(), self.align())
        }
    }

    /// Drops the value pointed at by `data` assuming it has the type
    /// associated with this vtable.
    ///
    /// Behaviour is Undefined if the type isn’t correct or the pointer
    /// is invalid to reinterpret as `&mut TheType`.
    pub unsafe fn drop_in_place(&self, data: *mut ()) { ... }
}
```

# Drawbacks
[drawbacks]: #drawbacks

If super-fat pointers are ever added to the language, this API will not work with them
and another, somewhat-redundant API will be needed.

The RFC as proposed also prevents us from ever changing
trait objects of a trait that has multiple super-traits
to using super-fat pointers.

```rust
trait A {}
trait B {}
trait C {}
trait Foo: A + B + C {}
let pointer_size = std::mem::size_of::<*const ()>();
// This holds as of Rust 1.30:
assert_eq!(std::mem::size_of::<&dyn Foo>(), 2 * pointer_size);
```

The author opinion is that the size cost for pointers (which can be copied or moved around a lot)
makes super-fat pointers prohibitive and that a different solution would be preferable,
regardless of this RFC.


# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

The status quo is that code (such as linked in [Motivation]) that requires this functionality
needs to transmute to and from `std::raw::TraitObject`
or a copy of it (to be compatible with Stable Rust).
Additionally, in cases where constructing the data pointer
requires knowing the alignment of the concrete type,
a dangling pointer such as `0x8000_0000_usize as *mut ()` needs to be created.
It is not clear whether `std::mem::align_of(&*ptr)` with `ptr: *const dyn SomeTrait`
is Undefined Behavior with a dangling data pointer.

Support for [Custom DSTs] would include functionality equivalent to the `DynTrait` trait.
But it has been postponed, apparently indefinitely.
It is a much more general feature that involves significant design and implementation work.

An itermediate solution might be to generalize `DynTrait` to a trait implemented for *all* types,
with an associated type for the metadata in pointers and references.
(This metadata would be `()` for thin pointers.)
This trait’s methods would be the same as in `DynTrait`,
with `Self::Metadata` instead of `&'static VTable`.
At first this trait would only be implemented automatically,
so libraries would not be able to create new kinds of DSTs,
but this design might be extensible in that direction later.

To allow for the possiblity of
changing trait objects with multiple super-traits to use super-fat pointers later,
we could make such trait objects not implement `DynTrait` at first.
Specifically: among all the traits and recursive super-traits involved in a trait object,
only implement `DynTrait` if each trait has at most one non-auto (or non-[marker]) super-trait.

[Custom DSTs]: https://internals.rust-lang.org/t/custom-dst-discussion/4842
[marker]: https://github.com/rust-lang/rust/issues/29864


# Unresolved questions
[unresolved-questions]: #unresolved-questions

* Is a new `dyn_trait` module inside `core` and `std` the appropriate location?
  `trait` would be nice, but that’s a reserved keyword.
  `traits` would work but there is not much precedent for pluralizing module names in this way.
  (For example `std::slice`, `std::string`, …)

* Every item name is of course also subject to the usual bikeshed.

* `*const ()` v.s. `*mut ()`?