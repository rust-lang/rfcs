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

The most important addition to the language shall be one new trait, a
modification of the Unsize trait, and a bit of new syntax (introduced later):

```rust
#[lang = "dst"]
unsafe trait Dst {
    type Meta: Copy;
    fn size_of_val(&self) -> usize;
}

#[lang = "unsize"]
unsafe trait Unsize<T: Dst + ?Sized> {
    const fn unsize(&self) -> &T;
}
```

The `Dst` trait should be unsafe, because it's very easy to give an incorrect
implementation of `size_of_val`, and safe and unsafe code *must* rely on it
being correct to not create undefined behavior.

Note that it is specified that, if `size_of::<<T as Dst>::Meta>() == 0`, then
`size_of::<*const T>()` == `size_of::<*const ()>`. This is really important for
C compatibility, because our current compatibility with flexible arrays is
really terrible.

The `Unsize` trait must be unsafe for much the same reasons.

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
        fn make_fat_ptr<U: Dst + ?Sized, T: Unsize<U>>(data: *const T,
            meta: U::::Meta) -> *const U;
        fn make_fat_ptr_mut<U: Dst + ?Sized, T: Unsize<U>>(data: *mut T,
            meta: U::::Meta) -> *mut U;
    }
}
```

These shall, very importantly, be closely tied to integer generics, and will not
be viable until integer generics are in the language. Fortunately, integer
generics are coming soon™.

```rust
// The [const N] means that the N parameter is optional, and if left off,
// unsizing coercion shall be done
#[lang = "slice"]
struct Slice<T>[const N: usize]([T; N]);
// The last member of a struct like this must be Unsize

unsafe impl<T, const N: usize> Unsize<[T]> for [T; N] {
    const fn unsize(&self) -> &[T] {
        unsafe {
            std::ptr::make_fat_ptr(self, N)
        }
    }
}

unsafe impl<T> Dst for Slice<T> {
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
        unsafe {
            core::intrinsics::fat_ptr_meta(self)
        }
    }

    pub fn as_ptr(&self) -> *const T {
        unsafe {
            &self.0 as *const [T] as *const T
            // importantly, you still have access to the last member of the
            // struct; it is the Output of Unsize on the last member
        }
    }

    pub unsafe fn from_raw_parts(buf: *const T, len: usize) -> &[T] {
        unsafe {
            &*core::intrinsics::make_fat_ptr(buf as *const [T; 1], len)
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
struct PixelBuffer[const W: usize, const H: usize]([f32; W * H]);

unsafe impl<const W: usize, const H: usize> Unsize<PixelBuffer> for PixelBuffer[W, H] {
    const fn unsize(&self) -> &PixelBuffer {
        unsafe {
            std::ptr::make_fat_ptr(self, (W, H))
        }
    }
}
impl Dst for PixelBuffer {
    type Meta = (usize, usize);
    fn size_of_val(&self) -> usize {
        std::mem::size_of::<f32>() * self.width() * self.height()
    }
}

impl<const W: usize, const H: usize> PixelBuffer[W, H] {
    const fn from_array(arr: [[f32; W]; H]) -> Self {
        Self(arr)
    }
}

impl PixelBuffer {
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

// ---

fn main() {
    let pixels: Box<PixelBuffer> = Box::new(PixelBuffer::from_array([[0.0; 1280]; 960);
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
        unsafe {
            std::ptr::fat_ptr_meta(self).size
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

We likely need an equivalent to the C `sizeof(unsized)`; what should that
function be?
