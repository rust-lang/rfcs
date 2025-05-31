- Feature Name: `trim_prefix_suffix`
- Start Date: 2025-05-31
- RFC PR: [rust-lang/rfcs#3827](https://github.com/rust-lang/rfcs/pull/3827)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Add `trim_prefix` and `trim_suffix` methods to `str` which remove at most one occurrence of a specified prefix or suffix, always returning a string slice rather than an `Option`.

# Motivation
[motivation]: #motivation

Currently, Rust's string API has a gap between two existing method families:

- `strip_prefix`/`strip_suffix`: Remove a prefix/suffix if present, but return `Option<&str>`
- `trim_start_matches`/`trim_end_matches`: Always return `&str`, but repeatedly remove *all* prefixes/suffixes that match a pattern

There's no method that removes *at most one* occurrence of a prefix/suffix while always returning a string slice. This breaks method chaining entirely and forces developers to write verbose code for a common pattern:

```rust
// Current verbose approach:
let result = if let Some(stripped) = s.strip_prefix(prefix) {
    stripped
} else {
    s
};

// What we want:
let result = s.trim_prefix(prefix);
```

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

The `trim_prefix`/`trim_suffix` methods work similarly to the `strip_prefix`/`strip_suffix` methods, but they always return a string slice instead of [`Option`], allowing easy method chaining:

```rust
let s1 = " <https://example.com/> ";
let s2 =  "<https://example.com/>";
let s3 =   "https://example.com/>";
let s4 =  "<https://example.com/";
let s5 =   "https://example.com/";
let s6 =  " https://example.com/ ";

assert_eq!(s1.trim().trim_prefix('<').trim_suffix('>'), "https://example.com/");
assert_eq!(s2.trim().trim_prefix('<').trim_suffix('>'), "https://example.com/");
assert_eq!(s3.trim().trim_prefix('<').trim_suffix('>'), "https://example.com/");
assert_eq!(s4.trim().trim_prefix('<').trim_suffix('>'), "https://example.com/");
assert_eq!(s5.trim().trim_prefix('<').trim_suffix('>'), "https://example.com/");
assert_eq!(s6.trim().trim_prefix('<').trim_suffix('>'), "https://example.com/");
```

Using `strip_prefix`/`strip_suffix` requires saving intermediate values, which is much more awkward:

```rust
let s = " <https://example.com/> ";
let s = s.trim();
let s = s.strip_prefix('<').unwrap_or(s);
let s = s.strip_suffix('>').unwrap_or(s);

assert_eq!(s, "https://example.com/");
```

These methods complement the existing string manipulation methods by providing a middle ground between the fallible `strip_prefix`/`strip_suffix` methods and the greedy `trim_start_matches`/`trim_end_matches` methods.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## Method signatures

```rust
impl str {
    pub fn trim_prefix<P>(&self, prefix: P) -> &str
    where
        P: Pattern,

    pub fn trim_suffix<P>(&self, suffix: P) -> &str
    where
        P: Pattern,
        <P as Pattern>::Searcher<'a>: for<'a> ReverseSearcher<'a>,
}
```

## Behavior specification

- `trim_prefix(prefix)`: If the string slice starts with the pattern `prefix`, return the subslice after that prefix. Otherwise, return the original string slice.
- `trim_suffix(suffix)`: If the string slice ends with the pattern `suffix`, return the subslice before that suffix. Otherwise, return the original string slice.

## Implementation

```rust
impl str {
    /// Returns a string slice with the optional prefix removed.
    ///
    /// If the string starts with the pattern `prefix`, returns the substring after the prefix.
    /// Unlike [`strip_prefix`], this method always returns a string slice instead of returning [`Option`].
    ///
    /// If the string does not start with `prefix`, returns the original string unchanged.
    ///
    /// The [pattern] can be a `&str`, [`char`], a slice of [`char`]s, or a
    /// function or closure that determines if a character matches.
    ///
    /// [`char`]: prim@char
    /// [pattern]: self::pattern
    /// [`strip_prefix`]: Self::strip_prefix
    ///
    /// # Examples
    ///
    /// ```
    /// // Prefix present - removes it
    /// assert_eq!("foo:bar".trim_prefix("foo:"), "bar");
    /// assert_eq!("foofoo".trim_prefix("foo"), "foo");
    ///
    /// // Prefix absent - returns original string
    /// assert_eq!("foo:bar".trim_prefix("bar"), "foo:bar");
    /// ```
    #[must_use = "this returns the remaining substring as a new slice, \
                  without modifying the original"]
    #[unstable(feature = "trim_prefix_suffix", issue = "none")]
    pub fn trim_prefix<P: Pattern>(&self, prefix: P) -> &str {
        prefix.strip_prefix_of(self).unwrap_or(self)
    }

    /// Returns a string slice with the optional suffix removed.
    ///
    /// If the string ends with the pattern `suffix`, returns the substring before the suffix.
    /// Unlike [`strip_suffix`], this method always returns a string slice instead of returning [`Option`].
    ///
    /// If the string does not end with `suffix`, returns the original string unchanged.
    ///
    /// The [pattern] can be a `&str`, [`char`], a slice of [`char`]s, or a
    /// function or closure that determines if a character matches.
    ///
    /// [`char`]: prim@char
    /// [pattern]: self::pattern
    /// [`strip_suffix`]: Self::strip_suffix
    ///
    /// # Examples
    ///
    /// ```
    /// // Suffix present - removes it
    /// assert_eq!("bar:foo".trim_suffix(":foo"), "bar");
    /// assert_eq!("foofoo".trim_suffix("foo"), "foo");
    ///
    /// // Suffix absent - returns original string
    /// assert_eq!("bar:foo".trim_suffix("bar"), "bar:foo");
    /// ```
    #[must_use = "this returns the remaining substring as a new slice, \
                  without modifying the original"]
    #[unstable(feature = "trim_prefix_suffix", issue = "none")]
    pub fn trim_suffix<P: Pattern>(&self, suffix: P) -> &str
    where
        for<'a> P::Searcher<'a>: ReverseSearcher<'a>,
    {
        suffix.strip_suffix_of(self).unwrap_or(self)
    }
}
```

## Examples

```rust
// String literals
assert_eq!("hello world".trim_prefix("hello"), " world");
assert_eq!("hello world".trim_prefix("hi"), "hello world");
assert_eq!("hello world".trim_suffix("world"), "hello ");
assert_eq!("hello world".trim_suffix("universe"), "hello world");

// Characters
assert_eq!("xhello".trim_prefix('x'), "hello");
assert_eq!("hellox".trim_suffix('x'), "hello");

// Empty prefix/suffix
assert_eq!("hello".trim_prefix(""), "hello");
assert_eq!("hello".trim_suffix(""), "hello");

// Multiple occurrences (only first/last is removed)
assert_eq!("aaahello".trim_prefix('a'), "aahello");
assert_eq!("helloaaa".trim_suffix('a'), "helloaa");
```

# Drawbacks
[drawbacks]: #drawbacks

- Adds two more methods to an already large `str` API.
- Potential for confusion with existing `strip_*` and `trim_*` methods.
- Minor increase in standard library maintenance burden.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

## Why this design?

The `trim_prefix`/`trim_suffix` naming follows established conventions:
- `trim_*` methods always return `&str` (never `Option`).
- `strip_*` methods return `Option<&str>` when removal might fail.
- The `_prefix`/`_suffix` suffixes clearly indicate what is being trimmed and match the existing `strip_prefix`/`strip_suffix` methods.

## Alternative designs considered

1. **Extension trait in an external crate**: This works but fragments the ecosystem and doesn't provide the discoverability of standard library methods.

2. **Alternative naming**: `trim_start_match`/`trim_end_match` names would follow the pattern of existing `trim_start_matches`/`trim_end_matches` methods, using singular vs plural to distinguish behavior. However, this was rejected because:
   - The singular/plural distinction is subtle and error-prone.
   - `trim_start_match` vs `trim_start_matches` could easily be confused.
   - `trim_prefix`/`trim_suffix` more clearly communicate the intent to remove a specific prefix/suffix.
   - The prefix/suffix terminology aligns naturally with existing `strip_prefix`/`strip_suffix` methods.

3. **Generic over removal count**: A method that could remove N occurrences was considered too complex for the common use case.

## Why not just use `value.strip_prefix().unwrap_or(value)`?

While the `unwrap_or()` pattern works for simple cases, it has significant drawbacks:

1. **Poor method chaining**: The `unwrap_or()` approach breaks fluent interfaces entirely.

```rust
// Clean, readable chaining with proposed methods:
let result = value.trim().trim_prefix(prefix).trim_suffix(suffix).trim();

// Current approach - chaining is impossible:
let result = value.trim();
let result = result.strip_prefix(prefix).unwrap_or(result);
let result = result.strip_suffix(suffix).unwrap_or(result);
let result = result.trim();

// Attempting to chain with current methods doesn't work:
let trimmed = value.trim();
let result = trimmed
    .strip_prefix(prefix).unwrap_or(trimmed)
    .strip_suffix(suffix).unwrap_or(???)  // Can't reference intermediate values
    .trim();
```

2. **Verbosity**: Requires storing intermediate results and repeating variable names.
3. **Unclear intent**: The `unwrap_or()` pattern doesn't clearly communicate "remove if present, otherwise unchanged".

# Prior art
[prior-art]: #prior-art

Many string processing libraries in other languages provide similar functionality:

- **Python**: `str.removeprefix()` and `str.removesuffix()` (Python 3.9+)
- **JavaScript**: Various utility libraries provide `trimPrefix`/`trimSuffix` functions
- **Go**: `strings.TrimPrefix()` and `strings.TrimSuffix()`

# Unresolved questions
[unresolved-questions]: #unresolved-questions

None at this time.

# Future possibilities
[future-possibilities]: #future-possibilities

Nothing comes to mind; this change is focused and complete.
