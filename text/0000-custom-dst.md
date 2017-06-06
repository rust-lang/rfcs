- Feature Name: `custom_dst`
- Start Date: 02 March, 2016
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Allow Rust code to define dynamically sized types with custom fat pointers, and
define slice functions in terms of these, instead of transmute.

# Motivation
[motivation]: #motivation

Many standard Rust features rely on references. The `Deref*` and `Index*` traits
are two major features where references are used to create lvalues on the other
side. Unfortunately, what can be a reference is very much restricted by the
language.

Let's look at a DST which is already in the language: `[T]`. There are three
main types which are connected to `[T]`: Heap (`Vec<T>`), Sized (`[T; n]`), and
Borrowed (`[T]`). We already have the ability to write the Heap part of custom
DSTs right now, and it looks something like this:

```rust
struct Vec<T> {
  ptr: Unique<T>,
  length: usize,
  capacity: usize,
}
```

The Borrowed and Sized types are both defined inside the compiler. The Heap type
is the easy part of the story. Being able to define your own is a capability in
the vast majority of languages. What this RFC attempts to do is allow defining
the Borrowed type inside user code. In current Rust, `[T]`'s special handling is
really nice when you want to deal with slices: they're very easy to use,
thankfully, for such a major feature. It means, on the other hand, that no one
else can make their own `[T]`-alikes. Let's look now at the original motivating
example:

Owned:

```rust
struct PixelBuffer {
  ptr: Unique<f32>,
  width: usize,
  height: usize,
}
```

Borrowed:

```rust
struct Pixels {
  // ?
}
```

The Borrowed type is undefineable. What you really want is `Pixels` to be a two
dimensional "slice". However, the important thing is that `Pixels` is, itself,
not a pointer. As `[T]` is just a block of memory, `Pixels` should just be a
block of memory. The crazy stuff happens in the pointer to `Pixels`, which you
want to be a {pointer, width, stride, height}. So, how do we do that?

Note: we can't just make a new struct for things like this, something like:

```rust
// represents an &Pixels from above
struct Pixels {
  ptr: *const f32,
  width: usize,
  stride: usize,
  height: usize,
}

// represents an &mut Pixels from above
struct PixelsMut {
  ptr: *mut f32,
  width: usize,
  stride: usize,
  height: usize,
}
```

because `Index` and `Deref` return references.

# Detailed design
[design]: #detailed-design

The real center of this RFC is `unsafe impl !Sized for T`:

```rust
// not technically a trait, but it looks enough like one
unsafe trait !Sized {
  // this is the "metadata"; for [T], it would be a usize. For Pixels, it
  // would be a struct { width: usize, stride: usize, height: usize }
  type Meta: Copy;
  // should eventually (before stabilization) be const fn
  // returns the number of contiguous bytes readable through this type
  // equivalently, how much a Box<T> must allocate for that value of T
  // for example:
  // size_of_val::<Pixels>(p) // size_of::<f32>() * height * stride
  fn size_of_val(&self) -> usize;
}
```

It's unsafe to implement `!Sized` because it's so easy to create undefined
behavior with an incorrectly implemented `size_of_val`.

If `size_of::<<T as DynamicallySized>::Meta>()` is zero, then `size_of::<&T>()`
shall be equal to `size_of::<&()>()`.

These three intrinsics shall be added to the language:

```rust
mod intrinsics {
  extern "rust-intrinsic" {
    // shall return the size of the beginning part of the struct, without the
    // dynamic data. This is equivalent to C's sizeof(struct with flexible
    // array member). If it's called on a Sized type T, it shall be equivalent
    // to size_of::<T>()
    fn size_of_prelude<T: ?Sized>() -> usize;
    // shall return the metadata of a fat pointer value. For example, with &[T],
    // returns the length
    fn fat_ptr_meta<T: !Sized>(ptr: *const T) -> T::Meta;
    // creates a fat pointer from it's requisite parts.
    fn make_fat_ptr<T: !Sized>(data: *const (), meta: T::Meta) -> *const T;
  }
}
```

And shall be stabilized (eventually) as:

```rust
mod ptr {
  pub unsafe fn make_fat_ptr<T: !Sized>(
      data: *const (), meta: T::Meta) -> *const T {
    intrinsics::make_fat_ptr(data, meta)
  }

  pub fn fat_ptr_meta<T: !Sized>(ptr: &T) -> T::Meta {
    unsafe {
      core::intrinsics::fat_ptr_meta(ptr)
    }
  }
}
mod mem {
  pub fn size_of_prelude<T: ?Sized>() -> usize {
    unsafe {
      core::intrinsics::size_of_prelude::<T>()
    }
  }
}
```

This is an example implementation of `[T]`:

```rust
// The last field of a !Sized type must be zero sized
// You may use it as the "jumping off point" for indexing into your block of
// memory
#[lang = "slice"]
struct Slice<T>([T; 0]);

unsafe impl<T> !Sized for [T] {
  type Meta = usize;
  fn size_of_val(&self) -> usize {
    mem::size_of::<T>() * self.len()
  }
}

// impl<T> Slice<T>
impl<T> [T] {
  pub fn len(&self) -> usize {
    ptr::fat_ptr_meta(self)
  }

  pub fn as_ptr(&self) -> *const T {
    unsafe {
      &self.0 as &[T] as *const [T] as *const T
    }
  }

  pub unsafe fn from_raw_parts(buf: *const T, len: usize) -> &[T] {
    unsafe {
      &*ptr::make_fat_ptr(buf as *const (), len)
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

And the following is an example of a custom Borrow type:

```rust
struct PixelBuffer {
  ptr: *const f32,
  width: usize,
  stride: usize,
  height: usize
}

struct Pixels([f32; 0]);

#[derive(Copy, Clone)]
struct PixelMeta {
  width: usize,
  stride: usize,
  height: usize
}

unsafe impl !Sized for Pixels {
  type Meta = PixelMeta;
  fn size_of_val(&self) -> usize {
    std::mem::size_of::<f32>() * self.stride() * self.height()
  }
}

impl Deref for PixelBuffer {
  type Target = Pixels;

  fn deref(&self) -> &Pixels {
    unsafe {
      &*ptr::make_fat_ptr(self.ptr as *const (),
        PixelMeta {
          width: self.width,
          stride: self.width,
          height: self.height
        })
    }
  }
}

impl DerefMut for PixelBuffer {
  fn deref_mut(&mut self) -> &mut Pixels {
    unsafe {
      &mut *ptr::make_fat_ptr(self.ptr as *const (),
        PixelMeta {
          width: self.width,
          stride: self.width,
          height: self.height
        }) as *mut _
    }
  }
}

impl Pixels {
  pub fn from_raw_parts(ptr: *const f32, 
      width: usize, stride: usize, height: usize) -> &Pixels {
    &*std::ptr::make_fat_ptr(ptr as *const (),
      PixelMeta { width: width, stride: stride, height: height })
  }

  pub fn width(&self) -> usize {
    std::ptr::fat_ptr_meta(self).width
  }

  pub fn stride(&self) -> usize {
    std::ptr::fat_ptr_meta(self).stride
  }

  pub fn height(&self) -> usize {
    std::ptr::fat_ptr_meta(self).height
  }
}

impl Index<Range<usize>> for Pixels {
  type Output = PixelBuffer;

  fn index(&self, idx: Range<usize>) -> &Pixels {
    assert!(idx.start <= idx.end);
    assert!(idx.end <= self.);
    unsafe {
      Pixels::from_raw_parts(
        self.as_ptr().offset(index.start as isize),
        index.end - index.start,
        self.stride,
        self.height)
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
existentials, like a rectangular window into one of those `PixelBuffer`s (as
seen with `Pixels`).

We could use an unsized type as the last type, instead of a zero sized type.

# Unresolved questions
[unresolved]: #unresolved-questions

How should these fat pointers be passed, if they are larger than two pointers?

What should the last type actually be? A zero sized type seems the least bad.
