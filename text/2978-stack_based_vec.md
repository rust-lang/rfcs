- Feature Name: `stack_based_vec`
- Start Date: 2020-09-27
- RFC PR: [rust-lang/rfcs#2990](https://github.com/rust-lang/rfcs/pull/2990)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

This RFC, which depends and takes advantage of the upcoming stabilization of constant generics (min_const_generics), tries to propose the creation of a new vector named `ArrayVec` that manages stack memory and can be seen as an alternative for the built-in structure that handles heap-allocated memory, aka `alloc::vec::Vec<T>`.

# Motivation
[motivation]: #motivation

`core::collections::ArrayVec<T>` should be conveniently added into the standard library due to its importance and potential.

### Optimization

Stack-based allocation is generally faster than heap-based allocation and can be used as an optimization in places that otherwise would have to call an allocator. Some resource-constrained embedded devices can also benefit from it.

### Unstable features and constant functions

By adding `ArrayVec` into the standard library, it will be possible to use internal unstable features to optimize machine code generation and expose public constant functions without the need of a nightly compiler.

### Useful in the real world

`arrayvec` is one of the most downloaded project of `crates.io` and is used by thousand of projects, including Rustc itself. Currently ranks ninth in the "Data structures" category and seventy-fifth in the "All Crate" category.

### Building block

Just like `Vec`, `ArrayVec` is also a primitive vector where high-level structures can use it as a building block. For example, a stack-based matrix or binary heap.

### Unification

There are a lot of different crates about the subject that tries to do roughly the same thing, a centralized implementation would stop the current fragmentation.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

`ArrayVec` is a container that encapsulates fixed size buffers. 

```rust
let mut v: ArrayVec<i32, 4> = ArrayVec::new();
v.push(1);
v.push(2);

assert_eq!(v.len(), 2);
assert_eq!(v[0], 1);

assert_eq!(v.pop(), Some(2));
assert_eq!(v.len(), 1);

v[0] = 7;
assert_eq!(v[0], 7);

v.extend([1, 2, 3].iter().copied());

for element in &v {
    println!("{}", element);
}
assert_eq!(v, [7, 1, 2, 3]);
```

Instead of relying on a heap-allocator, stack-based memory is added and removed on-demand in a last-in-first-out (LIFO) order according to the calling workflow of a program. `ArrayVec` takes advantage of this predictable behavior to reserve an exactly amount of uninitialized bytes up-front to form an internal buffer.

```rust
// `array_vec` can store up to 64 elements
let mut array_vec: ArrayVec<i32, 64> = ArrayVec::new();
```

Another potential use-case is the usage within constant environments:

```rust
const MY_CONST_ARRAY_VEC: ArrayVec<i32, 10> = {
    let mut v = ArrayVec::new();
    v.push(1);
    v.push(2);
    v.push(3);
    v.push(4);
    v
};
```

Of course, fixed buffers lead to inflexibility because unlike `Vec`, the underlying capacity can not expand at run-time and there will never be more than 64 elements in the above example.

```rust
// This vector can store up to 0 elements, therefore, nothing at all
let mut array_vec: ArrayVec<i32, 0> = ArrayVec::new();
array_vec.push(1); // Error!
```

A good question is: Should I use `core::collections::ArrayVec<T>` or `alloc::vec::Vec<T>`? Well, `Vec` is already good enough for most situations while stack allocation usually shines for small sizes.

* Do you have a known upper bound?

* How much memory are you going to allocate for your program? The default values of `RUST_MIN_STACK` or `ulimit -s` might not be enough.

* Are you using nested `Vec`s? `Vec<ArrayVec<T, N>>` might be better than `Vec<Vec<T>>`.

```
let _: Vec<Vec<i32>> = vec![vec![1, 2, 3], vec![4, 5]];

 +-----+-----+-----+
 | ptr | len | cap |
 +--|--+-----+-----+
    |
    |   +---------------------+---------------------+----------+
    |   |       Vec<i32>      |       Vec<i32>      |          |
    |   | +-----+-----+-----+ | +-----+-----+-----+ |  Unused  |
    '-> | | ptr | len | cap | | | ptr | len | cap | | capacity |
        | +--|--+-----+-----+ | +--|--+-----+-----+ |          |
        +----|----------------+----|----------------+----------+
             |                     |
             |                     |   +---+---+--------+
             |                     '-> | 4 | 5 | Unused |
             |                         +---+---+--------+
             |   +---+---+---+--------+
             '-> | 1 | 2 | 3 | Unused |
                 +---+---+---+--------+

Illustration credits: @mbartlett21
```

Can you see the `N`, where `N` is length of the external `Vec`, calls to the heap allocator? In the following illustration, the internal `ArrayVec`s are placed contiguously in the same space.

```txt
let _: Vec<ArrayVec<i32, 3>> = vec![array_vec![1, 2, 3], array_vec![4, 5]];

 +-----+-----+-----+
 | ptr | len | cap |
 +--|--+-----+-----+
    |
    |   +------------------------------+--------------------------+----------+
    |   |       ArrayVec<i32, 4>       |     Arrayvec<i32, 4>     |          |
    |   | +-----+---+---+---+--------+ | +-----+---+---+--------+ |  Unused  |
    '-> | | len | 1 | 2 | 3 | Unused | | | len | 4 | 5 | Unused | | capacity |
        | +-----+---+---+---+--------+ | +-----+---+---+--------+ |          |
        +------------------------------+--------------------------+----------+

Illustration credits: @mbartlett21
```

Each use-case is different and should be pondered individually. In case of doubt, stick with `Vec`.

For a more technical overview, take a look at the following operations:

```rust
// `array_vec` has a pre-allocated memory of 2048 bits (32 * 64) that can store up
// to 64 signed integers.
let mut array_vec: ArrayVec<i32, 64> = ArrayVec::new();

// Although reserved, there isn't anything explicitly stored yet
assert_eq!(array_vec.len(), 0);

// Initializes the first 32 bits with a simple '1' integer or
// 00000000 00000000 00000000 00000001 bits
array_vec.push(1);

// Our vector memory is now split into a 32/2016 pair of initialized and
// uninitialized memory respectively
assert_eq!(array_vec.len(), 1);
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

`ArrayVec` is a contiguous memory block where elements can be collected, therefore, a collection by definition and even though `core::collections` doesn't exist, it is the most natural module placement.

To avoid length and conflicting conversations, the API will mimic most of the current `Vec` surface, which also means that all methods that depend on valid user input or valid internal capacity will panic at run-time when something goes wrong. For example, removing an element that is out of bounds.

```rust
// Please, bare in mind that these methods are simply suggestions. Discussions about the
// API should probably take place elsewhere.

pub struct ArrayVec<T, const N: usize> {
    data: MaybeUninit<[T; N]>,
    len: usize,
}

impl<T, const N: usize> ArrayVec<T, N> {
    // Constructors

    pub const fn new() -> Self;

    // Infallible Methods

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

    // Methods that can panic at run-time

    pub fn drain<R>(&mut self, range: R) -> Drain<'_, T, N>
    where
        R: RangeBounds<usize>;

    pub fn extend_from_cloneable_slice<'a>(&mut self, other: &'a [T])
    where
        T: Clone;

    pub fn extend_from_copyable_slice<'a>(&mut self, other: &'a [T])
    where
        T: Copy;

    pub const fn insert(&mut self, idx: usize, element: T);

    pub const fn push(&mut self, element: T);

    pub const fn remove(&mut self, idx: usize) -> T;

    pub fn splice<I, R>(&mut self, range: R, replace_with: I) -> Splice<'_, I::IntoIter, N>
    where
        I: IntoIterator<Item = T>,
        R: RangeBounds<usize>;

    pub fn split_off(&mut self, at: usize) -> Self;

    pub fn swap_remove(&mut self, idx: usize) -> T;

    // Verifiable methods
    
    pub const fn pop(&mut self) -> Option<T>;
}
```

Meaningless, unstable and deprecated methods like `reserve` or `drain_filter` weren't considered. A concrete implementation is available at https://github.com/c410-f3r/stack-based-vec.

# Drawbacks
[drawbacks]: #drawbacks

### Additional complexity

New and existing users are likely to find it difficult to differentiate the purpose of each vector type, especially people that don't have a theoretical background in memory management.

### The current ecosystem is fine

`ArrayVec` might be an overkill in certain situations. If someone wants to use stack memory in a specific application, then it is just a matter of grabbing the appropriated crate.

# Prior art
[prior-art]: #prior-art

These are the most known structures:

 * `arrayvec::ArrayVec`: Uses declarative macros and an `Array` trait for implementations but lacks support for arbitrary sizes.
 * `heapless::Vec`: With the usage of `typenum`, can support arbitrary sizes without a nightly compiler.
 * `staticvec::StaticVec`: Uses unstable constant generics for arrays of arbitrary sizes.
 * `tinyvec::ArrayVec`: Supports fixed and arbitrary (unstable feature) sizes but requires `T: Default` for security reasons.

As seen, there isn't an implementation that stands out among the others because all of them roughly share the same purpose and functionality. Noteworthy is the usage of constant generics that makes it possible to create an efficient and unified approach for arbitrary array sizes.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

### Verifiable methods

Unlike methods that will abort the current thread execution, verifiable methods will signal that something has gone wrong or is missing. This approach has two major benefits:

- `Security`: The user is forced to handle possible variants or corner-cases and enables graceful program shutdown by wrapping everything until `fn main() -> Result<(), MyCustomErrors>` is reached.

- `Flexibility`: Gives freedom to users because it is possible to choose between, for example, `my_full_array_vec.push(100)?` (check), `my_full_array_vec.push(100).unwrap()` (panic) or `let _ = my_full_array_vec.push(100);` (ignore).

In regards to performance, since the upper capacity bound is known at compile-time and the majority of methods are `#[inline]`, the compiler will probably have the necessary information to remove most of the conditional bounding checking when producing optimized machine code.

```rust
pub fn drain<R>(&mut self, range: R) -> Option<Drain<'_, T, N>>
where
    R: RangeBounds<usize>;

pub fn extend_from_cloneable_slice<'a>(&mut self, other: &'a [T]) -> Result<(), &'a [T]>
where
    T: Clone;

pub fn extend_from_copyable_slice<'a>(&mut self, other: &'a [T]) -> Result<(), &'a [T]>
where
    T: Copy;

pub const fn insert(&mut self, idx: usize, element: T) -> Result<(), T>;

pub const fn push(&mut self, element: T) -> Result<(), T>;

pub const fn remove(&mut self, idx: usize) -> Option<T>;

pub fn splice<I, R>(&mut self, range: R, replace_with: I) -> Option<Splice<'_, I::IntoIter, N>>
where
    I: IntoIterator<Item = T>,
    R: RangeBounds<usize>;

pub fn split_off(&mut self, at: usize) -> Option<Self>;

pub fn swap_remove(&mut self, idx: usize) -> Option<T>;
```

In my opinion, every fallible method should either return `Option` or `Result` instead of panicking at run-time. Although the future addition of `try_*` variants can mitigate this situation, it will also bring additional maintenance burden.

### Nomenclature

`ArrayVec` will conflict with `arrayvec::ArrayVec` and `tinyvec::ArrayVec`.

### Prelude

Should it be included in the prelude?

### Macros

```rust
// Instance with 1i32, 2i32 and 3i32
let _: ArrayVec<i32, 33> = array_vec![1, 2, 3];

// Instance with 1i32 and 1i32
let _: ArrayVec<i32, 64> = array_vec![1; 2];
```

# Future possibilities
[future-possibilities]: #future-possibilities

### Dynamic array

An hybrid approach between heap and stack memory could also be provided natively in the future.

```rust
pub struct DynVec<T, const N: usize> {
    // Hides internal implementation
    data: DynVecData,
}

impl<T, const N: usize> DynVec<T, N> {
    // Much of the `Vec` API goes here
}

// This is just an example. `Vec<T>` could be `Box` and `enum` an `union`.
enum DynVecData<T, const N: usize> {
    Heap(Vec<T>),
    Inline(ArrayVec<T, N>),
}
```

The above description is very similar to what `smallvec` already does.

### Generic collections and generic strings

Many structures that use `alloc::vec::Vec` as the underlying storage can also use stack or hybrid memory, for example, an hypothetical `GenericString<S>`, where `S` is the storage, could be split into:

```rust
type DynString<const N: usize> = GenericString<DynVec<u8, N>>;
type HeapString = GenericString<Vec<u8>>;
type StackString<const N: usize> = GenericString<ArrayVec<u8, N>>;
```
