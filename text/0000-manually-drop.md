- Feature Name: manually_drop
- Start Date: 2014-06-27
- RFC PR #: (leave this empty)
- Rust Issue #: (leave this empty)

# Summary

Provide a `ManualDrop<T>` type that stores a `T` inline in memory, but
doesn't run `T`s destructor by default.

# Motivation

There is currently no long-term way to have partially (un)initialised
inline data in types, since internal fields always have their own
destructor run. This is effectively generalising the behaviour of
`*const` and `*mut` (which do not run destructors on their contents
automatically) to inline data, i.e. not requiring allocation or other
indirection like the pointer types.

The motivating example of this is a `SmallVec<T>` type
(e.g. servo's [smallvec], or @bluss's [arrayvec]), where vector instances
with a small number of elements are stored directly inline, not on the
heap. Something like

[smallvec]: https://github.com/servo/rust-smallvec
[arrayvec]: https://github.com/bluss/arrayvec

```rust
struct SmallVec<T> {
    length: uint,
    capacity: uint,
    pointer: *mut T,
    inline: [T, .. 8]
}
```

As an example of its behaviour consider:

```rust
let mut v = SmallVec::new();
// v.length == v.capacity == 0
// v.pointer == NULL
// v.inline = [uninit, uninit, ... ]


v.push("foo".to_string());
// v.length == 1, v.capacity == 0 (or something)
// v.pointer == NULL
// v.inline = ["foo", uninit, ... ]

for _ in range(0, 99) {
    v.push("bar".to_string());
}
// v.length == 100, v.capacity >= 100
// v.pointer == some allocation containing "foo", "bar", "bar", ...
// v.inline = [uninit, uninit, ... ]
```

When a `SmallVec` with the above definition is dropped, the
destructors of all 8 `T`s in `inline` will always be run (no matter
what happens in the `Drop` implementation of `SmallVec`), leading to
dramatic unsafety: in the first and last cases, destructors would be
running on uninitialised data, and in the second, it would correctly
destroy the first element (`"foo"`) but the last 7 are uninitialised.

With the `ManuallyDrop` type given in this RFC, `SmallVec` could be
defined as

```rust
// (sketch, without `unsafe {}`s/casts etc. for clarity)

struct SmallVec<T> {
    length: uint,
    capacity: uint,
    pointer: *mut T,

    // now these 8 T's are not automatically destroyed when a SmallVec
    // goes out of scope.
    inline: ManuallyDrop<[T, .. 8]>
}

impl<T> Drop for SmallVec<T> {
    fn drop(&mut self) {
        if !self.pointer.is_null() { // heap allocated
            // same as Vec<T>'s drop
            for i in range(0, self.length) {
                ptr::read(self.pointer.offset(i));
            }
            alloc::heap::deallocate(self.pointer, ...);
        } else {
            // run destructors on the first `self.length` elements of
            // `self.inline`, but no others.
            for i in range(0, self.length) {
                ptr::read(&self.inline.get().offset(i));
            }
        }
    }
}
```

This definition is now safe: destructors run on exactly those elements
that are valid and no others.


To be precise, at the time of writing, Rust defines that destructors
are safe to run on destroyed values (and guarantees it via the so called
"drop flag"), meaning this can currently be made safe via:

```rust
impl<T> Drop for SmallVec<T> {
    fn drop(&mut self) {
        // same as above.

        // flag the whole array as dropped: it's fine for the `inner`
        // field to be redestroyed now.
        ptr::write_bytes(&mut self.inner as *mut T, mem::POST_DROP_U8, self.inner.len());
    }
}
```

However, the drop flag is likely to disappear: [#5016].

[#5016]: https://github.com/mozilla/rust/issues/5016

NB. not running destructors is now safe in Rust, so this type isn't
inherently `unsafe`, except for the fact that it is designed to store
invalid instances of other types.

# Detailed design

A new lang-item type equivalent to

```rust
pub struct ManuallyDrop<T> {
    data: T
}
```

would be provided, and the compiler would know to *not* run the
destructor of the `T` when a `ManuallyDrop<T>` instance goes out of
scope. That is,

```rust
struct X { num: int };
impl Drop for X {
    fn drop(&mut self) { println!("{}", self.num) }
}

fn main() {
    let normal = X { x: 0 };
    let new = ManuallyDrop { data: X { x: 1 } };
}
```

would print only `0`, *not* `1`.

This would be modelled after the `UnsafeCell` type, providing similar
methods:

- `const fn new(x: T) -> ManuallyDrop<T>`
- `fn get(&self) -> *const T`
- `fn get_mut(&mut self) -> *mut T`
- `unsafe fn into_inner(self) -> T`

Additional initialisation functions can be made available:

- `fn zeroed()`
- `fn uninitialized()`

(These would be `const fn` if possible, but it is not clear to me that
it is right now.)

The `ManuallyDrop` type would be a black box for representation
optimisations: it is explicitly designed to be able to store arbitrary
junk, and so the assumptions made by the compiler conventionally may
not hold (for example, the `Option` null pointer optimisation [will
break] if a pointer is null). As a concrete example:

[will break]: https://github.com/servo/rust-smallvec/issues/5

```rust
type T = Option<Box<u8>>;
type T_MD = Option<ManuallyDrop<Box<u8>>>;

assert_eq!(size_of::<T>(),     8);
assert_eq!(size_of::<T_MD>(), 16);
```

# Alternatives

- This may be adequately modeled by enums in many situations, but I
  can't imagine it will be easy to wrangle `enum`s to make `SmallVec`
  work nicely (one variant for each possible on-stack arity? How do
  you avoid adding an (unnecessary) additional work for the
  discriminant?), and especially not if we ever get generic number
  parameters, so something like `struct SmallVec<T, n> { ... inner: ManuallyDrop<[T, .. n]> }`
  would work, allowing for `SmallVec<T, 2>`, `SmallVec<T, 100>` etc.

- A struct `UninterpretedBytesOfSize<T>` equal to
  `[u8, .. size_of::<T>()]`, that is, a chunk of memory large enough
  to store a `T`, but treated as raw memory (i.e. `u8`s). This has the
  (large) downside of losing all type information, interfering with
  the compiler's reachability analysis (e.g. for `UnsafeCell`), and
  making it easier for the programmer to make mistakes w.r.t. an
  incorrect or forgotten coercion (it's would be identical to C's
  `void*`).  This is getting more feasible with [`const fn`] support.

- Change drop to have (semantically) take full ownership of its
  contents, so that `mem::forget` works, e.g. `trait Drop { fn
  drop(self); }` or a design like that in [@eddyb's comment].

- Make no change and just perform manual ownership control via
  `Option<T>`: a data type can store `Option<T>` instead of `T` with
  the invariant that the `Option` is always `Some` except for when the
  destructor runs. This allows one to implement the previous
  alternative with `take`. E.g. `ManuallyDrop` can be shimmed as
  something like:

  ```rust
  struct ManuallyDrop<T> { x: Option<T> }
  impl<T> ManuallyDrop<T> {
      const fn new(x: T) -> ManuallyDrop<T> {
          ManuallyDrop { x: Some(x) }
      }
      fn get(&self) -> *const T {
          self.x.as_ref().unwrap()
      }
      fn get_mut(&mut self) -> *mut T {
          self.x.as_ref_mut().unwrap()
      }
      unsafe fn into_inner(self) -> T {
          self.x.unwrap()
      }
  }

  impl Drop for ManuallyDrop {
      fn drop(&mut self) { mem::forget(self.x.take()) }
  }
  ```

  This approach is used by [arrayvec] but has many downsides:

  - (much) larger compiled code when not using `unsafe` (`unwrap`
    includes branches and unwinding etc.)
  - more invariants to track
  - larger data types

[`const fn`]: https://github.com/rust-lang/rfcs/blob/master/text/0911-const-fn.md
[@eddyb's comment]: https://github.com/rust-lang/rfcs/pull/197#issuecomment-110850383

# Unresolved questions

- Should a `ManuallyDrop<T>` always be `Copy`? It no longer has a
  destructor and so the only risk of double freeing (etc.) would be
  when the user writes such a thing. This would allow it to be a
  maximally flexible building-block, but I cannot think of a specific
  use-case for `ManuallyDrop<NonCopy>` to be `Copy`. Being `Copy`
  would make `ManuallyDrop` entirely the inline equivalent of `*const`
  and `*mut`, since they are both `Copy` always.

- The name.
