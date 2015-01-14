- Start Date: 2015-01-13
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Add a `size_hint` method to each of the `fmt` traits, allowing a buffer to allocate with the correct size before writing.

# Motivation

Using the `fmt` traits is slower than a straight `memcpy` of data. The removal of `std::is_utf8` has helped some. The other low-hanging fruit is to add a size hint, so as to reduce or prevent unnecessary allocations during writing to the output buffer. My initial implementation includes [benchmarks][], which looked like this on machine:

[benchmarks]: https://gist.github.com/seanmonstar/8fb7aa6b0512b80522f9#file-size_hint-rs-L91-L162

```
running 11 tests
test bench_long         ... bench:       133 ns/iter (+/- 18)
test bench_long_hint    ... bench:        72 ns/iter (+/- 10)
test bench_long_memcpy  ... bench:        44 ns/iter (+/- 2)
test bench_med          ... bench:       112 ns/iter (+/- 10)
test bench_med_hint     ... bench:        59 ns/iter (+/- 7)
test bench_med_memcpy   ... bench:        32 ns/iter (+/- 6)
test bench_nested       ... bench:       248 ns/iter (+/- 19)
test bench_nested_hint  ... bench:       134 ns/iter (+/- 6)
test bench_short        ... bench:        96 ns/iter (+/- 13)
test bench_short_hint   ... bench:        60 ns/iter (+/- 3)
test bench_short_memcpy ... bench:        33 ns/iter (+/- 3)
```

# Detailed design

Add a `size_hint` method to all of the `fmt` traits, with a default implementation so no one is broken. Opting in simply means better performance. All traits should have their own size_hint implementation, since the trait used can change the length of the output written.

```rust
trait Show {
    fn fmt(&self, &mut Formatter) -> Result;
    fn size_hint(&self) -> SizeHint {
        SizeHint { min: 0, max: None }
    }
}
```

Add a `SizeHint` type, with named properties, instead of using tuple indexing. Include an `Add` implementation for `SizeHint`, so they can be easily added together from nested properties.

```rust
struct SizeHint {
    min: usize,
    max: Option<usize>
}

impl Add for SizeHint {
    type Output = SizeHint;
    fn add(self, other: SizeHint) -> SizeHint {
        SizeHint {
            min: self.min + other.min,
            max: match (self.max, other.max) {
                (Some(left), Some(right)) => Some(left + right),
                // if either is None, we can't assume a max
                _ => None
            }
        }
    }
}
```

Some example implementations:

```rust
impl fmt::String for str {
    // fn fmt ...
    fn size_hint(&self) -> SizeHint {
        let len = self.len();
        SizeHint { min: len, max: Some(len) }
    }

}

struct Foo(String, String);

impl fmt::Show for Foo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Foo({:?}, {:?})", self.0, self.1)
    }

    fn size_hint(&self) -> SizeHint {
        Show::size_hint(&self.0) + Show::size_hint(&self.1) + SizeHint {
            min: 7,
            max: Some(7)
        }
    }
}
```

Deriving `Show` would also be able to implement `size_hint` meaning most everyone just gets this for free.

# Drawbacks

I can't think of a reason to stop this.

# Alternatives

The impact of not doing this is that `"foo".to_string()` stays at its current speed. Adding the size hints knocks the time down by around half in each case.

# Unresolved questions

This RFC proposes a `SizeHint` that has both a lower and upper bound. It's not immediately clear to me how to intelligently make use of both.
