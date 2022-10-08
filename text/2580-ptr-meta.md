- Feature Name: `ptr-meta`
- Start Date: 2018-10-26
- RFC PR: https://github.com/rust-lang/rfcs/pull/2580
- Rust Issue: https://github.com/rust-lang/rust/issues/81513

# Summary
[summary]: #summary

Add generic APIs that allow manipulating the metadata of fat pointers:

* Naming the metadata’s type  (as an associated type)
* Extracting metadata from a pointer
* Reconstructing a pointer from a data pointer and metadata
* Representing vtables, the metadata for trait objects, as a type with some limited API

This RFC does *not* propose a mechanism for defining custom dynamically-sized types,
but tries to stay compatible with future proposals that do.


# Background
[background]: #background

Typical high-level code doesn’t need to worry about fat pointers,
a reference `&Foo` “just works” whether or not `Foo` is a DST.
But unsafe code such as a custom collection library may want to access a fat pointer’s
components separately.

In Rust 1.11 we *removed* a [`std::raw::Repr`] trait and a [`std::raw::Slice`] type
from the standard library.
`Slice` could be `transmute`d to a `&[U]` or `&mut [U]` reference to a slice
as it was guaranteed to have the same memory layout.
This was replaced with more specific and less wildly unsafe
`std::slice::from_raw_parts` and `std::slice::from_raw_parts_mut` functions,
together with `as_ptr` and `len` methods that extract each fat pointer component separately.

For trait objects, where we still have an unstable `std::raw::TraitObject` type
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
[`std::raw::TraitObject`]: https://doc.rust-lang.org/1.30.0/std/raw/struct.TraitObject.html


# Motivation
[motivation]: #motivation

We now have APIs in Stable Rust to let unsafe code freely and reliably manipulate slices,
accessing the separate components of a fat pointers and then re-assembling them.
However `std::raw::TraitObject` is still unstable,
but it’s probably not the style of API that we’ll want to stabilize
as it encourages dangerous `transmute` calls.
This is a “hole” in available APIs to manipulate existing Rust types.

For example [this library][lib] stores multiple trait objects of varying size
in contiguous memory together with their vtable pointers,
and during iteration recreates fat pointers from separate data and vtable pointers.

The new `Thin` trait alias also expanding to [extern types] some APIs
that were unnecessarily restricted to `Sized` types
because there was previously no way to express pointer-thinness in generic code.

[lib]: https://play.rust-lang.org/?version=nightly&mode=debug&edition=2015&gist=bbeecccc025f5a7a0ad06086678e13f3


# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation


Let’s build generic type similar to `Box<dyn Trait>`,
but where the vtable pointer is stored in heap memory next to the value
so that the pointer is thin.
First, let’s get some boilerplate out of the way:

```rust
use std::marker::{PhantomData, Unsize};
use std::ptr::{self, DynMetadata};

trait DynTrait<Dyn> = Pointee<Metadata=DynMetadata<Dyn>>;

pub struct ThinBox<Dyn: ?Sized + DynTrait<Dyn>> {
    ptr: ptr::NonNull<WithMeta<()>>,
    phantom: PhantomData<Dyn>,
}

#[repr(C)]
struct WithMeta<T: ?Sized> {
    vtable: DynMetadata,
    value: T,
}
```

Since [unsized rvalues] are not implemented yet,
our constructor is going to “unsize” from a concrete type that implements our trait.
The `Unsize` bound ensures we can cast from `&S` to a `&Dyn` trait object
and construct the appropriate metadata.

[unsized rvalues]: https://github.com/rust-lang/rust/issues/48055

We let `Box` do the memory layout computation and allocation:

```rust
impl<Dyn: ?Sized + DynTrait> ThinBox<Dyn> {
    pub fn new_unsize<S>(value: S) -> Self where S: Unsize<Dyn> {
        let vtable = ptr::metadata(&value as &Dyn);
        let ptr = NonNull::from(Box::leak(Box::new(WithMeta { vtable, value }))).cast();
        ThinBox { ptr, phantom: PhantomData }
    }
}
```

(Another possible constructor is `pub fn new_copy(value: &Dyn) where Dyn: Copy`,
but it would involve slightly more code.)

Accessing the value requires knowing its alignment:

```rust
impl<Dyn: ?Sized + DynTrait> ThinBox<Dyn> {
    fn data_ptr(&self) -> *mut () {
        unsafe {
            let offset = std::mem::size_of::<DynMetadata<Dyn>();
            let value_align = self.ptr.as_ref().vtable.align();
            let offset = align_up_to(offset, value_align);
            (self.ptr.as_ptr() as *mut u8).add(offset) as *mut ()
        }
    }
}

/// <https://github.com/rust-lang/rust/blob/1.30.0/src/libcore/alloc.rs#L199-L219>
fn align_up_to(offset: usize, align: usize) -> usize {
    offset.wrapping_add(align).wrapping_sub(1) & !align.wrapping_sub(1)
}

// Similarly Deref
impl<Dyn: ?Sized + DynTrait> DerefMut for ThinBox<Dyn> {
    fn deref_mut(&mut self) -> &mut Dyn {
        unsafe {
            &mut *<*mut Dyn>::from_raw_parts(self.data_ptr(), *self.ptr.as_ref().vtable)
        }
    }
}
```

Finally, in `Drop` we may not be able to take advantage of `Box` again
since the original `Sized` type `S` is not statically known at this point.

```rust
impl<Dyn: ?Sized + DynTrait> Drop for ThinBox<Dyn> {
    fn drop(&mut self) {
        unsafe {
            let layout = /* left as an exercise for the reader */;
            ptr::drop_in_place::<Dyn>(&mut **self);
            alloc::dealloc(self.ptr.cast(), layout);
        }
    }
}
```


# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The APIs whose full definition is found below
are added to `core::ptr` and re-exported in `std::ptr`:

* A `Pointee` trait,
  implemented automatically for all types
  (similar to how `Sized` and `Unsize` are implemented automatically).
* A `Thin` [trait alias].
  If this RFC is implemented before type aliases are,
  uses of `Thin` should be replaced with its definition.
* A `metadata` free function
* A `DynMetadata` struct
* A `from_raw_parts` constructor for each of `*const T`, `*mut T`, and `NonNull<T>`.

The bounds on `null()` and `null_mut()` function in that same module
as well as the `NonNull::dangling` constructor
are changed from (implicit) `T: Sized` to `T: ?Sized + Thin`.
Similarly for the `U` type parameter of the `NonNull::cast` method.
This enables using those functions with [extern types].

The `Pointee` trait is implemented for all types.
This can be relied on in generic code,
even if a type parameter `T` does not have an explicit `T: Pointee` bound.
This is similar to how the `Any` trait can be used without an explicit `T: Any` bound,
only `T: 'static`, because a blanket `impl<T: 'static> Any for T {…}` exists.
(Except that `Pointee` is not restricted to `'static`.)

For the purpose of pointer casts being allowed by the `as` operator,
a pointer to `T` is considered to be thin if `T: Thin` instead of `T: Sized`.
This similarly includes extern types.

`std::raw::TraitObject` and `std::raw` are deprecated and eventually removed.

[trait alias]: https://github.com/rust-lang/rust/issues/41517
[extern types]: https://github.com/rust-lang/rust/issues/43467

```rust
/// This trait is automatically implemented for every type.
///
/// Raw pointer types and reference types in Rust can be thought of as made of two parts:
/// a data pointer that contains the memory address of the value, and some metadata.
///
/// For statically-sized types (that implement the `Sized` traits)
/// as well as for `extern` types,
/// pointers are said to be “thin”: metadata is zero-sized and its type is `()`.
///
/// Pointers to [dynamically-sized types][dst] are said to be “fat”
/// and have non-zero-sized metadata:
///
/// * For structs whose last field is a DST, metadata is the metadata for the last field
/// * For the `str` type, metadata is the length in bytes as `usize`
/// * For slice types like `[T]`, metadata is the length in items as `usize`
/// * For trait objects like `dyn SomeTrait`, metadata is [`DynMetadata<Self>`][DynMetadata]
///   (e.g. `DynMetadata<dyn SomeTrait>`).
///
/// In the future, the Rust language may gain new kinds of types
/// that have different pointer metadata.
///
/// Pointer metadata can be extracted from a pointer or reference with the [`metadata`] function.
/// The data pointer can be extracted by casting a (fat) pointer
/// to a (thin) pointer to a `Sized` type with the `as` operator,
/// for example `(x: &dyn SomeTrait) as *const SomeTrait as *const ()`
/// or `(x: *const dyn SomeTrait).cast::<()>()`.
///
/// [dst]: https://doc.rust-lang.org/nomicon/exotic-sizes.html#dynamically-sized-types-dsts
#[lang = "pointee"]
pub trait Pointee {
    /// The type for metadata in pointers and references to `Self`.
    type Metadata: Copy + Send + Sync + Ord + Hash + Unpin;
}

/// Pointers to types implementing this trait alias are “thin”:
///
/// ```rust
/// fn this_never_panics<T: std::ptr::Thin>() {
///     assert_eq!(std::mem::size_of::<&T>(), std::mem::size_of::<usize>())
/// }
/// ```
pub trait Thin = Pointee<Metadata=()>;

/// Extract the metadata component of a pointer.
///
/// Values of type `*mut T`, `&T`, or `&mut T` can be passed directly to this function
/// as they implicitly coerce to `*const T`.
/// For example:
///
/// ```
/// assert_eq(std::ptr::metadata("foo"), 3_usize);
/// ```
///
/// Note that the data component of a (fat) pointer can be extracted by casting
/// to a (thin) pointer to any `Sized` type:
///
/// ```
/// # trait SomeTrait {}
/// # fn example(something: &SomeTrait) {
/// let object: &SomeTrait = something;
/// let data_ptr = object as *const SomeTrait as *const ();
/// # }
/// ```
pub fn metadata<T: ?Sized>(ptr: *const T) -> <T as Pointee>::Metadata {…}

impl<T: ?Sized> *const T {
    pub fn from_raw_parts(data: *const (), meta: <T as Pointee>::Metadata) -> Self {…}
}

impl<T: ?Sized> *mut T {
    pub fn from_raw_parts(data: *mut (), meta: <T as Pointee>::Metadata) -> Self {…}
}

impl<T: ?Sized> NonNull<T> {
    pub fn from_raw_parts(data: NonNull<()>, meta: <T as Pointee>::Metadata) -> Self {
        unsafe {
            NonNull::new_unchecked(<*mut _>::from_raw_parts(data.as_ptr(), meta))
        }
    }
}

/// The metadata for a `DynTrait = dyn SomeTrait` trait object type.
///
/// It is a pointer to a vtable (virtual call table)
/// that represents all the necessary information
/// to manipulate the concrete type stored inside a trait object.
/// The vtable notably it contains:
///
/// * type size
/// * type alignment
/// * a pointer to the type’s `drop_in_place` impl (may be a no-op for plain-old-data)
/// * pointers to all the methods for the type’s implementation of the trait
///
/// Note that the first three are special because they’re necessary to allocate, drop,
/// and deallocate any trait object.
///
/// It is possible to name this struct with a type parameter that is not a `dyn` trait object
/// (for example `DynMetadata<u64>`) but not to obtain a meaningful value of that struct.
#[derive(Copy, Clone)]
pub struct DynMetadata<DynTrait: ?Sized> {
    // Private fields
    vtable_ptr: ptr::NonNull<()>,
    phantom: PhantomData<DynTrait>
}

impl<DynTrait: ?Sized> DynMetadata<DynTrait> {
    /// Returns the size of the type associated with this vtable.
    pub fn size(self) -> usize { ... }

    /// Returns the alignment of the type associated with this vtable.
    pub fn align(self) -> usize { ... }

    /// Returns the size and alignment together as a `Layout`
    pub fn layout(self) -> alloc::Layout {
        unsafe {
            alloc::Layout::from_size_align_unchecked(self.size(), self.align())
        }
    }
}
```


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

A [previous iteration][2579] of this RFC proposed a `DynTrait`
that would only be implemented for trait objects like `dyn SomeTrait`.
There would be no `Metadata` associated type, `DynMetadata` was hard-coded in the trait.
In addition to being more general
and (hopefully) more compatible with future custom DSTs proposals,
this RFC resolves the question of what happens
if trait objects with super-fat pointers with multiple vtable pointers are ever added.
(Answer: they can use a different metadata type,
possibly like `(DynMetadata<dyn Trait>, DynMetadata<dyn OtherTrait>)`.)

[2579]: https://github.com/rust-lang/rfcs/pull/2579


# Prior art
[prior-art]: #prior-art

A previous [Custom Dynamically-Sized Types][cdst] RFC was postponed.
[Internals thread #6663][6663] took the same ideas
and was even more ambitious in being very general.
Except for `DynMetadata`’s methods, this RFC proposes a subset of what that thread did.

[cdst]: https://github.com/rust-lang/rfcs/pull/1524
[6663]: https://internals.rust-lang.org/t/pre-erfc-lets-fix-dsts/6663


# Unresolved questions
[unresolved-questions]: #unresolved-questions

* The name of `Pointee`. [Internals thread #6663][6663] used `Referent`.

* The location of `DynMetadata`. Is another module more appropriate than `std::ptr`?

* Should `DynMetadata` not have a type parameter?
  This might reduce monomorphization cost,
  but would force that the size, alignment, and destruction pointers
  be in the same location (offset) for every vtable.
  But keeping them in the same location is probably desirable anyway to keep code size small.

* The name of `Thin`.
  This name is short and sweet but `T: Thin` suggests that `T` itself is thin,
  rather than pointers and references to `T`.

* The location of `Thin`. Better in `std::marker`?

* Should `Thin` be added as a supertrait of `Sized`?
  Or could it ever make sense to have fat pointers to statically-sized types?

* Are there other generic standard library APIs like `ptr::null()`
  that have an (implicit) `T: Sized` bound that unnecessarily excludes extern types?

* Should `<*mut _>::from_raw_parts` and friends be `unsafe fn`s?

* API design: free functions v.s. methods/constructors on `*mut _` and `*const _`?

* Add `into_raw_parts` that returns `(*const (), T::Metadata)`?
  Using the `cast` method to a `Sized` type to extract the address as a thin pointer
  is less discoverable.
  Possibly *instead* of the metadata function?
