- Feature Name: deprecate_fromstr
- Start Date: 2020-05-11
- RFC PR: [#2924](https://github.com/rust-lang/rfcs/pull/2924)
- Rust Issue: TBD

# Summary
[summary]: #summary

Deprecate [`FromStr`](https://doc.rust-lang.org/std/str/trait.FromStr.html) in favor of [`TryFrom<&str>`](https://doc.rust-lang.org/std/convert/trait.TryFrom.html) and [`From<&str>`](https://doc.rust-lang.org/std/convert/trait.From.html).

# Motivation
[motivation]: #motivation

[`FromStr`](https://doc.rust-lang.org/std/str/trait.FromStr.html) was created when [`TryFrom`](https://doc.rust-lang.org/std/convert/trait.TryFrom.html) didn't exist. Now that it is stable, [`FromStr`](https://doc.rust-lang.org/std/str/trait.FromStr.html) becomes superfluous. 

[`FromStr`](https://doc.rust-lang.org/std/str/trait.FromStr.html) definition is virtually identical to [`TryFrom<&str>`](https://doc.rust-lang.org/std/convert/trait.TryFrom.html):
```rust
pub trait FromStr: Sized {
    type Err;

    fn from_str(s: &str) -> Result<Self, Self::Err>;
}

// Where T is &str
pub trait TryFrom<T>: Sized {
    type Error;

    fn try_from(value: T) -> Result<Self, Self::Error>;
}
```

Infallible conversions become more idiomatic:
```rust
struct Dummy(String);

impl std::str::FromStr for Dummy {
    type Err = core::convert::Infallible;
    
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Dummy(s.to_owned()))
    }
}

//vs

impl From<&str> for Dummy {

    fn from(s: &str) -> Self {
        Dummy(s.to_owned())
    }
}
```

[`FromStr`](https://doc.rust-lang.org/std/str/trait.FromStr.html) limits lifetimes in a way that prevents borrowing the passed string:
```rust
struct Dummy<'a>(&'a str);

// This doesn't compile
impl<'a> std::str::FromStr for Dummy<'a> {
    type Err = core::convert::Infallible;
    
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Dummy(s))
    }
}

//This works
impl<'a> From<&'a str> for Dummy<'a> {

    fn from(s: &'a str) -> Self {
        Dummy(s)
    }
}

```

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
- Implement [`TryFrom<&str>`](https://doc.rust-lang.org/std/convert/trait.TryFrom.html) for all types implementing [`FromStr`](https://doc.rust-lang.org/std/str/trait.FromStr.html)

# Drawbacks
[drawbacks]: #drawbacks

To be discussed

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

To be discussed

# Prior art
[prior-art]: #prior-art

Some discussion about a tangential problem: https://github.com/rust-lang/rfcs/issues/2143

# Unresolved questions
[unresolved-questions]: #unresolved-questions

To be discussed

# Future possibilities
[future-possibilities]: #future-possibilities

To be discussed
