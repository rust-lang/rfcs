- Feature Name: result-pass
- Start Date: (fill me in with today's date, YYYY-MM-DD)
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Result currently provides no convenient way to convert between `Result`s, even
if a conversion between the wrapped values is defined. This makes it much
more involved to pass on full `Result` values, then using `try!` or `?` for
passing on plain error values. 

This RFC suggests adding a method `pass()`, which converts `Result<T,E>`
into `Result<U,F>` if `T: Into<U>` and `E: Into<F>`.

# Motivation
[motivation]: #motivation

Consider the following code:

```rust
#[derive(Debug)]
enum ServerError {
    IoError(std::io::Error),
    // some more cases
}

impl From<std::io::Error> for ServerError {
    fn from(e: std::io::Error) -> ServerError {
        ServerError::IoError(e)
    }
}

fn main() {
    let listener = TcpListener::bind("127.0.0.1:7878").unwrap();

    for stream in listener.incoming() {
        let res = stream.map_err(ServerError::from)
                  .and_then(|mut s| {
                      handle(&mut s)
                  });
        
        if let Err(e) = res {
            println!("Error occured: {:?}", e);
        }
    }
}

fn handle(stream: &mut TcpStream) -> Result<(), ServerError> {
    write!(stream, "hello!").map_err(ServerError::from)
}
```

Note the calls to `map_err`, which only provide trivial conversion
between errors. Finding this pattern is non-trivial and its application
unnecessarily repetitive.

While `Result` provides a lot of convenience methods to be combined,
it does not provide a trivial way to be converted into other `Result`,
even if implementors followed good practice and provide `std::convert`
implementations.

Similar arguments are apply for the `T` value.

# Detailed design
[design]: #detailed-design

Add a method `pass` to `Result`, with the following implementation:

```rust
impl Result<T,E> {
    fn pass<U,F>(self) -> Result<U,F> where T: Into<U>, E: Into<F> {
        self.map(T::into).map_err(E::into)
    }
}
```

This allows easy, non-involved conversion between `Result` types that
have their `std::convert` story in order.

As `T` implies `T: From<T>`, this also neatly applies when the error
or the Result stay the same.

The name `pass` was chosen to evoke "passing the Result on".

# How We Teach This
[how-we-teach-this]: #how-we-teach-this

This is a method addition that should come with its own documentation,
and additional documentation in `std::result`. An addition to the
error handling practices in the book can be considered.

# Drawbacks
[drawbacks]: #drawbacks

`Result` already has a big interface and this adds a method to it.

# Alternatives
[alternatives]: #alternatives

Implement `From<Result<T,E>>` for `Result<U,F> where U: From<T>, F: From<E>`.

This is more general, but possibly harder to discover, as the appropriate method to call would end up `into()`.

Possibly also implement `TryFrom<Result<T,E>>` where `Result<U,F> where U: TryFrom<T, Err: Into<F>>, F: From<E>`.

# Alternative names

* `forward()`: "forwards the result to the next handler"

# Unresolved questions
[unresolved]: #unresolved-questions

🚲🏡: Is `pass` the best name?