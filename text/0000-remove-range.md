- Start Date: 2015-01-11
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Add a method to `String` that allow the user to remove more than
one character at a time while preserving the order of the remaining
characters.

# Motivation

`String`currently contains the following safe methods related to this proposal:

- `String::pop`, and
- `String::remove`.

`String::pop` remove a single character from the end of the string.

`String::remove` allows the user to remove a single character anywhere in the string by shifting the rest of the string one slot to the left.

For example, given the `String` "さび", which is internally represented as the
`Vec<u8>` `[0xe3, 0x81, 0x95, 0xe3, 0x81, 0xb3]`, `String::remove(0)` removes
the first 3 bytes.

This RFC proposes adding the following method:

- `String::remove_range<T: SomeTrait>(&mut self, range: T)`.

Leaving performance aside, calling `String::remove_range(n..m)` is equivalent
to calling `String::remove(n)` once for every character in the `[n, m)` range.

This method are necessary to remove multiple characters in a performant manner.
Calling `String::remove` `n` times has `O(n * String::len)` performance instead of
`O(String::len)` which can be achieved with `String::remove_range`.

One application of these methods is displaying text in user interfaces. More
precisely: Using `Backspace` inside a text box will remove a grapheme which can
consist of any number of characters. Using `Ctrl-Backspace` will remove a word, etc.

# Detailed design

Add the following method to `String`:

```rust
/// Removes the characters in the specified range.
///
/// # Panics
///
/// Panics if the range is decreasing, the upper bound is larger than the length of the
/// string, or the bounds are not character boundaries.
fn remove_range<T: SomeTrait>(&mut self, range: T) { }
```

Where `SomeTrait` should be implemented for all range types.
