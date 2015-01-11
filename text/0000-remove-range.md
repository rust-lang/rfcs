- Start Date: 2015-01-11
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Add methods to `String` and `Vec<T>` that allow the user to remove more than
one character/element at a time while preserving the order of the remaining
characters/elements.

# Motivation

`String` and `Vec<T>` currently contain the following safe methods related to
this proposal:

- `Vec::truncate`,
- `Vec::swap_remove`,
- `Vec::remove`,
- `Vec::pop`,
- `String::pop`, and
- `String::remove`.

`Vec::truncate` allows the user to remove an arbitrary number of elements from
the end of the vector.

`Vec::swap_remove` allows the user to remove a single element anywhere in the
vector by replacing it with the last element.

`Vec::remove` allows the user to remove a single element anywhere in the vector
by shifting the rest of the vector one slot to the left.

`Vec::pop` removes a single element from the end of the vector.

`String::pop` and `String::remove` are equivalent to the `Vec` methods with the
same name except that they operate on the UTF8-representations of characters and
`String::remove` panics if the index is not at the boundary of a character. For
example, given the `String` "さび", which is internally represented as the
`Vec<u8>` `[0xe3, 0x81, 0x95, 0xe3, 0x81, 0xb3]`, `String::remove(0)` removes
the first 3 bytes.

This RFC proposes adding the following methods:

- `Vec::remove_range(&mut self, start: usize, end: usize) -> ()`, and
- `String::remove_range(&mut self, start: usize, end: usize) -> ()`.

Leaving performance aside, calling `Vec::remove_range(n, m)` is equivalent to
calling `Vec::remove(n)` `m-n` times, and calling `String::remove_range(n, m)`
is equivalent to calling `String::remove(n)` once for every character in the
`[n, m)` range.

These methods are necessary to remove multiple elements in a performant manner.
Calling `Vec::remove` `n` times has `O(n * Vec::len)` performance instead of
`O(Vec::len)` which can be achieved with `Vec::remove_range`. This is true for
both `Copy` types and non-`Copy` types. For types with destructors it is clear
from the implementation that this cannot be improved (consider the case where
the destructor of one of the elements in the vector panics.) But even for `Copy`
types the compiled code will call `memmove` `n` times. One reason for this is
that calling `memmove` `n` times and calling `memmove` once will not leave the
memory in the same state. More precisely: If the user calls `Vec::set_len` after
calling `Vec::remove` `n` times, he will find that the last element has been
duplicated `n` times at the end of the vector. For these reasons, it seems
unreasonable to assume that any improvement of LLVM or the current `Vec::remove`
implementation will yield better performance.

(Furthermore: Calling `Vec::pop` in a loop for `Copy` types is `O(n)` while
`Vec::truncate` is `O(1)`.)

One application of these methods is displaying text in user interfaces. More
precisely: Using `Backspace` inside a text box will remove a grapheme which can
consist of any number of characters.

# Detailed design

Add the following method to `Vec`:

```rust
/// A view into a vector that frees the elements it contains but not the pointer.
struct DroppingSlice<T> {
    ptr: *mut T,
    len: usize,
}

#[unsafe_destructor]
impl<T> Drop for DroppingSlice<T> {
    fn drop(&mut self) {
        unsafe {
            let end = self.ptr.offset(self.len as isize);
            while self.ptr != end {
                ptr::read(self.ptr);
                self.ptr = self.ptr.offset(1);
            }
        }
    }
}

/// Removes the elements between `from` and `to`, shifting all elements after `to` to the
/// left.
///
/// # Panics
///
/// Panics if `to` is out of bounds or `from > to`.
///
/// # Examples
///
/// ```
/// rut mut vec = vec!(0, 1, 2, 3);
/// vec.remove_range(1, 3);
/// assert_eq!(vec, vec!(0, 3));
/// ```
fn remove_range(&mut self, from: usize, to: usize) {
    assert!(from <= to);
    assert!(to <= self.len());

    let ptr = self.as_mut_ptr();
    let len = self.len();

    unsafe {
        // If the code below fails, we don't want to drop anything twice.
        self.set_len(from);
        let middle = DroppingSlice { ptr: ptr.offset(from as isize), len: to  - from };
        // If the code above fails, let's at least try to drop the remaining elements.
        let end    = DroppingSlice { ptr: ptr.offset(to   as isize), len: len - to   };
        drop(middle);
        // Apparently it didn't fail, so don't drop the tail.
        mem::forget(end);
        ptr::copy_memory(ptr.offset(from as isize), ptr.offset(to as isize), len - to);
        self.set_len(len - (to - from));
    }
}
```

Add the following method to `String`:

```rust
/// Removes the characters between `from` and `to`, shifting all characters after `to` to
/// the left.
///
/// # Panics
///
/// Panics if `to` is out of bounds, or `from > to`, or `from` or `to` are not at
/// character boundaries.
fn remove_range(&mut self, from: usize, to: usize) {
    assert!(self.is_char_boundary(from));
    assert!(self.is_char_boundary(to));

    unsafe { self.as_mut_vec().remove_range(from, to); }
}
```

# Drawbacks

None

# Alternatives

None

# Unresolved questions

None
