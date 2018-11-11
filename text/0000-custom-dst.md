- Feature Name: `custom_dst`
- Start Date: 2018-11-06
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Allow Rust code to define dynamically sized types with custom thick (and thin)
pointers, and define slice functions in terms of these, instead of transmute.
Also, convert the `CStr` type to use this functionality,
and make it a thin pointer; this will allow use with FFI.

# Motivation
[motivation]: #motivation

As of right now, the lack of custom DSTs in Rust means that we can't communicate
with C in important ways - we lack the ability to define a `CStr` in the
language such that `&CStr` is compatible with `char const *`,
and we lack the ability to communicate nicely with C code that uses
[Flexible Array Members](https://en.wikipedia.org/wiki/Flexible_array_member).
This RFC attempts to fix this,
as well as introduce more correctness to existing practices.

Apart from FFI, it also has usecases for indexing and slicing 2-d arrays.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

There's a new language trait in the standard library, similar to `Sized`,
under `std::marker`:

```rust
unsafe trait DynamicallySized {
    type Metadata: 'static + Copy + Send + Sync + Unpin;

    fn size_of_val(&self) -> usize;
    fn align_of_val(&self) -> usize;
}
```

with an automatic implementation for all `Sized` types:

```rust
unsafe impl<T> DynamicallySized for T {
    type Metadata = ();

    fn size_of_val(&self) -> usize { size_of::<T>() }
    fn align_of_val(&self) -> usize { align_of::<T>() }
}
```

If you have a type which you would like to be unsized,
you can implement this trait for your type!

```rust
#[repr(C)]
struct CStr([c_char; 0]);

unsafe impl DynamicallySized for CStr {
    type Metadata = ();

    fn size_of_val(&self) -> usize { strlen(&self.0 as *const c_char) }
    fn align_of_val(&self) -> usize { 1 }
}
```

and automatically, your type will not implement `Sized`.

The existing `DynamicallySized` types will continue to work;
if one writes a `DynamicallySized` type `T`,
and then wraps `T` into a struct, they'll get the obvious semantics.

```rust
struct Foo {
    x: usize,
    y: CStr,
}

// size_of_val(&foo) returns size_of::<usize>() + size_of_val(&foo.y)
// same with align_of_val
```


# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

In addition to the explanation given above,
we will also introduce three functions into the standard library,
in `core::raw`, which allow you to create and destructure these
pointers to `DynamicallySized` types:

```rust
mod core::raw {
    pub fn from_raw_parts<T: DynamicallySized>(
        ptr: *const (),
        meta: <T as DynamicallySized>::Metadata,
    ) -> *const T;

    pub fn from_raw_parts_mut<T: DynamicallySized>(
        ptr: *mut (),
        meta: <T as DynamicallySized>::Metadata,
    ) -> *mut T;

    pub fn metadata<T: DynamicallySized>(
        ptr: *const T,
    ) -> <T as DynamicallySized>::Metadata;
}
```

and we will introduce two functions into `core::mem`,
to help people write types with Flexible Array Members:

```rust
mod core::mem {
    pub fn size_of_header<T: ?Sized>() -> usize;
    pub fn align_of_header<T: ?Sized>() -> usize;
}
```
These functions return the size and alignment of the header of a type;
or, the minimum possible size and alignment, in other words.
For existing `Sized` types, they are equivalent to `size_of` and `align_of`,
and for existing DSTs,

```rust
assert_eq!(size_of_header::<[T]>(), 0);
assert_eq!(align_of_header::<[T]>(), align_of::<T>());
assert_eq!(size_of_header::<dyn Trait>(), 0);
assert_eq!(align_of_header::<dyn Trait>(), 1);

// on 64-bit
struct RcBox<T: ?Sized> {
  strong: Cell<usize>,
  weak: Cell<usize>,
  value: T,
}
assert_eq!(size_of_header::<RcBox<dyn Trait>>(), 16);
assert_eq!(align_of_header::<RcBox<dyn Trait>>(), 8);
```

Note that this is a minimum - this means that for `extern type`s,
they return `0` and `1` respectively.

Notes:
  - names of the above functions should be bikeshed
  - `extern type`s do not implement `DynamicallySized`, although in theory one
    could choose to implement the trait for them
    (that usecase is not supported by this RFC).
  - `T: DynamicallySized` bounds imply a `T: ?Sized` bound.
    - The opposite is not true - `T: ?Sized` implies `T: ?DynamicallySized`

We will also change `CStr` to have the implementation from above.

On an ABI level, we promise that pointers to any type with

```rust
size_of::<Metadata>() == 0
&& align_of::<Metadata>() <= align_of::<*const ()>()
```

are ABI compatible with a C pointer - this is important,
since we want to be able to write:

```rust
extern "C" {
    fn printf(fmt: &CStr, ...) -> c_int;
}
```

Unfortunately, we won't be able to change existing declarations in `libc`
without a new major version.

`as` casts continue to allow

```rust
fn cast_to_thin<T: DynamicallySized, U: Sized>(t: *const T) -> *const U {
    t as *const U
}
```

so we do not introduce any new functions to access the pointer part
of the thick pointer.

The `DynamicallySized` trait may be implemented for a struct or union
-- The author is of the opinion that implementing it for `enum`s is
unlikely to be useful. That may be a future extension,
if people are interested
(once `<T: ?Sized>` is allowed on `enum` declarations).

`DynamicallySized` will be placed into the prelude.

# Drawbacks
[drawbacks]: #drawback

- More complication in the language.
- Lack of a `Sized` type dual to these unsized types --
  the lack of a `[u8; N]` to these types' `[u8]` is unfortunate.
- Inability to define a custom DST safely
- The `size_of_val` and `align_of_val` declarations are now incorrect;
  they should take `T: DynamicallySized`, as opposed to `T: ?Sized`.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

This has been a necessary change for quite a few years.
The only real alternatives are those which are simply different ways of writing
this feature. We need custom DSTs.

We also likely especially want to deprecate the current `?Sized` behavior of
`size_of_val` and `align_of_val`, since people are planning on
aborting/panicking at runtime when called on extern types. That's not great.
([link](https://github.com/rust-lang/rfcs/pull/2310#issuecomment-384770802))

# Prior art
[prior-art]: #prior-art

- [FAMs in C](https://en.wikipedia.org/wiki/Flexible_array_member)
- [FAMs in C++](https://htmlpreview.github.io/?https://github.com/ThePhD/future_cxx/blob/master/papers/d1039.html) (unfinished proposal)
- Existing Rust which could use this feature:
  - [CStr](https://doc.rust-lang.org/stable/std/ffi/struct.CStr.html)
  - [Pascal String](https://github.com/ubsan/epsilon/blob/master/src/string.rs#L11)
  - [Bit Vector](https://github.com/skiwi2/bit-vector/blob/master/src/bit_slice.rs)
- Other RFCs
  - [mzabaluev's Version](https://github.com/rust-lang/rfcs/pull/709)
  - [My Old Version](https://github.com/rust-lang/rfcs/pull/1524)
  - [japaric's Pre-RFC](https://github.com/japaric/rfcs/blob/unsized2/text/0000-unsized-types.md)
  - [mikeyhew's Pre-RFC](https://internals.rust-lang.org/t/pre-erfc-lets-fix-dsts/6663)
  - [MicahChalmer's RFC](https://github.com/rust-lang/rfcs/pull/9)
  - [nrc's Virtual Structs](https://github.com/rust-lang/rfcs/pull/5)
  - [Pointer Metadata and VTable](https://github.com/rust-lang/rfcs/pull/2580)
  - [Syntax of ?Sized](https://github.com/rust-lang/rfcs/pull/490)
- [Niko's Blog on DSTs](http://smallcultfollowing.com/babysteps/blog/2014/01/05/dst-take-5/)

(you will note the incredible number of RFCs on this topic -- we really need to fix this missing feature)

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- Bikeshedding names.
- Should `Metadata` really require all of those traits?
- Should `T: ?Sized` still imply `T: DynamicallySized`?
  - Introduce a new `T: ?DynamicallySized`.
  - If we change this, then `std::mem::{size,align}_of_val` are no longer broken
  - As well as all standard library types
    - `Box<extern-type>` is currently allowed
    - (despite being useless)
    - I'm pretty close to convinced of this being the better solution
- Should we allow implementing `DynamicallySized` for `extern type`s or `enum`s?

# Future possibilities
[future-possibilities]: #future-possibilities

- By overloading `DerefAssign`, we could add a `BitReference` type

# Examples
[more examples]: #examples

Put here at the end for ease of finding 😁

### Non-trivial types

For non-trivial types (i.e., those that have a destructor),
Rust generates the obvious destructor from the definition of the type itself -
i.e., if you hold a `Vec<T>` in your type, Rust will destroy it.
However, if your type contains additional data that Rust doesn't know about,
you'll have to destroy it yourself.

```rust
#[repr(C)] // we need this to be laid out linearly
struct InlineVec<T> {
    capacity: usize,
    len: usize,
    buffer: [T; 0], // for offset, alignment, and dropck
}

unsafe impl<T> DynamicallySized for InlineVec<T> {
    type Metadata = ();

    fn size_of_val(&self) -> usize {
        Self::full_size(self.capacity)
    }
    fn align_of_val(&self) -> usize {
        std::mem::align_of_header::<Self>()
    }
}
impl<T> Drop for InlineVec<T> {
    fn drop(&mut self) {
        std::mem::drop_in_place(self.as_mut_slice());
    }
}

impl<T> InlineVec<T> {
    // internal
    fn full_size(cap: usize) -> usize {
        std::mem::size_of_header::<Self>() + cap * std::mem::size_of::<T>()
    }

    pub fn new(cap: usize) -> Box<Self> {
        let size = Self::full_size(cap);
        let align = std::mem::align_of_header::<Self>();
        let layout = std::alloc::Layout::from_size_align(size, align).unwrap();
        let ptr = std::raw::from_raw_parts_mut(
            std::alloc::alloc(layout) as *mut (),
            (),
        );
        std::ptr::write(&mut ptr.capacity, cap);
        std::ptr::write(&mut ptr.len, 0);
        Box::from_raw(ptr)
    }

    pub fn len(&self) -> usize {
        self.len
    }
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn as_ptr(&self) -> *const T {
        &self.buff as *const [T; 0] as *const T
    }
    pub fn as_mut_ptr(&mut self) -> *mut T {
        &mut self.buff as *mut [T; 0] as *mut T
    }

    pub fn as_slice(&self) -> &[T] {
        unsafe {
            std::slice::from_raw_parts(self.as_ptr(), self.len())
        }
    }
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        unsafe {
            std::slice::from_raw_parts(self.as_mut_ptr(), self.len())
        }
    }

    // panics if it doesn't have remaining capacity
    pub fn push(&mut self, el: T) {
        assert!(self.size() < self.capacity());
        let ptr = self.as_mut_ptr();
        let index = self.len();
        std::ptr::write(ptr.offset(index as isize), el);
        self.len += 1;
    }

    // panics if it doesn't have any elements
    pub fn pop(&mut self) -> T {
        assert!(self.len() > 0);
        self.len -= 1;
        let ptr = self.as_mut_ptr();
        let index = self.len();
        std::ptr::read(ptr.offset(index as isize))
    }
}
```

### Flexible Array Members

Communicating with C types that contain flexible array members
is an important part of this RFC.

```rust
// note: a real example from winapi
#[repr(C)]
struct TOKEN_GROUPS {
    GroupCount: DWORD,
    Groups: [SID_AND_ATTRIBUTES; 0],
}

impl DynamicallySized for TOKEN_GROUPS {
    type Metadata = ();

    fn size_of_val(&self) -> usize {
        std::mem::size_of_header::<Self>()
        + self.GroupCount * std::mem::size_of::<SID_AND_ATTRIBUTES>()
    }

    fn align_of_val(&self) -> usize {
        std::mem::align_of_header::<Self>()
    }
}

extern "system" {
    pub fn AdjustTokenGroups(
        TokenHandle: HANDLE,
        ResetToDefault: BOOL,
        NewState: &mut TOKEN_GROUPS,
        BufferLength: DWORD,
        PreviousState: Option<&mut TOKEN_GROUPS>,
        ReturnLength: &mut DWORD,
    ) -> BOOL;
}
```
