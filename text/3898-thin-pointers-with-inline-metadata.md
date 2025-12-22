- Feature Name: (`thin_pointers`)
- Start Date: 2025-12-16
- RFC PR: [rust-lang/rfcs#3898](https://github.com/rust-lang/rfcs/pull/3898)
- Rust Issue:
  [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

This RFC adds `Thin<T>` that wraps `T`'s metadata inline, which makes `Thin<T>` thin even if
`T` is `!Sized`.

# Motivation
[motivation]: #motivation

Pointers of dynamically sized types (DSTs) are fat pointers, and they are not FFI-compatible,
which prevents some common types like `&str`, `&[T]`, and `&dyn Trait` from being passed across
the FFI boundaries.

## 1. Passing pointers of DSTs across FFI-boundaries is hard
Currently, it's difficult to use DSTs in FFI-compatible functions (even by-pointer).
For example, it is not allowed to use `&str`, `&[T]`, or `&dyn Trait` types in an
`extern "C"` function.
```rust
extern "C" fn foo(
    str_slice: &str, //~ ERROR not FFI-compatible
    int_slice: &[i32], //~ ERROR not FFI-compatible
    opaque_obj: &dyn std::any::Any, //~ ERROR not FFI-compatible
) { /* ... */ }
```

Instead, users have to wrap these types in `#[repr(C)]` structs:
```rust
/// FFI-compatible wrapper struct of `&[T]`
#[repr(C)]
pub struct Slice<'a, T> {
    len: usize,
    ptr: NonNull<()>,
    _marker: PhantomData<&'a [T]>,
}

/// FFI-compatible wrapper struct of `&str`
#[repr(C)]
pub struct StrSlice<'a> {
    len: usize,
    bytes: NonNull<u8>,
    _marker: PhantomData<&'a str>,
}

/// FFI-compatible wrapper of `&dyn Trait`
#[repr(C)]
pub struct DynTrait<'a> {
    vtable: NonNull<()>,
    ptr: NonNull<()>,
    _marker: PhantomData<&'a dyn Trait>,
}
```

Luckily, the [`abi_stable`] crate provides a series of FFI-compatible types like [`RSlice<'a, T>`],
[`RSliceMut`], [`RStr<'a>`], and an attribute macro [`sabi_trait`] that makes ABI-stable trait
objects (which are also FFI-compatible).

However, that is tedious and non-exhaustive because the library writer cannot enumerate all
compound DSTs (e.g. ADTs with a DST field) exhaustively.

[`abi_stable`]: https://crates.io/crates/abi_stable
[`RSlice<'a, T>`]: https://docs.rs/abi_stable/latest/abi_stable/std_types/struct.RSlice.html
[`RSliceMut`]: https://docs.rs/abi_stable/latest/abi_stable/std_types/struct.RSliceMut.html
[`RStr<'a>`]: https://docs.rs/abi_stable/latest/abi_stable/std_types/struct.RStr.html
[`sabi_trait`]: https://docs.rs/abi_stable/latest/abi_stable/attr.sabi_trait.html

## 2. Slices cannot be unsized to trait objects

Suppose there is a `dyn`-safe trait `MyTrait`, and it is implemented for `[T]`. However, it is
not possible to convert an `&[T]` to an `&dyn MyTrait` because `[T]` doesn't implement `Sized`. 
```rust
trait MyTrait {
    fn foo(&self);
}

impl<T> MyTrait for [T] {
    fn foo(&self) { /* ... */ }
}

fn as_my_trait<T>(x: &[T]) -> &dyn MyTrait {
    x //~ ERROR the size for values of type `[T]` cannot be known at compilation time
}
```

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation
To overcome the obstacles above, we introduce a `Thin<T>` wrapper that stores the metadata and
a (sized) value inside and thus keeps pointers of `Thin<T>` thin.

## Passing DST pointers across the FFI boundaries
```rust
extern "C" fn foo(
    str_slice: &Thin<str>, // ok because `&Thin<str>` is thin
    int_slice: &Thin<[i32]>, // ok because `&Thin<[i32]>` is thin
    opaque_obj: &Thin<dyn std::any::Any>, // ok because `&Thin<dyn std::any::Any>` is thin
} { /* ... */ }

// Construct the values of DSTs on stack
let str_slice: &Thin<str> = thin_str!("something");
let int_slice: &Thin<[i32]> = &Thin::new_unsized([1, 2, 3]);
let opaque_obj: &Thin<dyn std::any::Any> = &Thin::new_unsized(String::from("hello"));
// Pass the thin DSTs across FFI boundaries
unsafe {
    foo(str_slice, int_slice, opaque_obj);
}
```
## Making trait objects of slices
```rust
trait MyTrait {
    fn foo(&self);
}

impl<T> MyTrait for Thin<[T]> {
    fn foo(&self) { /* ... */ }
}

// Construct a thin `Thin<[i32]>` on stack
let value: &Thin<[i32]> = &Thin::new_unsized([1, 2, 3]);
// Coerce it to a trait object
// where `+ ValueSized` is needed to indicate that the size of this trait object
// is calculated from its value.
let dyn_value: &dyn MyTrait + ValueSized = value; // ok because `Thin<[i32]>` is thin
// Calls `<Thin<[i32]> as dyn MyTrait>::foo`
dyn_value.foo();
```
## Unify normal and thin containers
Given that:
* [`List<T>`] in rustc that is a thin `[T]` with the metadata (length) on the head;
* [`ThinVec<T>`] that put the length and capacity components together with its contents on the heap;
* [`ThinBox<T>`] like `Box<T>` but put the metadata together on the heap;
* [`thin_trait_object`], an attribute macro that makes a thin trait object (by manually
  constructing the vtable).

Now they can be rewritten as:
- `List<T>` -> `&Thin<[T]>`
- `ThinVec<T>`, technically `Box<(usize, Thin<[MaybeUninit<T>]>)>` (in representation)
- `ThinBox<T>` -> `Box<Thin<T>>`
- `BoxedTrait` -> `Box<Thin<dyn Trait>>`

where much less boilerplate code is needed.

[`List<T>`]: https://doc.rust-lang.org/nightly/nightly-rustc/rustc_middle/ty/type.List.html
[`ThinVec<T>`]: https://docs.rs/thin-vec/latest/thin_vec/struct.ThinVec.html
[`ThinBox<T>`]: https://doc.rust-lang.org/std/boxed/struct.ThinBox.html
[`thin_trait_object`]: https://docs.rs/thin_trait_object/latest/thin_trait_object/attr.thin_trait_object.html

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation
## Add `ValueSized` to the sized hierarchy

Regarding [sized hierarchy], `Thin` is more than `PointeeSized` but not `MetaSized`:
- it is not `MetaSized` because the metadata is not carried by the pointer itself;
- it is more than `PointeeSized` because we actually know its size by reading the metadata
  stored inside.

We need to add new stuff to the sized hierarchy, named `ValueSized`, to indicate a value of
which the size is known by reading its value, as mentioned in [RFC 3729 (comments)].

```rust
// mod core::marker;

/// Indicates that a type's size is known from reading its value.
/// 
/// Different from `MetaSized`, this requires pointer dereferences.
#[lang_item = "value_sized"]
pub trait ValueSized: PointeeSized {}

// Change the bound of `MetaSized: PointeeSized` to `MetaSized: ValueSized`
#[lang_item = "meta_sized"]
pub trait MetaSized: ValueSized {}
```

For `dyn Trait + ValueSized` types, the `MetadataSize` entry of the common vtable entries
will be a method of `fn size(&self) -> usize` which computes the value's size at runtime,
instead of a compile-time constant value.


[sized hierarchy]: https://github.com/rust-lang/rust/issues/144404
[RFC 3729 (comments)]: https://github.com/davidtwco/rfcs/blob/sized-hierarchy/text/3729-sized-hierarchy.md#user-content-fn-9-ebb4bca70c758b473dca6f49c3bb3cbc

## Public APIs

The public APIs of `Thin` consist of 2 parts.

### `Thin<T, U>`
`Thin<T, U>` is a (maybe unsized) value of `T` with the metadata type of `U` carried on.

Typically, `U = T` or `U` is some type that `T: Unsize<U>`.
```rust
// mod core::thin;

/// Wrapping a DST `T` with its metadata inlined,
/// then the pointers of `Thin<T>` are thin.
///
/// The generic type `U` is for two-stage construction of
/// `Thin`, i.e., `Thin<T, U> where T: Unsize<U>` must be
/// constructed first, then coerced (unsized) to `Thin<U>`
/// (aka `Thin<U, U>`)
#[repr(C)]
pub struct Thin<T: Pointee, U: Pointee = T> {
    metadata: U::Metadata,
    data: EraseMetadata<T>,
}

// The size is known via reading its metadata.
impl<U: Pointee> ValueSized for Thin<U> {}

// Value accesses
impl<U: Pointee> ops::Deref for Thin<U> {
    type Target = U;
    fn deref(&self) -> &U;
}
impl<U: Pointee> ops::DerefMut for Thin<U> {
    fn deref_mut(&mut self) -> &mut U;
}
```

### `EraseMetadata<T>`
`EraseMetadata<T>` is a wrapper of (maybe unsized) `T`, which ignores the metadata of `T`.

For example, both `&EraseMetadata<dyn Trait>` and `&EraseMetadata<[u8]>` have the same size as a thin
pointer `&()`.

```rust
/// A wrapper that ignores the metadata of a type.
#[lang = "erase_metadata"]
#[repr(transparent)]
pub struct EraseMetadata<T: Pointee>(T);

// For sized types, `EraseMetadata` is a simple wrapper.
impl<T: Sized> Sized for EraseMetadata<T> {}
impl<T: Sized> ops::Deref for EraseMetadata<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.0
    }
}
impl<T: Sized> ops::DerefMut for EraseMetadata<T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.0
    }
}

impl<T: Sized> EraseMetadata<T> {
    /// Wrap a sized value into an `EraseMetadata`.
    pub fn new(inner: T) -> EraseMetadata<T> {
        EraseMetadata(inner)
    }
    /// Unwrap a sized value from an `EraseMetadata`.
    pub fn into_inner(self) -> T {
        self.0
    }
}

// For unsized types, `EraseMetadata` is completely opaque because it is unsafe
// to read the inner value without the metadata.

// The size is unknown because the metadata is erased.
impl<T: MetaSized> PointeeSized for EraseMetadata<T> {}
```

## Value constructions
For a sized type `Thin<T>`, it can be constructed with `Thin::<T>::new`.
For an unsized (`MetaSized`) type `Thin<U>`, in general, it requires 3 steps to construct
a `Thin<U>` on stack or on heap:
- construct a sized value of `Thin<T, U>` via `Thin::<T, U>::new_unsized` (where `T: Unsize<U>`).
- obtain a pointer (i.e., `&`, `&mut`, `Box`, `Rc`, `Arc`, etc.) of `Thin<T, U>` via
  their constructors.
- coerce the pointer of `Thin<T, U>` to the pointer of `Thin<U>`.

Here are the APIs related to value constructions mentioned above:
```rust
impl<T: Sized> Thin<T> {
    /// Create a sized `Thin<T>` value, which is a simple wrapper of `T`
    pub fn new(value: T) -> Thin<T> {
        Self {
            metadata: (), // Sized type `T` has empty metadata
            data: EraseMetadata(value),
        }
    }
}

impl<T: Sized, U: Pointee> Thin<T, U> {
    /// Create a sized `Thin<T, U>` value with metadata of unsized type `U`,
    /// which can be coerced (unsized) to `Thin<U>`
    pub fn new_unsized(value: T) -> Self
    where
        T: Unsize<U>,
    {
        Self {
            metadata: ptr::metadata(&value as &U),
            data: EraseMetadata(value),
        }
    }
    /// Consume the `Thin<T>` and return the inner wrapped value of `T`
    pub fn into_inner(self) -> T {
        self.data.0
    }
}

/// `Thin<T, U>` has the same layout as `Thin<U>`, so that it can be coerced
/// (unsized) to `Thin<U>`
impl<T: Sized, U: Pointee> Unsize<Thin<U>> for Thin<T, U> 
where
    T: Unsize<U>
{}
```

# Drawbacks
[drawbacks]: #drawbacks

## `Thin` is confusing with existing concepts
The term `Thin` has a different meaning from a previous term: the trait `core::ptr::Thin`
(that means types with `Metadata = ()`).

## `ValueSized` doesn't fit `Thin` 100%
Generally, the size of `ValueSized` type is calculated from its value. For instance, the size of
a *real* C string (not `core::ffi::CStr`) is determined by counting the bytes until a `\0`
is reached. In general, `ValueSized` types are `Freeze`, or else the size can change after
calculating from its value. Hence, `CStrV2` (the real C string type) is `ValueSized` but
`UnsafeCell<CStrV2>` is not.

However, for `Thin<T: MetaSized>`, we are definitely sure that `<T as Pointee>::Metadata` is
`Freeze`, then even if `T: !Freeze`, `Thin<T>: ValueSized` still holds.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

Don't introduce `Thin`, but use the extern type instead, then `Thin` can be represented by:
```rust
pub struct Thin<T: Pointee, U: Pointee = T> {
    metadata: U::Metadata,
    marker: PhantomData<T>,
    extern_type: ThinExtra,
}

extern "C" {
    type ThinExtra;
}
```
However, it is hard to construct values on stack.

# Prior art
[prior-art]: #prior-art

- [RFC 2594: Custom DSTs] introduced a more general way to define a DST. This RFC
  focuses on how to "thinify" a pointer to an existing DST via inlining the metadata,
  which is a convenient helper for some common situations.
- [RFC 3536: Trait for `!Sized` thin pointers] was very similar to this RFC.
    This RFC doesn't change the semantics of existing DSTs like `[T]`, which can avoid
    potential breaking changes of composed DSTs.

[RFC 2594: Custom DSTs]: https://github.com/rust-lang/rfcs/pull/2594
[RFC 3536: Trait for `!Sized` thin pointers]: https://github.com/rust-lang/rfcs/pull/3536

# Unresolved Questions
[unresolved-questions]: #unresolved-questions

- Should the trait `ValueSized` provide a user-implementable method `size`? (In this RFC,
  such a `size` method can only be generated by compiler.)

## Future possibilities
[future-possibilities]: #future-possibilities

None yet.