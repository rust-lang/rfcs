- Feature Name: remove-char-toloweruppercase
- Start Date: 2015-03-17
- RFC PR:
- Rust Issue:

# Summary

Deprecate the `to_lowercase` and `to_uppercase` methods of `CharExt`,
and replace them with methods on `IteratorExt<Item=char>`.


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

Removing these methods removes this temptation,
and also reflects that Unicode Scalar Values (i.e. `char`) are not such a meaningful unit
when doing case mapping.


# Detailed design

`CharExt::to_lowercase` and `CharExt::to_uppercase` as well as their return types
would be marked as deprecated,
with a message like “use `Some(c).iter().to_lowercase()` or `String::to_lowercase` instead.”

To replace them, add new methods to the `std::iter::IteratorExt` trait:

```rust
trait IteratorExt {
    // ...

    fn to_lowercase(self) -> ToLowercase<Self> where Self: Iterator<Item=char> {
        // ...
    }

    fn to_uppercase(self) -> ToUppercase<Self> where Self: Iterator<Item=char> {
        // ...
    }
}

struct ToLowercase<I> where I: Iterator<Item=char> {
    // ...
}

struct ToUppercase<I> where I: Iterator<Item=char> {
    // ...
}

impl Iterator for ToLowercase {
    type Item = char;

    // ...
}

impl Iterator for ToUppercase {
    type Item = char;

    // ...
}
```

The `String::to_lowercase` and `String::to_uppercase` convenience methods
would stay externally unchanged.

To provide a way forward [for the regex crate](https://github.com/rust-lang/regex/issues/55),
Unicode case folding (which is not quite the same as lowercase mapping)
will be provided in a crate.io library.


# Drawbacks

The `CharExt` methods could be convenient in some cases, and removing them might be a loss.
However, e.g. `(c: char).to_lowercase()` can easily be replaced
with `Some(c: char).iter().to_lowercase()`.


# Alternatives

* Status quo.
* Instead of new methods on `IteratorExt`, have functions that accept `I: Iterator<Item=char>`.


# Unresolved questions

None.
