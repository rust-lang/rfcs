- Feature Name: `array_vec`
- Start Date: 2020-09-27
- RFC PR: [rust-lang/rfcs#3316](https://github.com/rust-lang/rfcs/pull/3316)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)


# Summary
[summary]: #summary

This RFC proposes the creation of an object to represent variable-length data
within a fixed size memory buffer, with associated methods to easily manipulate
it. The interface will mimic the most common methods of `Vec`, and this memory
buffer is an array; hence, the selected name of `ArrayVec`. This will provide
Rust with a representation of a very prevelant programming concept to enable
higher-level data manipulation without heap reliance, as well as create a
backend to simplify implementation of various concepts in `core`.


# Motivation
[motivation]: #motivation

Vectors provide one of the easiest ways to work with data that does not have a
fixed length, and an API to do this is provided in Rust via `std::vec::Vec`.
However, vectors requires heap allocations that may not be desirable (or
possible) in cases where:

- An allocator is not available. This is typically `no_std` environments like
  embedded, kernel, or safety-critical applications.
- A previous stack frame provides a buffer for data, and heap allocating would
  be redundant. This is common in C, which has no vector type, so function
  signatures like `void somefunc(int buf[BUF_SIZE], int* len)` are used when a
  function must return variable-length data. `ArrayVec` would provide a
  convenient wrapper for these representations, bolstering the ease of use of
  Rust's FFI.
- `Vec`-style data structures are required in `const` scopes
- Small or short-lived representations of variable data are preferred for
  performance or memory optimization
- The buffer does not represent actual memory, e.g. memory-mapped I/O

_(**RFC TODO** is the last item even worth mentioning? Could we guarantee
anything that would make this useful in MMIO? Would it be good/better to provide
a trait for `push`, `pop`, etc that would apply for this, some custom MMIO
implementation, and `Vec`?)_

While this sort of datastructure will usually reside on the stack, it is
entirely possible to be placed on the heap within something like a `Box` or
`Vec`.

Possibly the most persuasive argument for why `ArrayVec` belongs in Rust's
`core` is that bits and pieces of the language already use it. Additionally, it
would provide a pathway for easing future development instead of piecewise
re-implementing the concept as needed. Some examples:

- [`try_collect_into_array`][try_collect_arr] and its variants are used
  internally. This function wraps a `Guard` struct containing a `MaybeUninit`
  array and a length that it initializes item by item. Essentially, _this is the
  fundamental structure of `ArrayVec`_, it is just not made public. Having
  `ArrayVec` would allow simplifying this function and others like it.
- The much-requested feature of some way to collect into arrays would have a
  more clear path, potentially by making `try_collect_into_array` public
- Constructing a `core::ffi::CStr` is not directly possible from `&str` due to
  the extra bit needed. `ArrayVec` would allow for a more clear way to perform
  this common operation in `no_std` environments.
- A structure such as `ArrayString` would be posssible to enable easier string
  manipulation in `no_std` environments

In short, the benefits to an `ArrayVec` concept are notable enough that there
are already parts of the implementation in core, and there are a handful of top
100 crates that provide similar functionality. Exporsing a public `ArrayVec` in
`core` would help reduce fragmentation, provide a pathway for future language
features, and give users a builtin tool for a common form of data manipulation.


[try_collect_arr]: https://github.com/rust-lang/rust/blob/17cbdfd07178349d0a3cecb8e7dde8f915666ced/library/core/src/array/mod.rs#L804


# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

`ArrayVec` is a simple data structure, represented internally with a fixed-size
memory buffer (an array) and a length. It should feel very familiar to `Vec`.
The main difference to `Vec` is, the maximum capacity of that memory buffer must
be known at compile time, and is specified through a generic paramter. See the
comments in this example:

```rust
use core::collections::ArrayVec;

const ARR_LEN: usize = 4;

// We are creating an `ArrayVec` here that contains instances of i32, and has a capacity
// of 4. That capacity cannot be changed during runtime
let mut v: ArrayVec<i32, ARR_LEN> = ArrayVec::new();

// Adding values to this `ArrayVec` works almost as you would expect
// One difference is, `push()` returns a `Result<(), InsertionError>`.
// This is because there is a much higher chance that the insertion may fail at
// runtime (running out of space on the buffer) compared to `Vec`
v.push(1).unwrap();
v.push(2).unwrap();

// Length and indexing work similarly to other data structures
assert_eq!(v.len(), 2);
assert_eq!(v[0], 1);
assert_eq!(v.pop(), Some(2));
assert_eq!(v.len(), 1);

// Indexed assignment works as expected
// **RFC TODO** what is the safe/checked way to perform assignment? It seems
// like there should be a `.set(&T) -> Result` method to go along with `get`,
// but I don't know what it is (probably just missing something)
v[0] = 7;
assert_eq!(v[0], 7);

// Many higher-order concepts from `Vec` work as well
v.extend([1, 2, 3].iter().copied());

// `ArrayVec` can also be iterated
for element in v {
    println!("{}", element);
}

v.iter_mut().for_each(|x| *x += 2);

// And can be cloned and compared
assert_eq!(v, v.clone());

// Comparisons to standard arrays also work
assert_eq!(v, [7, 1, 2, 3]);
```

In the above example, the `ArrayVec` is allocated on the stack, which is its
usual home (though one can be present on the heap within another type). There
are advantages and disadvantages to this, but the main thing to keep in mind is
that the maximum capacity of the `ArrayVec` must be known at compile time.

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

This will also implement macros that mirror `vec!`:

```rust
// Instantiate an i32 ArrayVec with capacity 40 and elements 1, 2, and 3
let _: ArrayVec<i32, 40> = array_vec![1, 2, 3];

// Instantiate an i32 ArrayVec with capacity 64, and 4 instances of `1`
let _: ArrayVec<i32, 64> = array_vec![1; 4];
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

    // Basic methods similar to `Vec`
    pub const fn insert(&mut self, idx: usize, element: T) -> Result<(), InsertionError>;

    pub const fn push(&mut self, element: T) -> Result<(), InsertionError>;

    pub const fn remove(&mut self, idx: usize) -> Result<T, RemovalErro> ;

    pub const fn pop(&mut self) -> Option<T>;

    pub const fn get(&mut self, idx: usize) -> Option<T>;

    pub const fn first(&self) -> Option<&T>

    pub const fn first_mut(&self) -> Option<&mut T>

    pub const fn last(&self) -> Option<&T>
    
    pub const fn last_mut(&self) -> Option<&mut T>

    pub fn clear(&mut self);

    // General methods
    pub const fn as_slice(&self) -> &[T];
    
    pub const fn as_mut_slice(&mut self) -> &mut [T];
    
    pub const fn as_ptr(&self) -> *const T;

    pub const fn as_mut_ptr(&mut self) -> *mut T;

    pub const fn capacity(&self) -> usize;

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

    // **RFC TODO** Is this name apropriate? Should const 
    // This is designed to easily map to C's `void somefunc(int buf[BUF_SIZE], int* len)`
    pub unsafe fn from_raw_parts_mut<'a, T, const N: usize>(
        data: *mut [T; N],
        len: usize,
    ) -> &'a mut Self<T, N>

}
```

Traits that are implemented for `Vec` and `array` will be implemented for
`ArrayVec`, as is applicable. These may include:

- `AsMut<[T]>`
- `AsRef<[T]>`
- `Borrow<[T]>`
- `Clone`
- `Debug` (creates an empty `ArrayVec`)
- `Deref`
- `DerefMut`
- `Drop`
- `Extend`
- `FromIterator` _**RFC Note** this needs discussion_
- `Hash`
- `Index`
- `IndexMut`
- `IntoIterator`
- `Ord`
- `PartialEq` 
- `TryFrom` _**RFC Note** probably need to use this wherever possible instead of `From`_

The list of traits above is a tall list, and it is likely to require some
pruning based on what is possible, and what takes priority.

# Drawbacks
[drawbacks]: #drawbacks

One drawback is that new (and existing) users are likely to find it difficult to
differentiate the purpose of each vector type, especially those that don't have
a theoretical background in memory management. This can be mitigated by
providing coherent docs in `ArrayVec` that indicate `Vec` is to be preferred.

The main drawback with anything new is that adding _any_ code adds a maintenance
overhead. The authors of the RFC consider this to nevertheless be a worthwhile
addition because it simplifies design patterns used not only by external users,
but also by `Rust` itself.


# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

_**RFC TODO** More can be added to this section once an interface is decided
upon_

# Prior art
[prior-art]: #prior-art

Similar concepts have been implemented in crates:

- `smallvec::SmallVec` Smallvec uses an interface that predates const generics,
  i.e. `SmallVec<A: Array>` with `Array` implemented for `[T; 0]`, `[T; 1]`,
  etc. It allows overflowing of the data onto the heap.
- `arrayvec::ArrayVec`: Similar to the implementation described here, quite
  popular but unfortunately about a year out of maintenance
- `heapless::Vec`: Similar to the implementation described here, also includes
  many other nonallocating collections.
-  `tinyvec::ArrayVec`: Provides similar features to the described
   implementation using only safe code. Has features 
- `staticvec::StaticVec`: Similar features to the described implementation,
  generally regarded as the most performant crate (so should be observed for
  implementation guidelines)
- Work in `core` as described in [motivation](#motivation)

# Unresolved questions
[unresolved-questions]: #unresolved-questions

### Generic Interface

Thom Chiovoloni suggested an alternate interface based around `ArrayVec<[T]>`
and `ArrayVec<[T; N]>` [in a comment on the original pull
request](https://github.com/rust-lang/rfcs/pull/2990#issuecomment-848962572)
that allows for some code deduplication in assembly. The generics API is not
quite as elegant or clear as `<T, const N: usize>`, but the benefits are worth
investigating.

### Slice Backing

A more generic representation of an `ArrayVec` could be something that is
slice-backed, rather than array-backed. This could be quite powerful and is
worth looking into because it would allow using the `ArrayVec` API in places
where size is not known at compile time. For example: allocations in heap,
chunks of a stack-based buffer, FFI buffers where length is passed as an
argument, or really any arbitrary slice.

Developing a clean API for this concept that still allows initializing as an
array is difficult. The "Generic Interface" suggestion directly above may
provide a solution via `ArrayVec<&[T]>`, or perhaps an enum-based option could
work.

If slices are acceptable backing, something like `BufVec` would likely be a
better name. Additional methods along the lines of the following could be added
(generics and lifetimes omitted for brevity):

```rust
/// Creates a zero-length ArrayVec that will be located on the provided slice
fn from_slice(buf: &mut [T]) -> Self

/// Create an `ArrayVec` on a slice with a specified length of elements
/// 
/// Safety: this function is not unsafe within Rust as all slices always contain
/// valid data. However, if the slice is coming from an external FFI, note that
/// the first `len` items of `buf _must_ contain valid data, otherwise undefined
/// behavior is possible.
fn from_slice_with_len(buf: &mut [T], len: usize) -> Self
```

# Future possibilities
[future-possibilities]: #future-possibilities


### `ArrayVec`-backed `StringVec`

A simple extension of `ArrayVec<T, N>` would be `StringVec<N>`, an array of
`u8`s. This would greatly simplify string manipulation options when a heap is
not available


### Easier interface between `CStr` and `&str`

`ArrayVec` would allow for a function that enables converting between `&str` and
`CStr` by providing a fixed-size buffer to write the `&str` and the terminating
`\0`.
