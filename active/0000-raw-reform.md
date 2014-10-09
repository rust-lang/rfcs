- Start Date: 2014-10-07
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

* Introduce RawSlice and RawMutSlice extension traits to be implemented on
`*const [T]` and `*mut [T]` that provide parts of the slice and ptr API to better bridge the
gap between the two.

* Provide unsafe versions of the slicing and indexing operators on raw slices to provide ergonomic
unchecked slice manipulation.

* Provide unsafe versions of the addition, subtraction, and indexing operators on raw ptrs to
provide more ergonomic ptr manipulation.

* Deprecate some of the free functions in `ptr`, and duplicate the rest as convenience methods on
the RawPtr extension traits.

* Deprecate *all* `unsafe` methods on slices and `slice::raw` in favour of raw slices.

* Add conversion methods to the three families of extension traits for conveniently moving between
them.

# Motivation

Unsafe code is a reality of high performance or low-level programming. Ideally, Rust can provide
safe and effecient abstractions for most use-cases, but there's always going to be unsafe code
that needs to be written, even if it's just to make the safe public APIs. Unsafe code is also by
definition the most important code to write correctly. Incorrect unsafe code can be exploited.

Therefore, unsafe code should be *ergonomic* to read and write. Unsafe code should look like safe
code wherever possible. Unsafe APIs should be as easy to use as safe APIs. Rust has unsafe blocks
to warn the programmer and delimit dangerous operations, it doesn't need to force them to write
verbose and repetitive code to discourage their use. Worse, forcing laborious and repetitive unsafe
code can reduce readability and introduce more places for errors.

Two of the biggest sources of unsafe code are slices and raw ptrs. Unsafe code in slices largely
revolves around bypassing bounds checks with `unsafe_*` variants of common operations. This might
be done to improve performance in hot code paths, or just to reduce failbloat. The `slice::raw`
module also provides constructors for converting a raw ptr to a slice, and methods for completely
removing an index from the end of a slice.

Unsafe code for raw ptrs represents a much more diverse set of usecases. They can be used to:

* Maintain references that would otherwise violate Rust's aliasing rules.
* Maintain a handle to a manually allocated buffer or value.
* Unsafely manipulate the contents of a slice without bounds checking.
* Bulk move the contents of a slice or buffer in memory.
* Initialize, zero, or copy memory without interacting with `drop`.
* etc.

Note that there is a *significant* overlap between rawptr behaviour and slice behaviour. In C,
the strong relationship between arrays and pointers is precisely captured by the fact that the two
aren't different types *at all*. Pointers are passed where arrays are expected, and the programmer
is expected to track length manually. In Rust, slices are used liberally to avoid the many problems
this has lead to, but when the raw C-like representation is desirable no middle-ground is provided.
If the programmer wants to do something C-like, they are forced to use a raw ptr and track length
manually.

However, as an incidental result of DST Rust now has `*const [T]` and `*mut [T]`. These types
could be used to bridge the gap between the "pure" ptr representation of an array, and the "safe"
slice representation. Rather than requiring programmers to use verbose `unsafe_*` methods or drop
all the way down to ptrs, these "raw" slices can provide unsafe versions of the slice
operations, and more convenient versions of the ptr ones. If a programmer has a slice, they can
just call `.as_raw` on it, and use the resultant object as if it was a slice, but without any
bounds checking overhead. They can also more ergonomically convert the raw slice back into a normal
slice when they're done. Similarly, rather than fumbling with offseting and then using raw ptrs,
raw slices can provide versions of the ptr methods which take an index to offset to.

# Detailed design

Introduce two new extension traits `RawSlice` and `RawMutSlice` to be implemented on `*const [T]`
and `*mut [T]` accordingly with the following APIs:

```
trait RawSlice<T> {
    /// Gets the length of the rawslice
    fn len(self) -> uint;

    /// Converts the rawslice into a slice
    unsafe fn as_slice<'a>(self) -> &'a [T];

    /// Converts the rawslice into a rawptr
    fn as_ptr(self) -> *const T;

    /// Reads the data at the given index and interprets it as a value of T.
    /// This does not move the value out, and ignores the length of the raw slice.
    unsafe fn read(self, index: uint) -> T;
}
```

```
trait RawMutSlice<T> : RawSlice<T> {
    //// Converts the rawslice into a mutable slice
    unsafe fn as_mut_slice<'a>(self) -> &'a mut[T];

    /// Converts the rawslice into a mutable rawptr
    fn as_mut_ptr(self) -> *mut T;

    /// Writes a value to the given index without reading or destroying whatever
    /// data might exist at that index. Appropriate for initializing unitialized data.
    /// Ignores the length of the raw slice.
    unsafe fn write(self, index: uint, val: T);

    /// Sets every byte in the slice to to the given one, without reading or destroying whatever
    /// data might have been contained. Can be used to zero memory out.
    unsafe fn write_bytes(self, u8: byte);

    /// Copies the contents of the given rawslice into this one, assuming that they might
    /// have overlapping regions of memory. Uses from.len() to determine the length of the
    /// copied data, but does not consider the target's length.
    unsafe fn copy(self, from: *const[T]);

    /// Copies the contents of the given rawslice into this one, assuming they don't have any
    /// overlapping memory. Uses `from.len()` to determine the length copied data, but does
    /// not consider the target's length.
    unsafe fn copy_nonoverlapping(self, from: *const[T]);
}
```

In addition, `*const [T]` and `*mut [T]` should provide *unsafe* implementations of the slicing
and indexing operators. That is, provide the operators, but require them to be used in an unsafe
block. Unsafe operators should behave exactly like dereffing a raw ptr. Unsafe slicing should
yield a new raw slice. Unsafe indexing takes a uint, and returns (and then deref) an
`&T` or `&mut T` as appropriate (an alternative can be found in the alternatives section).

All of these operations are *completely* unchecked, even though the length information is
available. If you wish to perform a checked operation on a raw slice, you can coerce to a
normal slice and perform the operation there. Similarly, if you wish to perform an unchecked
operation on a normal slice, you can safely coerce to a raw slice temporarily.

Unsafe addition, subtraction, and indexing of `*const T` and `*mut T`
should also be provided with ints rather than uints. Unsafe addition and subtraction should be
equivalent to calling `offset` on the ptr today. Indexing should have the same behaviour as
indexing into a raw slice.

Note that providing indexing on both `*const [T]` and `*const U` appears to be a conflict, as `U`
can equal `[T]`. As a solution to this, indexing should be provided only on `*const U` where
`U: Sized` (an alternative can be found in the alternatives section).

There are several possible ways to obtain these unsafe operators. For discussion's sake, we
will note several options ranging from very conservative to very extreme:

* Extend whatever trick is used in the compiler to make dereffing a raw ptr possible to these
other operators for these specific types.

* Add UnsafeIndex, UnsafeIndexMut, UnsafeSlice, etc. traits that the compiler
understands as also providing the relevant operators. Also potentially extend this to some or
all of the other operator traits. If UnsafeDeref is added in this way, it should *not* enable
autoderef.

* Add a mechanism for implementing a trait that expects safe impls with unsafe impls. Such unsafe
impls would not be permissable implementations in generic code. Perhaps an `Unsafe?` annotation
could be introduced to opt in to accepting these impls, where are all methods are assumed to be
unsafe.

We propose the most conservative approach of compiler special-casing, as it has the least
ramifications for 1.0, and is backwards compatible with the other options.

In order to better normalize unsafe manipulation of slices and ptrs, this RFC also proposes the
following refactors:

* Deprecate `ptr::zero_memory` as a weak convenience for `set_memory`.

* Deprecate `ptr::read_and_zero` as a weak convenience for `read` and `set_memory`.

* Deprecate `RawPtr::null` and `RawPtr::is_not_null` as awkward to use.

* Deprecate all of `slice::raw` as poorly motivated, especially with raw slices available.

* Deprecate `ImmutableSlice::unsafe_get` and `MutableSlice::unsafe_mut` in favour of using raw
slices and indexing into them.

* Add `as_raw` and `as_raw_mut` to the `Slice` and `MutSlice` extension traits.

* Add `as_slice`, `as_mut_slice`, `as_raw_slice`, and `as_raw_mut_slice` to the
`RawPtr` and `RawMutPtr` extension traits.

* Move all remaining free functions from `ptr` except for the `null` and `null_mut` constructors
to the RawPtr and RawPtrMut extension traits as appropriate. These make much more sense as methods.
In addition, this would have the benefit of removing the need to explicitly cast `*mut`'s to
`*const`'s in some places.

* Rename `copy_memory`, `copy_nonoverlapping_memory`, and `set_memory` to `copy`,
`copy_nonoverlapping` and `write_bytes` for simplification.

* Rename `RawPtr::to_uint` to `RawPtr::as_uint` to match conversion conventions.

* Make the RawPtr traits take self by value, because ptrs are `copy`.

When all of this is done, slices will no longer have *any* unsafe methods. This clearly delinates
slices as "the safe way" and raw slices as "the unsafe way". The naive transition path will be
`foo.unsafe_get(i)` -> `foo.as_raw()[i]`. However if lots of unsafe work needs to be done, the
`as_raw` conversion need only be done once.

We also duplicate the free functions on `ptr` to the RawPtr traits because while methods are
generally more convenient to use, raw ptrs are frequently made very transiently.
ref-to-ptr coercion means the free functions can be more ergonomic to use when the user only has
a proper reference to the value. Therefore we offer both free functions and methods so that the
most ergonomic calling style can be used. When UFCS is fully available, it should be possible to
import the RawPtr methods *as* free functions, in which case the free functions can reasonably
be deprecated.

The ptr module ends up only having the following functions:

```
/// Create a null pointer.
pub fn null<T>() -> *const T;

/// Create an unsafe mutable null pointer.
pub fn null_mut<T>() -> *mut T;

/// Reads the value from *src and returns it.
pub unsafe fn read<T>(src: *const T) -> T;

/// Unsafely overwrites a memory location with the given value without destroying the old value.
pub unsafe fn write<T>(dest: *mut T, src: T);

/// Unsafely overwrites `count * size_of<T>()` bytes with the given byte.
pub unsafe fn write_bytes(dest: *mut T, byte: u8, count: uint);

/// Swaps the values of `x` and `y`. Note that in contrast to `mem::swap`, `x` and `y` may point
/// to the same address of memory. Useful for making some operations branchless.
pub unsafe fn swap<T>(x: *mut T, y: *mut T);

/// Replace the value at a mutable location with a new one, returning the old value. This is simply
/// a convenience for calling `mem::replace` with a raw pointer.
pub unsafe fn replace<T>(dest: *mut T, src: T) -> T;

/// Copies `count * size_of<T>()` many bytes from `src` to `dest`,
/// assuming that the source and destination *may* overlap.
pub unsafe fn copy<T>(dest: *mut T, src: *const T, count: uint);

/// Copies `count * size_of<T>()` many bytes from `src` to `dest`,
/// assuming that the source and destination *do not* overlap.
pub unsafe fn copy_nonoverlapping<T>(dest: *mut T, src: *const T, count: uint);
```

And the ptr traits end up looking like this:

```
/// Methods on raw pointers
pub trait RawPtr<T> {
    /// Returns true if the pointer is equal to the null pointer.
    fn is_null(self) -> bool;

    /// Returns the value of this pointer (ie, the address it points to)
    fn as_uint(self) -> uint;

    /// Converts the pointer into a raw slice.
    fn as_raw_slice(self, len: uint) -> *const [T];

    /// Converts the pointer into a slice.
    unsafe fn as_slice<'a>(self, len: uint) -> &'a [T];

    /// Returns `None` if the pointer is null, or else returns a reference to the
    /// value wrapped in `Some`.
    unsafe fn as_ref<'a>(self) -> Option<&'a T>;

    /// Calculates the offset from a pointer. The offset *must* be in-bounds of
    /// the object, or one-byte-past-the-end.  `count` is in units of T; e.g. a
    /// `count` of 3 represents a pointer offset of `3 * sizeof::<T>()` bytes.
    unsafe fn offset(self, count: int) -> Self;

    /// Reads the value from `self` and returns it.
    unsafe fn read(self) -> T;
}
```

```
/// Methods on mutable raw pointers
pub trait RawMutPtr<T>{
    /// Returns `None` if the pointer is null, or else returns a mutable reference
    /// to the value wrapped in `Some`. As with `as_ref`, this is unsafe because
    /// it cannot verify the validity of the returned pointer.
    unsafe fn as_mut<'a>(self) -> Option<&'a mut T>;

    /// Converts the pointer into a raw mutable slice.
    fn as_raw_mut_slice(self, len: uint) -> *mut [T];

    /// Converts the pointer into a mutable slice.
    unsafe fn as_mut_slice<'a>(self, len: uint) -> &'a mut [T];

    /// Unsafely overwrite a memory location with the given value without destroying
    /// the old value.
    ///
    /// This operation is unsafe because it does not destroy the previous value
    /// contained at the location `dst`. This could leak allocations or resources,
    /// so care must be taken to previously deallocate the value at `dst`.
    unsafe fn write(self, src: T);

    /// Sets the `count * size_of<T>()` bytes at the address of this pointer to the the given
    /// byte. Good for zeroing out memory.
    unsafe fn write_bytes(self, byte: u8, count: uint);

    /// Copies `count * size_of<T>()` many bytes from `src` to the address of this pointer,
    /// assuming that the source and destination *may* overlap.
    unsafe fn copy(self, src: *const T, count: uint);

    /// Copies `count * size_of<T>()` many bytes from `src` to the address of this pointer,
    /// assuming that the source and destination *do not* overlap.
    unsafe fn copy_nonoverlapping(self, src: *const T, count: uint);

    /// Swaps the values of `self` and `y`. Note that in contrast to `mem::swap`, `x` and `y`
    /// may point to the same address of memory. Useful for making some operations branchless.
    pub unsafe fn swap<T>(self, y: *mut T);

    /// Replace the value of the pointer, returning the old value. This is simply
    /// a convenience for calling `mem::replace` with a raw pointer.
    pub unsafe fn replace<T>(self, src: T) -> T;
}
```

And the `ImmutableSlice` and `MutableSlice` traits end up like this:

```
pub trait ImmutableSlice<'a, T> {
    // ... a bunch of stuff that doesn't change ...

    /// Converts the slice into a raw slice
    fn as_raw(&self) -> *const [T];

    // remove this:
    unsafe fn unsafe_get(self, index: uint) -> &'a T
}
```

```
pub trait MutableSlice<'a, T> {
    // ... a bunch of stuff that doesn't change ...

    /// Converts the slice into a raw slice
    fn as_raw_mut(self) -> *mut [T];

    // remove this:
    unsafe fn unsafe_mut(self, index: uint) -> &'a mut T
}
```

## Case study: Porting insertion sort from raw ptrs to raw slices

Old code:
```
fn insertion_sort<T>(v: &mut [T], compare: |&T, &T| -> Ordering) {
    let len = v.len() as int;
    let buf_v = v.as_mut_ptr();

    // 1 <= i < len;
    for i in range(1, len) {
        // j satisfies: 0 <= j <= i;
        let mut j = i;
        unsafe {
            // `i` is in bounds.
            let read_ptr = buf_v.offset(i) as *const T;

            // find where to insert, we need to do strict <,
            // rather than <=, to maintain stability.

            // 0 <= j - 1 < len, so .offset(j - 1) is in bounds.
            while j > 0 && compare(&*read_ptr, &*buf_v.offset(j - 1)) == Less {
                j -= 1;
            }

            // shift everything to the right, to make space to
            // insert this value.

            // j + 1 could be `len` (for the last `i`), but in
            // that case, `i == j` so we don't copy. The
            // `.offset(j)` is always in bounds.

            if i != j {
                let tmp = ptr::read(read_ptr);
                ptr::copy_memory(buf_v.offset(j + 1),
                                 &*buf_v.offset(j),
                                 (i - j) as uint);
                ptr::copy_nonoverlapping_memory(buf_v.offset(j),
                                                &tmp as *const T,
                                                1);
                mem::forget(tmp);
            }
        }
    }
}
```

New code:
```
fn insertion_sort<T>(v: &mut [T], compare: |&T, &T| -> Ordering) {
    let len = v.len() as int;
    let buf_v = v.as_raw_mut();

    // 1 <= i < len;
    for i in range(1, len) {
        // j satisfies: 0 <= j <= i;
        let mut j = i;
        unsafe {
            // `i` is in bounds.
            let read_ptr = buf_v.as_ptr().offset(i);

            // find where to insert, we need to do strict <,
            // rather than <=, to maintain stability.

            // 0 <= j - 1 < len, so j - 1 is in bounds.
            while j > 0 && compare(&*read_ptr, &buf_v[j - 1])) == Less {
                j -= 1;
            }

            // shift everything to the right, to make space to
            // insert this value.

            // j + 1 could be `len` (for the last `i`), but in
            // that case, `i == j` so we don't copy. The
            // offset by j is always in bounds.

            if i != j {
                let tmp = ptr::read(read_ptr);
                buf_v[j + 1, ..].copy(buf_v[j, i]);
                buf_v.write(j, tmp);
            }
        }
    }
}
```

The resultant code is more concise, and the use of slicing syntax makes it more clear what exactly
is happening. Note that an explicit length computation is completely dropped, as it falls
naturally out of the range that the slice is over.

# Drawbacks

* May encourage more unsafe code to be written where safe code is acceptable.

* Adds more special-casing to the compiler to handle the unsafe operators.

* The raw slice API *significantly* duplicates the ptr API. The authors of this RFC consider this
a significant trade-off for the sake of ergonomics.

* As written, `RawSlice<T>` theoretically prevents `RawPtr<T>` ever being implemented for
`T: Unsized`. This could be potentially worked around by a very specific exception to the blanket
impl for `T = [U]`.

* Coercing to a raw slice to perform an unchecked operation completely clobbers all lifetime
information that would otherwise normally be available. This is unfortunate.

# Alternatives

* All of this design can be *easily* implemented *right now* thanks to DST, except for the unsafe
operators. Therefore, it may be desirable to move forward with this design without the operators,
but just some concrete methods that can be replaced by operators in the future. However, the
operators are *very* attractive for improving ergonomics. They would also allow a more fluid
transition between code that uses safe slices and raw slices. This would encourage safe design and
unsafe deployment with minimal translation (and therefore minimal chance for introducing errors).
Carefully written C code could also be translated to Rust more directly with the operators
available.

* The `*const [T]` is a `*const U` for `U = [T]` ambiguity could also be resolved by
special-casing  `*const [T]` as "different" somehow in the compiler. This would allow indexing
into raw ptrs of other unsized types. However `*const [T]` should *really* just be used most of
the time when there really is an underlying buffer.

* Unsafe indexing could also just return a raw ptr that isn't auto-dereffed. This could have
benefits for "being explicit" in unsafe code, and make it easier to get a rawptr to a particular
index of a raw slice. It would also be reasonable to then remove many of the "with an index" ptr
operations that are on raw slice in favour of indexing and then calling the ptr method.
However it may reduce ergonomics in the common case.

# Unresolved questions

* Are some of the proposed-to-be-deprecated functions worth saving?

* Should `ptr.offset` be deprecated in favour of only using pointer arithmetic? Being able to
method chain offsets is moderately convenient.

* `*const [T]` is a `*const U` for `U = [T]`, but we propose indexing on these two types to have
different effects. Must one be chosen, or can this be disambiguated somehow? How would this
interact with the possibility of a future migration to one of the more extreme choices for unsafe
operators?

* raw slices could also support `offset`, and consequently unsafe addition and subtraction. This
could be useful for shifting a window into a larger slice around. This would also bring raw slices
and pointers closer together in functionality. Unclear if this is desirable. May accidentally fall
out of just providing the functionality on raw ptrs, unless explicitly prevented.

* More slice methods can be ported to raw slices to provide more unchecked operations. It may
be worth considering this. This can be done in a back-compat way later, though.

* Checked or truncating versions of the copy methods on raw slices?

* Maybe deprecate `is_null()` in favour of `== ptr::null()`?
