- Feature Name: array_builder
- Start Date: 2021-05-26
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

A data structure to allow building a `[T; N]` dynamically. Safely handling drops and efficiently being convertable into the underlying `[T; N]`.

# Motivation
[motivation]: #motivation

Array initialisation is surprisingly unsafe. The safest way is to initialise with a default value, then replacing the values.
This is not always possible and requires moving to using MaybeUninit and unsafe. This is very easy to get wrong.

For example:
```rust
let mut array: [MaybeUninit<String>; 4] = MaybeUninit::uninit_array();
array[0].write("Hello".to_string());
panic!("some error");
```

Despite being completely safe, this will cause a memory leak. This is because `MaybeUninit` does not call `drop` for the string.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

## Definition

This RFC proposes a new struct, `ArrayBuilder`. It has a very basic API designed solely for building new `[T; N]` types without initialising it all before hand.
This is not a heapless replacement for Vec.

```rust
pub struct ArrayBuilder<T, const N: usize> {
    // hidden fields
}

// ArrayBuilder implements drop safely, and prevents any memory leaks
impl<T, const N: usize> Drop for ArrayBuilder<T, N> {}

impl<T, const N: usize> ArrayBuilder<T, N> {
    /// Create a new uninitialized ArrayBuilder
    pub fn new() -> Self;

    /// Adds the value onto the end of the ArrayBuilder
    pub fn push(&mut self, t: T); // Panics if array is full
    pub fn try_push(&mut self, t: T) -> Result<(), T>; // Returns Err(t) if array is full
    pub unsafe fn push_unchecked(&mut self, t: T); // UB if array is full

    /// Complements push, added for consistency
    pub fn pop(&mut self) -> Option<T>; // Returns None if array is empty
    pub unsafe fn pop_unchecked(&mut self) -> T; // UB if array is empty

    /// Gets the current length of the ArrayBuilder
    pub fn len(&self) -> usize;
    pub fn is_full(&self) -> bool;
    pub fn is_empty(&self) -> bool;

    /// If the ArrayBuilder is full, returns the successfully initialised array
    pub fn build(self) -> Result<[T; N], Self>; // returns Err(self) if not full
    pub unsafe fn build_unchecked(self) -> [T; N]; // UB if not full

    /// Returns the ArrayBuilder, leaving behind an empty
    /// ArrayBuilder in it's place
    pub fn take(&mut self) -> Self;
}

// Implements AsRef/AsMut for slices. These will return references to
// any initialised data. Useful if extracting data when the ArrayBuilder is not yet full
impl<T, const N: usize> Deref for ArrayBuilder<T, N> { type Target = [T]; }
impl<T, const N: usize> DerefMut for ArrayBuilder<T, N>;
```

## Example uses

A very simple demonstration:

```rust
let mut arr = ArrayBuilder::<String, 4>::new();

arr.push("a".to_string());
arr.push("b".to_string());
arr.push("c".to_string());
arr.push("d".to_string());

let arr: [String; 4] = arr.build().unwrap();
```

If you want the first 10 square numbers in an array:

```rust
let mut arr = ArrayBuilder::<usize, 10>::new();
for i in 1..=10 {
    arr.push(i*i);
}
arr.build().unwrap()
```

A simple iterator that can iterate over blocks of `N`:

```rust
struct ArrayIterator<I: Iterator, const N: usize> {
    builder: ArrayBuilder<I::Item, N>,
    iter: I,
}

impl<I: Iterator, const N: usize> Iterator for ArrayIterator<I, N> {
    type Item = [I::Item; N];

    fn next(&mut self) -> Option<Self::Item> {
        for _ in self.builder.len()..N {
            // If the underlying iterator returns None
            // then we won't have enough data to return a full array
            // so we can bail early and return None
            self.builder.push(self.iter.next()?);
        }
        // At this point, we must have N elements in the builder
        // So extract the array and reset the builder for the next call
        self.builder.take().build()
    }
}

impl<I: Iterator, const N: usize> ArrayIterator<I, N> {
    pub fn remaining(&self) -> &[I::Item] {
        &self.builder
    }
}
```


# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

A pretty clear example could be porting relevant features from [arrayvec::ArrayVec](https://github.com/bluss/arrayvec/blob/master/src/arrayvec.rs).

Example basic implementation (does not cover the entire API suggested):

```rust
pub struct ArrayBuilder<T, const N: usize> {
    buf: [MaybeUninit<T>; N],
    len: usize,
}

impl<T, const N: usize> Drop for ArrayBuilder<T, N> {
    fn drop(&mut self) {
        self.clear()
    }
}

impl<T, const N: usize> Deref for ArrayBuilder<T, N> {
    type Target = [T];
    fn deref(&self) -> &[T] {
        unsafe { slice::from_raw_parts(self.as_ptr(), self.len) }
    }
}

impl<T, const N: usize> DerefMut for ArrayBuilder<T, N> {
    fn deref_mut(&mut self) -> &mut [T] {
        unsafe { slice::from_raw_parts_mut(self.as_mut_ptr(), self.len) }
    }
}

impl<T, const N: usize> ArrayBuilder<T, N> {
    pub fn clear(&mut self) {
        let s: &mut [T] = self;
        unsafe { ptr::drop_in_place(s); }
        self.len = 0;
    }

    fn as_ptr(&self) -> *const T {
        self.buf.as_ptr() as _
    }

    fn as_mut_ptr(&mut self) -> *mut T {
        self.buf.as_mut_ptr() as _
    }

    pub unsafe fn push_unchecked(&mut self, t: T) {
        ptr::write(self.as_mut_ptr().add(self.len), t);
        self.len += 1;
    }

    pub unsafe fn build_unchecked(self) -> [T; N] {
        let self_ = ManuallyDrop::new(self);
        ptr::read(self_.as_ptr() as *const [T; N])
    }
}
```

# Drawbacks
[drawbacks]: #drawbacks

As suggested above. There already exists several crates that _can_ implement this functionality. [arrayvec](https://crates.io/crates/arrayvec) has over 20 million downloads
and is moving to version 1.0 soon.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

Many of the other implementations are focused on this pattern being for 'heapless' Vec.
This is a reasonable desire, especially in `no_std` use cases and low memory embedded
systems. This is not the case for this proposal. Instead specifically related to initiating
arrays dynamically then building them into full arrays.

Another reason to have this in core, is that there are already some PRs for adding new features
to `core::array`. For example, [`from_fn`](https://github.com/rust-lang/rust/pull/75644) or [`FromIterator`](https://github.com/rust-lang/rust/issues/81615). It would be nice to have a re-usable, safe backing for them. And then it might as well be stablised and exposed to others too.

# Prior art
[prior-art]: #prior-art

## [array-init crate](https://crates.io/crates/array-init):

This is good for basic tasks, but it has only a very minimal API. It only allows
for creating an array in one big go. There's no way to extract the data out of a partial
fill.

## [arrayvec crate](https://crates.io/crates/arrayvec):

Discussed above, this crate focuses on the heapless `Vec` aspect. It can be used to implement
array initialisation, but that doesn't seem to be it's primary intention.

## [ArrayVec/StackVec RFC](https://github.com/rust-lang/rfcs/pull/2990):

Simply put, this appears to be a re-implementation of `arrayvec::ArrayVec` in core.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- What should the exact API be?

# Future possibilities
[future-possibilities]: #future-possibilities

Once implemented in `core`, some current implementations and PRs could be updated to use it.
