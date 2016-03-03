- Feature Name: custom\_dst
- Start Date: 02 March, 2016
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

# Detailed design
[design]: #detailed-design

The most important addition to the language shall be one new trait:

```rust
#[lang = "dst"]
unsafe trait Dst {
    type Meta: Copy;
    fn size_of_val(&self) -> usize;
}
```

The `Dst` trait should be unsafe, because it's very easy to give an incorrect
implementation of `size_of_val`, and safe and unsafe code *must* rely on it
being correct to not create undefined behavior.

Note that it is specified that, if `size_of::<<T as Dst>::Meta>() == 0`, then
`size_of::<*const T>()` == `size_of::<*const ()>`. This is really important for
C compatibility, because our current compatibility with flexible arrays is
really terrible.

To actually get use of the improved fat pointers, we must add four new
intrinsics (the names are up for bikeshedding):

```rust
// core
mod ptr {
    pub use intrinsics::{make_fat_ptr, make_fat_ptr_mut};
    pub fn fat_ptr_meta<T: Dst + ?Sized>(ptr: &T) -> T::Meta {
        unsafe {
            core::intrinsics::fat_ptr_meta(ptr)
        }
    }
}
mod mem {
    pub use intrinsics::size_of_prelude;
}
mod intrinsics {
    extern "rust-intrinsic" {
        fn size_of_prelude<T: ?Sized>() -> usize;
        fn fat_ptr_meta<T: ?Sized + Dst>(ptr: *const T) -> T::Meta;
        fn make_fat_ptr<T: Dst + ?Sized>(data: *const (),
            meta: U::::Meta) -> *const T;
        fn make_fat_ptr_mut<T: Dst + ?Sized>(data: *mut (),
            meta: U::::Meta) -> *mut T;
    }
}
```

This is an example implementation of `[T]`:

```rust
// The last member of a struct like this must be Unsize
#[lang = "slice"]
struct Slice<T>([T]);

unsafe impl<T> Dst for [T] {
    type Meta = usize;
    fn size_of_val(&self) -> usize {
        if self.len() > 0 {
            core::mem::size_of_val(&self[0]) * self.len()
        } else {
            0
        }
    }
}

// impl<T> Slice<T>
impl<T> [T] {
    pub fn len(&self) -> usize {
        core::intrinsics::fat_ptr_meta(self)
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

unsafe impl Dst for PixelBuffer {
    type Meta = (usize, usize);
    fn size_of_val(&self) -> usize {
        std::mem::size_of::<f32>() * self.width() * self.height()
    }
}

impl PixelBuffer {
    pub fn new_zeroed(width, height) -> Box<PixelBuffer> {
        if width > 0 {
            assert!(usize::max_value() / width > height);
        }
        let backing = vec![0.0; width * height];
        let ptr = Box::into_raw(backing.into_boxed_slice()) as *mut ();
        unsafe {
            Box::from_raw(
                std::ptr::make_fat_ptr(ptr, (width, height))
            )
        }
    }

    pub fn width(&self) -> usize {
        std::ptr::fat_ptr_meta(self).0
    }

    pub fn height(&self) -> usize {
        std::ptr::fat_ptr_meta(self).1
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
    fn size_of_val(&self) -> usize {
        std::mem::size_of_prelude::<Self>() + self.len
    }
}

impl PascalStr {
    pub fn len(&self) {
        self.len
    }
}
```

One more type which is always brought up when one talks about DSTs is the trait
object; this won't allow us to write trait objects in the standard library, or
anything, but it is interesting to look at how the compiler will see trait
objects in the context of wider DSTs:

```rust
trait Trait {
    fn func1();
}
struct TraitVtable {
    size: usize,
    func1: fn(),
}
unsafe impl<T: Trait> Unsize<Trait> for T {
    const fn unsize(&self) -> &Trait {
        static vt = TraitVtable {
            size: size_of::<T>(),
            func1: Self::func1
        };
        unsafe {
            std::ptr::make_fat_ptr(self, vt)
        }
    }
}

unsafe impl Dst for Trait { 
    type Meta = TraitVtable;
    fn size_of_val(&self) -> {
        std::ptr::fat_ptr_meta(self).size
    }
}
```

# Future Extensions
[future extensions]: #extensions

A future extension could add unsizing coercions very easily, especially with
integer generics. We could use this syntax, allowing you to define the same
struct for both the sized and unsized versions. This would make for very nice
slice-like types. However, this is not necessary for the actual RFC.

```rust
#[lang = "unsize"]
unsafe trait Unsize<T: Dst + ?Sized> {
    const fn unsize(&self) -> &T;
}
```

```rust
mod intrinsics {
    fn make_fat_ptr<U: Dst + ?Sized, T: Unsize<U>>(data: *const T,
        meta: U::::Meta) -> *const U;
    fn make_fat_ptr_mut<U: Dst + ?Sized, T: Unsize<U>>(data: *mut T,
        meta: U::::Meta) -> *mut U;
}
```

```rust
// The [const N] means that the N parameter is optional, and if left off,
// unsizing coercion shall be done
#[lang = "slice"]
struct Slice<T>[const N: usize]([T; N]);
// The last member, in these types, shall be an array, and will be turned into a
// slice for the Unsize coercion

unsafe impl<T> Unsize<[T]> for [T; N] {
    const fn unsize(&self) -> &[T] {
        unsafe {
            std::ptr::make_fat_ptr(self, N)
        }
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

I originally had an idea where the DSTs were the first class types, because I
was afraid of putting integer generics into my proposal. The current proposal
is, in my opinion, far nicer.

You could define a struct type
`PixelBuffer<'a> { width: usize, height: usize, ptr: &'a f32 }`, and a similar
PixelBufferMut struct, but this is *very* annoying, and cannot be returned by
Index traits.

# Unresolved questions
[unresolved]: #unresolved-questions

How should these fat pointers be passed, if they are larger than two pointers?

Should you be able to implement a `Dst` without an `Unsize` coercion? My guts on
yes.
