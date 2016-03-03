- Feature Name: custom\_dst
- Start Date: 01 March, 2016
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Allow Rust code to define custom fat pointers, and define slice code in terms of
these new operators (instead of the current `std::raw::Repr`).

# Motivation
[motivation]: #motivation

One major missing feature of Rust is custom dynamically sized types; it's
impossible to implement a slice type in Rust, for example. This has led to
things like the `std::raw::Repr` trait, a stopgap measure. It's also very
difficult to represent things like views into 2d slices; for example,
pixel buffers. In many places, something other than the Rust slice type is
needed (if you have both a width and a height, or if you have a width, height,
and stride, or even further).

This doesn't make it very *nice* to work with custom DSTs; it makes it possible,
and makes sure there's a strong base so that's there's a lot of room to grow.

# Detailed design
[design]: #detailed-design

The most important addition to the language shall be two new traits:

```rust
#[lang = "dst"]
trait Dst {
    type Meta: Copy;
}
#[lang = "sizeable"]
pub unsafe trait Sizeable {
    fn size_of_val(&self) -> usize;
}
```

Sizeable should be unsafe, because it's very easy to give an incorrect
implementation, and safe and unsafe code *must* rely on it being correct to not
create undefined behavior.

Note that it is specified that, if `size_of::<<T as Dst>::Meta>() == 0`, then
`size_of::<*const T>()` == `size_of::<*const ()>`. This is really important for
C compatibility, because our current compatibility with flexible arrays is
really terrible.

`Sizeable` *must* be implemented for all types, and shall be automatically
implemented for any type which does not implement `Dst`, and will be defined
as the `size_of_val` of each of the members.

To actually get use of the improved fat pointers, we must add four new
intrinsics (the names are up for bikeshedding):

```rust
// core
mod ptr {
    pub use intrinsics::{fat_ptr_meta, make_fat_ptr, make_fat_ptr_mut};
}
mod mem {
    pub use intrinsics::size_of_prelude;
}
mod intrinsics {
    extern "rust-intrinsic" {
        fn size_of_prelude<T: ?Sized>() -> usize;
        fn fat_ptr_meta<T: ?Sized + Dst>(ptr: *const T) -> T::Meta;
        fn make_fat_ptr<T: ?Sized + Dst>(data: *const (), meta: T::Meta) -> *const T;
        fn make_fat_ptr_mut<T: ?Sized + Dst>(data: *mut (), meta: T::Meta) -> *mut T;
    }
}
```

The following is an example implementation of `[T]`:

```rust
// The last field of a type that implements Dst must be a 0 sized array
#[lang = "slice"]
struct Slice<T>([T]);
impl<T> Dst for [T] {
    type Meta = usize;
}
unsafe impl<T> Sizeable for [T] {
    fn size_of_val(&self) -> usize {
        if self.len() > 0 {
            core::mem::size_of_val(&self[0]) * self.len()
        } else {
            0
        }
    }
}

impl<T> [T] {
    pub fn len(&self) -> usize {
        unsafe {
            core::intrinsics::fat_ptr_meta(self)
        }
    }

    pub fn as_ptr(&self) -> *const T {
        unsafe {
            &self.0 as *const [T] as *const T
        }
    }

    pub unsafe fn from_raw_parts(buf: *const T, len: usize) -> &[T] {
        unsafe {
            &*core::intrinsics::make_fat_ptr(buf as *const (), len)
        }
    }
}

impl<T: Drop> Drop for [T] {
    fn drop(&mut self) {
        for el in self {
            drop_in_place(el)
        }
    }
}
```

And the following, an example implementation of a pixel buffer; this is the real
meat of the RFC, custom fat pointers for libraries:

```rust
struct PixelBuffer([f32]);
impl Dst for PixelBuffer {
    type Meta = (usize, usize);
}
unsafe impl Sizeable for PixelBuffer {
    fn size_of_val(&self) -> usize {
        std::mem::size_of::<f32>() * self.width() * self.height()
    }
}
impl PixelBuffer {
    pub fn new_zeroed(width: usize, height: usize) -> Box<PixelBuffer> {
        if height != 0 {
            assert!(usize::max_value() / height > width);
        }
        let backing_store = vec![0.0; height * width];
        let ptr = Box::into_raw(backing_store.into_boxed_slice()) as *mut ();
        unsafe {
            Box::from_raw(
                std::ptr::make_fat_ptr_mut(ptr, (width, height)))
        }
    }

    pub fn width(&self) -> usize {
        unsafe {
            std::ptr::fat_ptr_meta(self).0
        }
    }

    pub fn height(&self) -> usize {
        unsafe {
            std::ptr::fat_ptr_meta(self).1
        }
    }
}
```

And now, an example implementation of a Pascal string type (this is good for
compatibility with C libraries that use flexible length arrays):

```rust
#[repr(C)]
struct PascalStr {
    len: usize,
    buffer: [u8],
}
impl Dst for PascalStr {
    type Meta = ();
}
unsafe impl Sizeable for PascalStr {
    fn size_of_val(&self) -> usize {
        std::mem::size_of_prelude::<Self>() + self.len
    }
}

impl PascalStr {
    pub fn len(&self) {
        self.len
    }
    pub fn from_str(s: &str) -> Box<PascalStr> {
        let backing_store = vec![0; std::mem::size_of::<usize>() + s.len()];
        let ptr = Box::into_raw(backing_store.into_boxed_slice()) as *mut ();
        let ps = Box::from_raw(std::ptr::make_fat_ptr_mut(ptr as *mut (), ()));
        ps.len = s.len();
        unsafe {
            std::ptr::copy_nonoverlapping(s, ps.buffer.as_mut_ptr(), ps.len)
        }

        ps
    }
}
```

Any type which is, currently, something like


```rust
struct CStr([u8]);
// or
struct Wrapper(u16, PascalStr);
```

will be treated just as they are today. A pointer to a `CStr` will be a 
`{ptr, usize}`, and a pointer to a `Wrapper` will be a `{ptr, ()}` (although it
will still be very difficult to create a `Wrapper`).

One more type which is always brought up when one talks about DSTs is the trait
object. Unfortunately, this change won't help trait objects that much; it will
allow us to get rid of `std::repr` for them, but most of it is compiler magic.
The implementation of these DST traits should looks something like the
following, however (unfortunately, inside the compiler because we can't impl
across all trait objects):

```rust
// trait is a separate type
impl Dst for Trait { 
    type Meta = TraitVtable;
}
unsafe impl Sizeable for Trait {
    fn size_of_val(&self) -> {
        // intrinsic
    }
}
```

# Drawbacks
[drawbacks]: #drawbacks

More complication in the language.

# Alternatives
[alternatives]: #alternatives

@eddyb's original idea of existentials `struct [T] = for<N: usize> [T; N];`,
which unfortunately doesn't give us things that can't be represented with
existentials, like a rectangular window into one of those `PixelBuffer`s.

My original idea had two types in `Dst`, Meta, and Data, where Data was similar
to the `[T]` at the end of the struct. This isn't nice for types like
`PascalStr`.

You could define a struct type
`PixelBuffer<'a> { width: usize, height: usize, ptr: &'a f32 }`, and a similar
PixelBufferMut struct, but this is *very* annoying, and cannot be returned by
Index traits.

# Unresolved questions
[unresolved]: #unresolved-questions

How should these fat pointers be passed, if they are larger than two pointers?

We likely need an equivalent to the C `sizeof(unsized)`; what should that
function be?
