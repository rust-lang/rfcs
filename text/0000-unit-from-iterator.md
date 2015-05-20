- Feature Name: unit_from_iterator
- Start Date: 2015-05-20
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Implement `std::iter::FromIterator<()>` for the empty tuple type `()`, also known as “unit”.

# Motivation

The `Result` type implements `FromIterator` in a way that creates a result out of an iterable of results, extracting the first error if any, and otherwise returning `Ok` with the contents of all values:

```rust
println!("{:?}", Result::<Vec<i32>, MyError>::from_iter(vec![Ok(4), Ok(7), Ok(-3903)])); // Ok([4, 7, -3903])
println!("{:?}", Result::<Vec<i32>, MyError>::from_iter(vec![Ok(5), Err(MyError::DivisionByZero), Ok(842), Err(MyError::Overflow)])); // Err(DivisionByZero)
```

Implementing this RFC would allow this pattern to be used with iterables whose item type is of the form `Result<(), T>`.

For example, suppose we have a function which moves all values from a `mpsc::Receiver` into a `mpsc::Sender`. Currently, this could be written as follows:

```rust
fn forward_values<T>(src: Receiver<T>, dst: Sender<T>) -> Result<(), SendError<T>> {
    src.iter().map(|val| dst.send(val)).fold(Ok(()), Result::and))
}
```

This has the flaw of exhausting the receiver even after an error is encountered. With the proposed trait implementation, it could be refactored into the following:

```rust
fn forward_values<T>(src: Receiver<T>, dst: Sender<T>) -> Result<(), SendError<T>> {
    src.iter().map(|val| dst.send(val)).collect()
}
```

This version of the function immediately returns when the first error is encountered.

# Detailed design

Implement the trait `std::iter::FromIterator<()>` for the primitive type `()`.

The implementation is very short:

```rust
impl FromIterator<()> for () {
    fn from_iter<T>(_: T) -> () where T: IntoIterator<Item = ()> {
        ()
    }
}
```

# Drawbacks

The only known drawback is that the use-cases for this functionality seem to be quite limited and as such may not warrant an addition to the standard library. However, the `()` type has only one possible value, so if more use-cases requiring an implementation of this trait for `()` are found, the proposed implementation must already be correct.

# Alternatives

*   Do nothing. The same short-circuiting behavior shown in the example can be achieved by using a for loop and `try!`:
    
    ```rust
    fn forward_values<T>(src: Receiver<T>, dst: Sender<T>) -> Result<(), SendError<T>> {
        for val in src {
            try!(dst.send(val));
        }
        Ok(())
    }
    ```
*   Add a special-cased `FromIterator` implementation for `Result`:
    
    ```rust
    Impl<E> FromIterator<Result<(), E>> for Result<(), E> {
        fn from_iter<T>(iterable: T) -> Result<(), E> where T: Iterator<Item = Result<(), E>> {
            for val in iterable {
                try!(val);
            }
            Ok(())
        }
    }
    ```

# Unresolved questions

None at this time.
