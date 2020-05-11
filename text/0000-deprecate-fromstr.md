- Feature Name: deprecate_fromstr
- Start Date: 2020-05-11
- RFC PR: [#2924](https://github.com/rust-lang/rfcs/pull/2924)
- Rust Issue: TBD

# Summary
[summary]: #summary

Deprecate [`FromStr`](https://doc.rust-lang.org/std/str/trait.FromStr.html) in favor of [`TryFrom<&str>`](https://doc.rust-lang.org/std/convert/trait.TryFrom.html)

# Motivation
[motivation]: #motivation

[`FromStr`](https://doc.rust-lang.org/std/str/trait.FromStr.html) was created when [`TryFrom`](https://doc.rust-lang.org/std/convert/trait.TryFrom.html) did not exist, [`FromStr`](https://doc.rust-lang.org/std/str/trait.FromStr.html) is now superfluous.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Basic implementation:
```rust
use std::convert::TryFrom;
use std::num::ParseIntError;

#[derive(Debug, PartialEq)]
struct Point {
    x: i32,
    y: i32,
}

impl TryFrom<&str> for Point {
    type Error = ParseIntError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        let coords: Vec<&str> = s
            .trim_matches(|p| p == '(' || p == ')')
            .split(',')
            .collect();

        let x_fromstr = i32::try_from(coords[0])?;
        let y_fromstr = i32::try_from(coords[1])?;

        Ok(Point {
            x: x_fromstr,
            y: y_fromstr,
        })
    }
}
```

Example:
```rust
let p = Point::try_from("(1,2)");
assert_eq!(p.unwrap(), Point { x: 1, y: 2 })
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

- Mark [`FromStr`](https://doc.rust-lang.org/std/str/trait.FromStr.html) as deprecated
- Mark [`str::parse()`](https://doc.rust-lang.org/std/primitive.str.html#method.parse) as deprecated
- Make [`FromStr`](https://doc.rust-lang.org/std/str/trait.FromStr.html) implement `TryFrom<&str>` with [specialization](https://github.com/rust-lang/rfcs/pull/1210):
```rust
impl<T> TryFrom<String> for T
where
    T: FromStr,
{
    type Error = <T as FromStr>::Err;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse()
    }
}
```

# Drawbacks
[drawbacks]: #drawbacks

- `TryFrom<&str> for U` implies `TryInto<U> for &str` which may be unwanted

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

# Prior art
[prior-art]: #prior-art

Some discussion about a tangential problem https://github.com/rust-lang/rfcs/issues/2143

# Unresolved questions
[unresolved-questions]: #unresolved-questions

# Future possibilities
[future-possibilities]: #future-possibilities
