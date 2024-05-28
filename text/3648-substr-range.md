- Feature Name: substr_range
- Start Date: 2024-05-28
- RFC PR: [rust-lang/rfcs#3648](https://github.com/rust-lang/rfcs/pull/3648)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Add the `substr_range`, `subslice_range`, and `elem_offset` methods. Unlike `str::find` and similar methods, these do not search the `slice`/`str` and instead use safe pointer arithmetic to find where something is within a `slice`/`str` based on its memory address.

```rust
impl str {
    fn substr_range(&self, substr: &str) -> Option<Range<usize>>;
}
impl<T> [T] {
    fn subslice_range(&self, subslice: &[T]) -> Option<Range<usize>>;
    fn elem_offset(&self, element: &T) -> Option<usize>;
}
```


# Motivation
[motivation]: #motivation

The most important use case of this is with iterators like `str::lines`, `str::split`, and `slice::split`. The author of this RFC often finds themself having to avoid methods like `str::split` because of their inflexibility. This inflexibility also exists when using similar methods from external crates.
### Example
This is a function in rustdoc. Specifically, it's in [`src/librustdoc/markdown.rs`](https://github.com/rust-lang/rust/blob/4dfe7a16cd8dff6bef0eea989ae98f4e9d8910fc/src/librustdoc/markdown.rs#L29). This function puts the leading lines that start with `%` into a `Vec`. It then returns this `Vec` and the remaining string. As you can see, it has this `count` variable that keeps track of how many bytes `s.lines()` has gone through.
```rust
/// Separate any lines at the start of the file that begin with `%`.
fn extract_leading_metadata<'a>(s: &'a str) -> (Vec<&'a str>, &'a str) {
    let mut metadata = Vec::new();
    let mut count = 0;
    for line in s.lines() {
        if line.starts_with("%") {
            // remove %<whitespace>
            metadata.push(line[1..].trim_left());
            count += line.len() + 1;
        } else {
            return (metadata, &s[count..]);
        }
    }
    // if we're here, then all lines were metadata % lines.
    (metadata, "")
}
```

There are several issue with this. First of all, it adds unneeded complexity and overhead. Additionally, this code doesn't work if the file has [CRLF](https://en.wikipedia.org/wiki/Newline#Representation) line breaks because those take up two bytes instead of one.

This `count` approach may also be entirely unfeasable when using other iterators, such as `str::split_whitespace` or `str.split(',').filter(|s| !s.is_empty())`.

However, the methods proposed in this RFC would allow us to entirely avoid the `count` variable and to properly handle CRLF line breaks. Instead, we could do this:
```rust
/// Separate any lines at the start of the file that begin with `%`.
fn extract_leading_metadata<'a>(s: &'a str) -> (Vec<&'a str>, &'a str) {
    let mut metadata = Vec::new();
    for line in s.lines() {
        if line.starts_with("%") {
            // remove %<whitespace>
            metadata.push(line[1..].trim_left());
        } else {
            let line_idx = s.substr_range(line).unwrap();
            return (metadata, &s[line_idx.start..]);
        }
    }
    // if we're here, then all lines were metadata % lines.
    (metadata, "")
}
```

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

## `str` methods
### substr_range

```rust
impl<T> [T] {
    pub fn substr_range(&self, substr: &str) -> Option<Range<usize>>
}
```

Returns the range of indices referred to by a `str` based on its memory address.

If `substr` falls entirely within `self`, returns the range of indices `substr` refers to otherwise returns `None`.

If a range is returned, `start` and `end` are less than or equal to `self.len()`.

Note that this does not look at the contents of `self` and `substr`. It only looks at the memory addresses.

```rust
let a = "hey, hello world!";
let second_he = a.matches("he").nth(1).unwrap();

assert_eq!(a.substr_range(second_he), Some(5..7));
assert_eq!(a.substr_range("he"), None);
```

## `slice` methods

### elem_offset

```rust
impl<T> [T] {
    pub fn elem_offset(&self, element: &T) -> Option<usize>
}
```

Returns the index of an element based on its memory address.

If `element` points inside `self`, returns the index of `element` otherwise returns `None`.

If an index is returned, it is less than `self.len()`.

Note that this does not look at the value of `element`. It only looks at its memory address. If you want to find the index of an element equal to a given value, use `iter().position()` instead.

```rust
let a = [0u32; 5];

assert_eq!(a.elem_offset(&a[2]), Some(2));
assert_eq!(a.elem_offset(&0u32), None);
```

### subslice_range

```rust
impl<T> [T] {
    pub fn subslice_range(&self, subslice: &[T]) -> Option<Range<usize>>
}
```

Returns the range referred to by a subslice.

If `subslice` falls entirely within `self`, returns the range of indices `subslice` refers to otherwise returns `None`.

If a range is returned, `start` and `end` are less than or equal to `self.len()`.

Note that this does not look at the contents of the slice. It only looks at its memory address.

```rust
let a = [0; 5];

assert_eq!(a.subslice_range(&a[2..5]), Some(2..5));
assert_eq!(a.subslice_range(&[7, 8, 9]), None);
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

```rust
impl str {
    fn substr_range(&self, substr: &str) -> Option<Range<usize>> {
        self.as_bytes().subslice_range(substr.as_bytes())
    }
}

impl<T> [T] {
    fn subslice_range(&self, subslice: &[T]) -> Option<Range<usize>> {
        let self_start = self.as_ptr() as usize;
        let subslice_start = subslice.as_ptr() as usize;

        let start = subslice_start.wrapping_sub(self_start) / core::mem::sizeof::<T>();
        let end = start + subslice.len();

        if start <= self.len() && end <= self.len() {
            Some(start..end)
        } else {
            None
        }
    }

    fn elem_offset(&self, element: &T) -> Option<usize> {
        let self_start = self.as_ptr() as usize;
        let elem_start = element as *const T as usize;

        let byte_offset = elem_start.wrapping_sub(self_start);
        let offset = byte_offset / core::mem::size_of::<T>();

        if offset < self.len() {
            Some(offset)
        } else {
            None
        }
    }
}
```

### ZSTs
With the above implementation, `elem_offset` and `subslice_range` will panic if they're used with ZSTs.

# Drawbacks
[drawbacks]: #drawbacks

- These methods may be confusing for people coming from high level languages.
- The signature of these methods may cause people to confuse them with `str::find` or `slice.iter().position()`. This [actually happened](https://github.com/rust-lang/rust/commit/b3aa1a6d4ac88f68e036a05fdf19be63b522b65d#diff-f606ed7686845628c1cf3a99af4c184d8bb533d235230c86548386e549ae7675L37) with the example laid out in the "Motivation" section (NOTE: the old code uses the now deprecated `subslice_offset` which is analogous to `substr_range().unwrap().start`).

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

This is a good addition because it's a simple way to extend `str::split` and friends. On top of extending `str::split` and similar methods, it also has [other, more niche, uses](https://stackoverflow.com/questions/50781561/how-to-find-the-starting-offset-of-a-string-slice-of-another-string).

### Alternatives
 - Add methods, such as `split_indices` or `matches_indices` that return ranges instead of subslices.
   - This adds a bunch of new methods to the standard library. This is especially bad if you look at all of the variants of these methods (eg `str::rsplit`, `str::split_inclusive`, `slice::rsplitn`, and `str::rsplit_terminator`).
 - Use a crates.rs crate such as [subslice_offset](https://docs.rs/subslice-offset/latest/subslice_offset/index.html).
   - These methods are very simple and powerful and are worth having in the standard library.
   - Having these in the standard library may encourage people to use `str::split` more.
 - Not using `str::split` if you need more complex behavior.
   - `str::split` and methods like `str::lines` properly deal with certain edge cases (such as trailing separators or CRLF), so they lead to more concise and less buggy code.


# Prior art
[prior-art]: #prior-art

- In languages like C, doing pointer arithmetic like this is both trivial and common.
- [subslice_offset](https://docs.rs/subslice-offset/latest/subslice_offset/index.html) crate.
- [original `subslice_offset` PR from 2013](https://github.com/rust-lang/rust/pull/5823).
  - [deprecated here](https://github.com/rust-lang/rust/commit/b3aa1a6d4ac88f68e036a05fdf19be63b522b65d#diff-cc3d2c2b93569a4e3ce58a7a53e3bb443e77a9cd2e4015af53d0152f2f8e3183R1648) with `str::find` being implied as the alternative.
- https://github.com/rust-lang/rfcs/pull/2796 (an RFC similar to this but only for slices). The PR for this was abandoned because the author did not have time to make suggested changes. This current RFC uses a lot of content from this original RFC.
- https://stackoverflow.com/questions/50781561/how-to-find-the-starting-offset-of-a-string-slice-of-another-string (illustrates a more niche use for this method).

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- What should be done in the case of zero-sized types? The current implementations panic, but maybe `None` should be returned instead?
- Should these methods return `Option`s or should they panic instead? Returning `Option`s is more versatile but is also more verbose.

# Future possibilities
[future-possibilities]: #future-possibilities

This would hopefully increase the usage of `str::split` and similar methods.
