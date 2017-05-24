- Feature Name: explicit-char-toloweruppercase-length
- Start Date: 2015-04-07
- RFC PR:
- Rust Issue:

# Summary

Change the `to_lowercase` and `to_uppercase` methods of `char` so they return an enum
that makes the possibility that a case-change also increases the number of chars explicit.


# Motivation

In [#23126](https://github.com/rust-lang/rust/pull/23126),
these two methods on `CharExt` where changed to return an `Iterator` of `char`
instead of a single `char`.
This is to enable the more correct
[`SpecialCasing.txt`](http://www.unicode.org/Public/UCD/latest/ucd/SpecialCasing.txt)
Unicode mapping where the number of code points can grow.

However, since these iterators often (and currently always) yield exactly one item,
it is tempting to write incorrect code like:

```rust
let c: char = ...;
c.to_uppercase().next().unwrap()
```

Changing these methods in the proposed way prevents such code from creating un-intuitive results.

# Detailed design

The enum types `UppercasedChar` and `LowercasedChar` is implemented as follows:

```rust
enum UppercasedChar {
    Single(char),
    Many(ToUppercase),
}

impl UppercasedChar {
    fn unwrap_single(self) -> char {
        match self {
            Single(c) => c,
            Many(_) => panic!("expected a single char, got many"),
        }
    }
}

impl IntoIterator for UppercasedChar {
    type Item = char;
    type IntoIter = ToUppercase;

    fn into_iter(self) -> ToUppercase {
        match self {
            Single(c) => ToUppercase(Some(c)),
            Many(tu) => tu,
        }
    }
}

enum LowercasedChar {
    Single(char),
    Many(ToLowercase),
}

impl LowercasedChar {
    fn unwrap_single(self) -> char {
        match self {
            Single(c) => c,
            Many(_) => panic!("expected a single char, got many"),
        }
    }
}

impl IntoIterator for LowercasedChar {
    type Item = char;
    type IntoIter = ToLowercase;

    fn into_iter(self) -> ToLowercase {
        match self {
            Single(c) => ToLowercase(Some(c)),
            Many(tl) => tl,
        }
    }
}
```

The return type of `char::to_lowercase` and `char::to_uppercase` is changed to `UppercasedChar` and `LowercasedChar` respectively.


# Drawbacks

None?

# Alternatives

* Status quo.
* https://github.com/rust-lang/rfcs/pull/986


# Unresolved questions

None so far.
