- Feature Name: `Duration::is_zero()`
- Start Date: 2019-11-12
- RFC PR: N/A
- RFC Issue: [rust-lang/rfcs#2809](https://github.com/rust-lang/rfcs/issues/2809)

# Summary
[summary]: #summary

Adding new function `is_zero(&self) -> bool` to `core::time::Duration`.

# Motivation
[motivation]: #motivation

- Why are we doing this?

    Personally I found many times the need to compare a duration to zero. What I would do is:
    
    ```rust
    if some_duration == Duration::from_secs(0) {
        ...
    }
    ```

- What use cases does it support?

    It can be used to check if a duration is zero.

- What is the expected outcome?

    It can help eliminate workaround such as above.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

- Function name: `is_zero()`

    A duration holds a point related to timestamp. When we work with durations, we often think of:
    
    + 2 seconds
    + 30 seconds
    + 3 hours
    
    So using `is_zero()` is appropriate.

- Example usage:

    + `std::net::TcpStream::connect_timeout()` takes a duration as timeout. The documentation says: _It is an
      error to pass a zero `Duration` to this function._ This is a good example of the proposal function
      `is_zero()`.

    + `std::thread::sleep()` takes a duration. A zero duration does not help much in this case. If one program
      allows the user to set some delay time between jobs via command line, it needs to verify that a duration
      is not zero.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

Implementing this is quite simple. In Rust `1.39.0`, there are only 2 internal fields `secs` and `nanos`. So
an implementation might look like this:

```rust
impl Duration {

    /// # Checks if this duration is zero
    pub fn is_zero(&self) -> bool {
        self.secs == 0 && self.nanos == 0
    }

}
```

I think there are no corner cases.

# Drawbacks
[drawbacks]: #drawbacks

I could not think of any drawbacks.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

- First alternative:

    In (issue #2809)(https://github.com/rust-lang/rfcs/issues/2809), one developer suggested new constant in
    `core::time` module:

    ```rust
    pub const ZERO: Duration = ...
    ```

    Some drawbacks I think of are:

    + Developers need to import the constant or the `time` module (for usage like
      `if some_duration == time::ZERO`).
    + Importing the constant might introduce new conflicting name to developer's existing code, which might not
      be their desire.
    + Performance: I have no knowledge about compiler. But I _guess_ comparing 2 instances of `Duration`
      generates more code than having one instance checking its internal fields. If it's true, we should prefer
      `is_zero()` instead of this constant.

- Second alternative:

    Using `is_empty()` instead of `is_zero()`.

    I think `is_empty()` is related to collections (string, vector, set, map, slice...) However a duration is
    not like a collection. So this name is not appropriate.

# Prior art
[prior-art]: #prior-art

I have no examples of prior arts.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

I could not think of any unresolved questions.

# Future possibilities
[future-possibilities]: #future-possibilities

I could not think of any future possibilities.
