- Feature Name: `slice_ptr_range`
- Start Date: 2019-10-25
- RFC PR: None yet
- Rust Issue: None yet

# Summary
[summary]: #summary

Add `.as_ptr_range()` and `.as_mut_ptr_range()` to slices, which give
both the start and the (one-past-the-)end pointer of a slice.
(As a `Range<*const T>` or `Range<*mut T>`.)

# Motivation
[motivation]: #motivation

Many C and C++ APIs use a range of pointers instead of pointer and size to
refer to a slice of items. These functions makes it easier to obtain those two
pointers.

The alternative right now is to either use:

```rust
let start = slice.as_ptr();
let end = slice[slice.len()..].as_ptr();
some_ffi_function(start, end);
```

Or to use the (unsafe) `add` function to add the `len()` to `as_ptr()`:
The current documentation for raw pointers shows a few examples of
`vec.as_ptr().add(vec.len())`.

Both are not ideal.

With these new functions, it'd be:

```rust
let r = slice.as_ptr_range();
some_ffi_function(r.start, r.end);
```

It also allows for things like

```rust
slice.as_ptr_range().contains(some_element)
```

to see if a reference or pointer to an element points into the given slice.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

`as_ptr_range` and `as_mut_ptr_range` return the range of raw pointers spanning
the slice.

The slice is half-open, which means that the end pointer points *one past* the
last element of the slice. This way, an empty slice is represented by two equal
pointers, and the difference between the two pointers represents the size of
the size.

See `as_ptr` and `as_mut_ptr` for warnings on using these pointers.

This function is useful for interacting with foreign interfaces which use two
pointers to refer to a range of elements in memory, as is common in C++.

It can also be useful to check if a reference or pointer to an element refers
to an element of this slice:

```rust
let a = [1,2,3];
let e1 = &a[1];
let e2 = &5;
assert!(a.as_ptr_range().contains(e1));
assert!(!a.as_ptr_range().contains(e2));
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The code below is added to `impl<T> [T] { .. }` in `src/libcore/slice/mod.rs`,
with doc comments based on the [Guide-level explanation][guide-level-explanation] above.

```rust
    fn as_ptr_range(&self) -> Range<*const T> {
        let start = self.as_ptr();
        let end = unsafe { start.add(self.len()) };
        start..end
    }

    fn as_mut_ptr_range(&mut self) -> Range<*mut T> {
        let start = self.as_mut_ptr();
        let end = unsafe { start.add(self.len()) };
        start..end
    }
```

# Drawbacks
[drawbacks]: #drawbacks

It might encourage users to do more things with pointers.
(But since that happens anyway, it's probably best to have tools like this to
do it more precisely and safely.)

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

Another option is to instead have `as_end_ptr()` and `as_end_mut_ptr()`
functions, which would only give the (one-past-the-)end pointer. In combination
with `as_ptr()` and `as_mut_ptr()`, this provides the same functionality,
although less ergonomic in many cases.

It probably rarely happens somebody wants the end pointer, without also wanting the
start pointer. (And if they do, `slice.as_ptr_range().end` is only a few keystrokes away.)

# Prior art
[prior-art]: #prior-art

In C++ it is common to use two pointers (or other type of iterators) to refer
to a range of elements, instead of a start and a size.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

None yet

# Future possibilities
[future-possibilities]: #future-possibilities

It would be useful to add more functions to safely use pointer (ranges) of slices:

 - `index_of(e: &T) -> Option<usize>` which would give the index of an element
   in the slice that the reference refers to, if it points inside the slice.
   (Possibly using a `*const T` as parameter instead of a `&T`.)

   ```rust
   assert_eq!(a.index_of(&a[7]), Some(7));
   ```

 - `range_of(e: &[T]) -> Option<Range<usize>>` which would give the range of
   indexes of an subslice of this slice, or `None` if it is not a subslice of
   this slice.
   (Possibly using a `Range<*const T>` as parameter instead of a `&[T]`.)

   ```rust
   assert_eq!(a.range_of(&a[1..7]), Some(1..7));
   ```

These things are now done using raw pointers using `offset_from` or subtracting
`usize`s, [such as here][offset_example].

[offset_example]: https://github.com/rust-lang/rust/blob/18ae175d60039dfe2d2454e8e8099d3b08039862/src/libstd/path.rs#L1372-L1375
