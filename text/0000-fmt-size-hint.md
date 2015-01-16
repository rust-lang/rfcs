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

## size_hint method

Add a `size_hint` method to all of the `fmt` traits, with a default implementation so no one is broken. Opting in simply means better performance. All traits should have their own size_hint implementation, since the trait used can change the length of the output written.

```rust
trait Show {
    fn fmt(&self, &mut Formatter) -> Result;
    #[unstable]
    fn size_hint(&self) -> SizeHint {
        SizeHint { min: 0, max: None }
    }
}
```

## SizeHint type

Add a `SizeHint` type, with named properties, instead of using tuple indexing. Include an `Add` implementation for `SizeHint`, so they can be easily added together from nested properties.

```rust
#[unstable]
struct SizeHint {
    min: usize,
    max: Option<usize>
}

impl Add for SizeHint {
    type Output = SizeHint;
    fn add(self, other: SizeHint) -> SizeHint {
        SizeHint {
            min: self.min.saturating_add(other.min),
            max: if let (Some(left), Some(right)) = (self.max, other.max) {
                Some(left.checked_add(right)),
            } else {
                None
            }
        }
    }
}
```

This type differs from `Iter::size_hint`, primarily to provide an `Add` implementation that doesn't interfere with `(usize, Option<usize>)` globally. Since using our own internal type, a struct with named properties is more expressive than a tuple-struct using tuple indexing.

It's possible that `Iter::size_hint` could adopt the same type, but that seems out of scope of this RFC.

## std::fmt::format

There are 2 ways that the format traits are used: through `std::fmt::format`, and `std::string::ToString`. The `ToString` blanket implementation will be adjusted to simply wrap `std::fmt::format`, so there is no longer duplicated code.

```rust
impl<T: fmt::String> ToString for T {
    fn to_string(&self) -> String {
        format!("{}", self)
    }
}
```

The size hint will be accessed in `std::fmt::format` to provide the initial capacity to the `fmt::Writer`. Since calls to `write!` use a pre-existing `Writer`, use of a size hint there is up to the creator of said `Writer`.

Here is where we could be clever with `SizeHint`'s `min` and `max` values. Perhaps if difference is large enough, some value in between could be more efficient. This is left in the Unresolved Questions section.

```rust
pub fn format(args: Arguments) -> string::String {
    let mut output = string::String::with_capacity(args.size_hint().min);
    let _ = write!(&mut output, "{}", args);
    output
}
```

This involves implementing `size_hint` for `Arguments`:

```rust
impl String for Arguments {
    //fn fmt(&self, ...)
    fn size_hint(&self) -> SizeHint {
        let pieces = self.pieces.iter().fold(0, |sum, &piece| sum.saturating_add(piece.len()));
        let args = self.args.iter().fold(SizeHint { min: 0, max: None }, |sum, arg| {
            sum + String::size_hint(arg)
        });
        args + SizeHint { min: pieces, max: Some(pieces) }
    }
}

```

Each `Argument` includes a `fmt` function, and reference to the object to format, with its type erased. In order to get the `SizeHint`, the appropriate `size_hint` function will need to be included in the `Argument` struct.


```rust
pub struct Argument<'a> {
    value: &'a Void,
    formatter: fn(&Void, &mut Formatter) -> Result,
    hint: fn(&Void) -> SizeHint,
}

impl<'a> String for Argument<'a> {
    // fn fmt ...
    fn size_hint(&self) -> SizeHint {
        (self.hint)(self.value)
    }
}
```

The public facing constructor of `Argument` would be altered to this:

```rust
pub fn argument<'a, T>(f: fn(&T, &mut Formatter) -> Result,
                       s: fn(&T) -> SizeHint,
                       t: &'a T) -> Argument<'a> {
    Argument::new(t, f, s)                       
}
```

## Examples 

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

Added complexity may conflict with landing more critical things for 1.0, but that should only mean a possible postponing, vs rejection.

# Alternatives

An alternative proposed by @kballard:

> I've got a different approach that does not change the API of `Show` or `String` (or anything else that third-party code is expected to implement). It's focused around optimizing the case of `"foo".to_string()`. The approach here is to add a parameter to `std::fmt::Writer::write_str()` called more_coming that is true if the caller believes it will be calling `write_str()` again. The flag is tracked in the Formatter struct so calls to `write!()` inside an implementation of `Show/String` don't incorrectly claim there will be no writes if their caller still has more to write. Ultimately, the flag is used in the implementation of `fmt::Writer` on `String` to use `reserve_exact()` if nothing more is expected to be written.

A drawback of this alternative is that it focuses improvements only when the object is a String, or at least does not contain properties that will be be formatted as well. The proposal in this RFC provides improvements for all types.

The impact of not doing this at all is that `"foo".to_string()` stays at its current speed. Adding the size hints knocks the time down by around half in each case.

# Unresolved questions

This RFC proposes a `SizeHint` that has both a lower and upper bound. It's not immediately clear to me how to intelligently make use of both.
