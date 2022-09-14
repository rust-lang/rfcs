- Feature Name: `array_vec`
- Start Date: 2020-09-27
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)
- Original PR: [rust-lang/rfcs#2990](https://github.com/rust-lang/rfcs/pull/2990)

**RFC TODO** _Would a name like `BufVec`/`BufferVec` be better? This is sort of
generic across both `array`s and buffers for things like MMIO that may benefit
from the structure._

# Summary
[summary]: #summary

This RFC proposes the creation of an object to represent variable-length data
within a fixed size memory buffer, with associated methods to easily manipulate
it. The interface will mimic the most common methods of `Vec`, and this memory
buffer is an array; hence, the selected name of `ArrayVec`. This will provide
Rust with a representation of a very prevelant programming concept to enable
higher-level data manipulation without heap reliance.


# Motivation
[motivation]: #motivation

Vectors provide one of the easiest ways to work with data that may change its
length, and this is provided in Rust via `std::vec::Vec`. However, this requires
heap allocations, and this may not always be desirable in cases where:

- An allocator is not available. This is typically `no_std` environments like
  embedded, kernel, or safety-critical applications.
- A previous stack frame provides a buffer for data, and heap allocating would
  be redundant. (This is very pervasive in C which has no vector representation,
  which extends to Rust's FFI. Instead of vectors, function signatures like
  `void somefunc(buf [BUF_SIZE], int* len)` are used when a function must return
  variable-length data.)
- `Vec`-style data structures are required in `const` scopes
- Small or short-lived representations of variable data are preferred for
  performance or memory optimization
- The buffer does not represent memory, e.g. memory-mapped I/O **RFC TODO** _is
  this even worth mentioning? Could we guarantee anything that would make this
  useful in MMIO? Would it be good/better to provide a trait for `push`, `pop`,
  etc that would apply for this, some custom MMIO implementation, and `Vec`?_

While this sort of datastructure is likely to usually reside on the stack, it is
entirely possible to reside in some form on the heap within a `Box`, `Vec`, or
other structure.

Possibly the most persuasive argument for why `ArrayVec` belongs in Rust's
`core` is that bits and pieces of the language already use it. Additionally, it
would provide a pathway for easing future development instead of piecewise
re-implementing the concept as needed. Some examples:

- [`try_collect_into_array`][try_collect_arr] and its variants are used
  internally. This function wraps a `Guard` struct containing an array and a
  length that it initializes item by item. Essentially, _this is the fundamental
  structure of `ArrayVec`_, it is just not made public. Having `ArrayVec` would
  allow simplifying this function.
- The much-requested feature of some way to collect into arrays would have a
  more clear path
- Constructing a `core::ffi::CStr` is not directly possible from `&str` due to
  the extra bit. `ArrayVec` would allow for a more clear way to perform this
  common operation in `no_std` environments.
- A structure such as `ArrayString` would be posssible to enable easier string
  manipulation in `no_std` environments

In short, the benefits to an `ArrayVec` concept are notable enough that there
are already parts of the implementation in core, and there are a handful of top
100 crates that provide similar functionality. Exporsing a public `ArrayVec` in
`core` would help fragmentation, provide a pathway for future language features,
and give users a builtin tool for a common form of data manipulation.


[try_collect_arr]: https://github.com/rust-lang/rust/blob/17cbdfd07178349d0a3cecb8e7dde8f915666ced/library/core/src/array/mod.rs#L804)


# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

`ArrayVec` is a simple data structure, represented internally with a fixed-size
memory buffer (an array) and a length. It should feel very familiar to `Vec`.
The main difference to `Vec` is, the maximum capacity of that memory buffer must
be known at compile time, and is specified through a generic paramter. See the
comments in this example:

```rust
// RFC TODO: core::collections and core::ext have also been proposed
use core::array::ArrayVec;

// We are creating an `ArrayVec` here that contains an i32, and has a capacity
// of 4. That capacity cannot be changed during runtime
let mut v: ArrayVec<i32, 4> = ArrayVec::new();

// Adding values to this `ArrayVec` works almost as you would expect
// One difference is, `push()` returns a `Result<(), InsertionError>`.
// This is because there is a higher chance that the insertion may fail at runtime,
// compared to `Vec`
v.push(1).unwrap();
v.push(2).unwrap();

// Length, indexing, and end access work similarly to other data structures
assert_eq!(v.len(), 2);
assert_eq!(v[0], 1);

assert_eq!(v.pop(), Some(2));
assert_eq!(v.len(), 1);

// Indexed assignment works as expected
// **RFC TODO** what is the safe/checked way to perform assignment? It seems
// like there should be a `.set(&T) -> Result` method to go along with `get`,
// but I don't know what it is
v[0] = 7;
assert_eq!(v[0], 7);

// Many higher-order concepts work from `Vec` as well
v.extend([1, 2, 3].iter().copied());

for element in &v {
    println!("{}", element);
}
assert_eq!(v, [7, 1, 2, 3]);
```

In the above example, the `ArrayVec` is allocated on the stack, which is its
usual home (though one can be present on the heap within another type). There
are advantages and disadvantages to this, but the main thing is that the maximum
capacity of the `ArrayVec` must be known at compile time.

```rust
// `av` can store up to 64 elements
let mut v: ArrayVec<u8, 64> = ArrayVec::new();
```

As its size is known at compile time, `ArrayVec` can also be initialized within
const environments:

```rust
const MY_CONST_ARRAY_VEC: ArrayVec<i32, 10> = {
    let mut v = ArrayVec::new();
    v.push(1).unwrap();
    v.push(2).unwrap();
    v.push(3).unwrap();
    v.push(4).unwrap();
    v
};
```

The biggest downside to `ArrayVec` is, as mentioned, that its capacity cannot be
changed at runtime. For this reason, `Vec` is generally preferable unless you
know you have a case that requires `ArrayVec`.

```rust
// An example attempting to push more than 2 elements
let mut array_vec: ArrayVec<i32, 2> = ArrayVec::new();
array_vec.push(1).unwrap(); // Ok
array_vec.push(1).unwrap(); // Ok
array_vec.push(1).unwrap(); // Error!
```

In the above example, the `push()` fails because the `ArrayVec` is already full.

**RFC TODO** _I will add some more here_.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

`ArrayVec` represents a higher-level concept that is essentially a type of
collection that should be available without `std`. For this reason,
`core::collections` was chosen as its home. This does not exist yet, but may be
created with the intent that future collections may arise.

In general, the API mimics that of `Vec` for simplicity of use. However: it is
expected that there is a relatively high chance of failure `pushing` to a
fixed-length `ArrayVec`, compared to the chance of an allocation failure for
pushing to a `Vec`. For that reason, failable methods return a `Result`.

The reason behind this decision (instead of `panic!`ing) is that `ArrayVec` will
likely find common use in `no_std` systems like bare metal and kernelland. In
these environments, panicking is considered undefined behavior, so it makes
sense to guide the user toward infailable methods. (`unwrap` can easily be used
to change this behavior, at user discretion).

```rust

// The actual internal representation may vary, and should vary
pub struct ArrayVec<T, const N: usize> {
    data: MaybeUninit<[T; N]>,
    len: usize,
}

impl<T, const N: usize> ArrayVec<T, N> {
    // Constructors

    pub const fn new() -> Self;

    // Basic methods
    pub const fn insert(&mut self, idx: usize, element: T) -> Result<(), InsertionError>;

    pub const fn push(&mut self, element: T) -> Result<(), InsertionError>;

    pub const fn remove(&mut self, idx: usize) -> Result<T, RemovalErro> ;

    pub const fn pop(&mut self) -> Option<T>;

    pub const fn get(&mut self, idx: usize) -> Option<T>;

    pub const fn first(&self) -> Option<&T>

    pub const fn first_mut(&self) -> Option<&mut T>

    pub const fn last(&self) -> Option<&T>
    
    pub const fn last_mut(&self) -> Option<&mut T>

    // General methods
    // **RFC TODO** verify what makes sense to return a `Result`
    pub const fn as_mut_ptr(&mut self) -> *mut T;

    pub const fn as_mut_slice(&mut self) -> &mut [T];

    pub const fn as_ptr(&self) -> *const T;

    pub const fn as_slice(&self) -> &[T];

    pub const fn capacity(&self) -> usize;

    pub fn clear(&mut self);

    pub const fn is_empty(&self) -> bool;

    pub const fn len(&self) -> usize;

    pub fn retain<F>(&mut self, mut f: F)
    where
        F: FnMut(&mut T) -> bool;

    pub fn truncate(&mut self, len: usize);

    pub fn drain<R>(&mut self, range: R) -> Result<Drain<'_, T, N>, IndexError>
    where
        R: RangeBounds<usize>;

    pub fn extend_from_cloneable_slice<'a>(&mut self, other: &'a [T]) -> Result<(), &'a [T]>
    where
        T: Clone;

    pub fn extend_from_copyable_slice<'a>(&mut self, other: &'a [T]) -> Result<(), &'a [T]>
    where
        T: Copy;

    pub fn splice<I, R>(&mut self, range: R, replace_with: I) -> Result<Splice<'_, I::IntoIter, N>, IndexError>
    where
        I: IntoIterator<Item = T>,
        R: RangeBounds<usize>;

    pub fn split_off(&mut self, at: usize) -> Result<Self, IndexError>;

    pub fn swap_remove(&mut self, idx: usize) -> Result<T, IndexError>;

    // Maybe needed: Some sort of `from_ptr(*ptr, len)` that would ease FFI use
}
```

Traits that are implemented for `Vec` and `array` will be implemented for
`ArrayVec`, as is applicable. Unstable and deprecated methods like `reserve` or
`drain_filter` weren't considered.

**RFC todo** _We need a discussion on `FromIter`. I don't know whether it belongs
in this RFC, or would be better to mention as a future use case_

# Drawbacks
[drawbacks]: #drawbacks

### Additional complexity

New and existing users are likely to find it difficult to differentiate the
purpose of each vector type, especially those that don't have a theoretical
background in memory management. This can be mitigated by providing coherent
docs in `ArrayVec`.

### The current ecosystem is fine

`ArrayVec` is arguably not needed in `core`, as there are a handful of existing
crates to handle the problem. However, being available in `core` will add the
possiblity of Rust using the feature, which otherwise wouldn't be an option.

# Prior art
[prior-art]: #prior-art

These are the most known structures:

- `arrayvec::ArrayVec`: Uses declarative macros and an `Array` trait for
   implementations but lacks support for arbitrary sizes.
- `heapless::Vec`: With the usage of `typenum`, can support arbitrary sizes
   without a nightly compiler.
- `staticvec::StaticVec`: Uses unstable constant generics for arrays of
   arbitrary sizes.
-  `tinyvec::ArrayVec`: Supports fixed and arbitrary (unstable feature) sizes
   but requires `T: Default` to avoid unsafe `MaybeUninit`.

As seen, there isn't an implementation that stands out among the others because
all of them roughly share the same purpose and functionality. Noteworthy is the
usage of constant generics that makes it possible to create an efficient and
unified approach for arbitrary array sizes.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

### Nomenclature

`ArrayVec` will conflict with `arrayvec::ArrayVec` and `tinyvec::ArrayVec`.
`BufVec` or `BufferVec` may be alternatives.

### Macros

Macros should likely mimic `vec!`.

```rust
// Instance with 1i32, 2i32 and 3i32
let _: ArrayVec<i32, 33> = array_vec![1, 2, 3];

// Instance with 1i32 and 1i32
let _: ArrayVec<i32, 64> = array_vec![1; 2];
```

# Future possibilities
[future-possibilities]: #future-possibilities


### Generic collections and generic strings

Many structures that use `alloc::vec::Vec` as the underlying storage can also
use stack or hybrid memory, for example, an hypothetical `GenericString<S>`,
where `S` is the storage, could be split into:

```rust
type DynString<const N: usize> = GenericString<DynVec<u8, N>>;
type HeapString = GenericString<Vec<u8>>;
type StackString<const N: usize> = GenericString<ArrayVec<u8, N>>;
```
