- Feature Name: `Duration::is_zero()`
- Start Date: 2019-11-12
- RFC PR: Not available
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
      error to pass a zero `Duration` to this function._ This is a good example of the proposed function
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

    In [issue #2809](https://github.com/rust-lang/rfcs/issues/2809), one developer suggested new constant in
    `core::time` module:

    ```rust
    pub const ZERO: Duration = ...
    ```

    Some drawbacks I think of are:

    + Developers need to import one more component: either the constant or the `time` module (for usage like
      `if some_duration == time::ZERO`). While with `is_zero()`, they can just call it directly on any
      `Duration` instance.
    + Importing the constant might introduce new conflicting name to developer's existing code, which might not
      be their desire.

- Second alternative:

    Using `is_empty()` instead of `is_zero()`.

    I think `is_empty()` is related to collections (string, vector, set, map, slice...) However a duration is
    not like a collection. So this name is not appropriate.

- Third alternative:

    ```rust
    impl Duration {

        /// Creates new zero duration
        pub const fn zero() -> Self {
            ...
        }

    }
    ```

    This one was suggested by a developer, originally I didn't have this in mind. So from my view, I might not
    see proper use cases of it. However for this RFC, I think it's less convenient than `is_zero()`. For example:

    ```rust
    // It's longer if we type this
    if some_duration == Duration::zero() { ... }

    // Or, we have to declare new (internal) constant, and we might need
    // to repeat this for each independent project.
    const ZERO: Duration = Duration::zero();
    if some_duration == ZERO { ... }
    ```

    But I think `zero()` might be useful in other situations. So I'll add this one to
    [Future possibilities][future-possibilities] section.

# Prior art
[prior-art]: #prior-art

- <https://time-rs.github.io/time/time/struct.Duration.html#method.is_zero>

# Unresolved questions
[unresolved-questions]: #unresolved-questions

I could not think of any unresolved questions.

# Future possibilities
[future-possibilities]: #future-possibilities

One future possibility is this:

```rust
impl Duration {

    /// Creates new zero duration
    pub const fn zero() -> Self {
        ...
    }

}
```

It's one of above alternatives. From my view, I explained that it's less convenient than `is_zero()`. But I
think it might be useful for other developers.
