- Feature Name: `subslice_offset`
- Start Date: 2019-10-27
- RFC PR: None yet
- Rust Issue: None yet

# Summary
[summary]: #summary

Add functions to the slice primitive to get the offset/index of an element or
subslice based on their memory addresses:

```rust
impl<T> [T] {
    pub fn index_of(&self, element: &T) -> Option<usize>;
    pub fn range_of(&self, subslice: &[T]) -> Option<Range<usize>>;
}
```

# Motivation
[motivation]: #motivation

In many cases, one might end up with a reference to some element in&mdash;or
some part of&mdash;a slice.
For example, after using `split` or `chunks` on a slice, using some searching
algorithm from a external crate, etc.

Some of these, like `matches`, have a counterpart that also gives the indices of
the parts, like `match_indices`. However, many don't.

The proposed functions make it easy to safely get indices in these cases,
without having to resort to manual bookkeeping or (unsafe) pointer math.

For example, for `split`: instead of adding a separate `[T]::split_indices`,
`range_of` could be used to get the indices of the split results.

Even when all standard library functions giving references to elements or
subslices would have a variant giving the indices as well, we can't expect all
crates to do this. `index_of` and `range_of` provide a safe way to get an index
when all you have is a reference (and the original slice).

Also, in some cases, using this functionality would allow for more efficient
code, as the best alternative is often manual bookkeeping: keeping track of the
offset/index in addition to the reference, which may take extra time and space.

#### Use Case Example 1: Avoid Pointer Math

In the implementation of `PathBuf::set_extension`,
[pointer math is used][offset_example] to calculate the index of the end
of the `Path::file_stem()` in the path, to find the position where the new
extension should be added. Right now this involves casting raw pointers to
`usize`s and subtracting them, making the code non-trivial and error-prone:

```rust
let end_file_stem = file_stem[file_stem.len()..].as_ptr() as usize;
let start = os_str_as_u8_slice(&self.inner).as_ptr() as usize;
(...)
v.truncate(end_file_stem.wrapping_sub(start));
```

With the proposed functions, it'd look like this:

```rust
let file_stem_range = os_str_as_u8_slice(&self.inner)
    .range_of(file_stem)
    .expect("file_stem() gave a slice outside of the path");
(...)
v.truncate(file_stem_range.end);
```

[offset_example]: https://github.com/rust-lang/rust/blob/18ae175d60039dfe2d2454e8e8099d3b08039862/src/libstd/path.rs#L1372-L1375

#### Use Case Example 2: Increase Efficiency

For this example, take this struct representing some token in a tokenized source
file:

```rust
struct Token<'a> {
  (...)
  source: &'a str, // the exact source of this token
  offset_in_source_file: usize, // for error reporting
}
```

If the `source` always points into the full `String` contents of the file,
keeping `offset_in_source_file` is a waste of space, as that information is
already encoded in the memory address stored in `source`:
`file_contents.range_of(token.source)`.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

## index_of

```rust
impl<T> [T] {
    pub fn index_of(&self, element: &T) -> Option<usize>
}
```

Returns the index of the element that a reference refers to.

If the given reference points inside the slice, returns the index of
the element it refers to. If the reference does not refer to within the
slice, `None` is returned instead.

If an index is returned, it is less than `self.len()`.

Note that this does not look at the value, but only at the reference.
If you want to find the index of an element equal to a given value, use
`iter().position()` instead.

```rust
let a = [0; 5];

assert_eq!(a.index_of(&a[2]), Some(2));
assert_eq!(a.index_of(&3), None);
```

## range_of

```rust
impl<T> [T] {
    pub fn range_of(&self, subslice: &[T]) -> Option<Range<usize>>
}
```

Returns the range referred to by a subslice.

If the given slice falls entirely within this slice, returns the range
of indices the subslice refers to. If the given slice is not a subslice
of this slice, `None` is returned.

If a range is returned, both ends are less than or equal to `self.len()`.

Note that this does not look at the contents of the slice, but only at
the memory addresses.

```rust
let a = [0; 5];

assert_eq!(a.range_of(&a[2..5]), Some(2..5));
assert_eq!(a.range_of(&[7, 8, 9]), None);
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The functions below are added to `impl<T> [T] { .. }` in `src/libcore/slice/mod.rs`,
with doc comments based on the [Guide-level explanation][guide-level-explanation] above.

```rust
    pub fn index_of(&self, element: &T) -> Option<usize> {
        let element = element as *const _;
        let range = self.as_ptr_range();
        if range.contains(&element) {
            unsafe { Some(element.offset_from(range.start) as usize) }
        } else {
            None
        }
    }

    pub fn range_of(&self, subslice: &[T]) -> Option<Range<usize>> {
        let range = self.as_ptr_range();
        let subrange = subslice.as_ptr_range();
        if subrange.start >= range.start && subrange.end <= range.end {
            unsafe {
                Some(Range {
                    start: subrange.start.offset_from(range.start) as usize,
                    end: subrange.end.offset_from(range.start) as usize,
                })
            }
        } else {
            None
        }
    }
```

# Drawbacks
[drawbacks]: #drawbacks

- It might encourage users to use this in places where better alternatives
  are available. For example: `.iter().find()` in combination with `.index_of()`
  instead of `.iter().position()`.

- In most parts of the library, two references to different but equal objects
  behave the same. The memory address itself being significant instead of the
  value it refers to is somewhat uncommon.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

This functionality is already available through calculations with raw pointers.
Often, people prefer to avoid that, which in many cases leads to complicating
the code in other ways.

The main consideration is which things should have a safe and ergonomic option,
and which are best left to raw pointer manipulation.

The proposed functions put the line between references and pointers:
the functions only work on references; as soon as
you're using raw pointers, you'll have to do things manually instead.

The alternative is to try to cover some cases for raw pointers as well:

## Raw Pointers

By requiring a `&T` or a `&[T]` in these functions, we needlessly restrict them
to only work when they point to a valid object, which needs to be properly
borrowed at that point. Since only the memory addresses are looked at, and not
the values, it might be good to provide this functionality for raw pointer
(ranges). This can be done in different ways:

#### Option 1: Take pointers instead of references

```rust
pub fn index_of(&self, element: *const T) -> Option<usize>;
pub fn range_of(&self, subslice: Range<*const T>) -> Option<Range<usize>>;
```

In the case of `index_of` this is a small change. Since a `&T` coerces to a
`*const T`, this doesn't change much in the sense that it can still be used the
same for references.
This is not memory unsafe (since we only look at the address and not at the
value), but it might lead to subtle bugs when keeping pointers across potential
`Vec` reallocations, for example.

In the case of `range_of` this is a bigger change.
It could either take a `*const [T]` or a `Range<*const T>`:

- In the first case, a `&[T]` can be used just like 'before', as it is coerced
  to a `*const [T]` automatically. However, there's no proper way to construct
  such a 'fat pointer' manually, making it not very useful for anything other
  than taking a `&[T]`, except that it lifts the borrowing/lifetime
  requirements.

- In the second case, a user would have to explicitly convert a `&[T]` to a
  `Range<*const T>` using `.as_ptr_range()`, making the code more verbose and
  less clear:

  ```rust
  assert_eq!(a.range_of(a[2..5].as_ptr_range()), Some(2..5));
  ```

#### Option 2: Add a second set of functions

To keep the ergonomics of taking regular references/slices in the proposed
functions, it might be a good idea to keep these functions, and have two extra
functions for raw pointers:

```rust
pub fn index_of(&self, element: &T) -> Option<usize>;
pub fn range_of(&self, subslice: &[T]) -> Option<Range<usize>>;
pub fn ptr_index_of(&self, element: *const T) -> Option<usize>;
pub fn ptr_range_of(&self, subslice: Range<*const T>) -> Option<Range<usize>>;
```

#### Option 3: Use a trait `AsPtrRange`

As a way to not have different functions for pointer ranges and slices, but
keep the ergonomics of being able to pass a `&[T]` as the subslice, a trait
could be used to accept both `Range`s and `&[T]`s:

```rust
pub trait AsPtrRange {
    type Element;
    fn as_ptr_range(&self) -> Range<*const Self::Element>;
}
impl<T> AsPtrRange for &[T] { ... }
impl<T> AsPtrRange for Range<*const T> { ... }

impl<T> [T] {
  ...
  pub fn range_of<S>(&self, subslice: S) -> Option<Range<usize>>
  where
    S: AsPtrRange<Element = T>
  { ... }
}
```

#### Option 4: Add this functionality to `Range<*const T>`

Instead of, or in addition to, adding this functionality to slices,
it can be added to `Range<*const T>`.
In this case, the user will have to explicitly call `.as_ptr_slice()`
on the slice, and then ask the returned `Range` for the offset of a pointer or
another `Range` inside of it:

```rust
impl<T> Range<*const T> {
    pub fn index_of(&self, element: *const T) -> Option<usize>;
    pub fn indices_of(&self, subrange: Range<*const T>) -> Option<Range<usize>>;
}
```

If used instead of the functionality directly on slices, usage would look like:

```rust
assert_eq!(a.as_ptr_range().index_of(&a[2]), Some(2));
assert_eq!(a.as_ptr_range().indices_of(a[2..5].as_ptr_range()), Some(2..5));
```

Very verbose, but it does provide a clearer hint that pointer math is involved.

The downside is that we lose the guarantees of a `&[T]`, and can no longer make
assumptions such as `start <= end` or about the maximum size of a slice
(which is needed to safely use `pointer::offset_from`).

# Prior art
[prior-art]: #prior-art

In C it is very common to do these type of calculations on pointers, although
manually instead of through a library function.
For example, after `strstr` returns a pointer to a location in a string,
it is rather common to subtract a pointer to the start of the string to get the
offset of the found substring.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- What are the best names for these functions?

  The wrong names might suggest these functions do something different.

  Alternative name ideas:

  - `index_of` / `range_of`
  - `offset_of` / `offset_of_slice`
  - `offset_of` / `offset_range_of`
  - `get_index` / `get_indices`
  - `get_offset` / `get_offsets`
  - `ptr_offset_of` / `ptr_range_of`
  - ...

- Should this functionality also be there for raw pointers, and if so, how?

  See the [Raw Pointers section](#raw-pointers) above.

- Should this be added for `&str` as well?

  This would for example allow using `s.range_of(part)` to get the indices
  of a part of the string given by `s.split(',')`.

  Without, it'd look like `s.as_bytes().range_of(part.as_bytes())`.
