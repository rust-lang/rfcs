- Feature Name: localkey_try_with
- Start Date: 2017-06-14
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Add a `try_with` method to `std::thread::LocalKey`, which attempts to borrow the value.
This replaces the current `state` method, which will be marked as deprecated along with its return value `LocalKeyState`.
It will return an error if the value has been destroyed, but will panic if the value initializer panics.

# Motivation
[motivation]: #motivation

Many users of `thread_local` will need to handle the case in which the value has already been destroyed.
For instance, in the standard library, `print!` is useable in destructors because it uses the existing (but unstable)
`state` method. If the thread local stdout has been destroyed, `print!` falls back on a global stdout.

`try_with` is an improvement on `state` because:

- It simplifies the code, and puts it in the normal `Result` error handling pattern instead of matching `LocalKeyState`.
- It removes an additional check on the state (the state is checked again when `.with` is called).

The existing `state` method often creates code similar to the following anti-pattern (as seen in `print!`):

```rust
if result.is_ok() {
    let product = result.unwrap();
    ...
}
```

Whereas `try_with` can be used similar to:

```rust
if let Ok(product) = result {
    ...
}
```

Both `state` and checking `is_ok` usually require checking their value twice before proceeding,
and make the code more complicated than necessary.

# Detailed design
[design]: #detailed-design

Method signature:
```rust
pub fn try_with<F, R>(&'static self, f: F) -> Result<R, LocalKeyError>
                      where F: FnOnce(&T) -> R
```

`LocalKeyError` definition (same as `std::cell::BorrowError`):

```rust
pub struct LocalKeyError {
    _private: (),
}
```

Implementing this is trivial. The main difference from the existing `with` method implementation
is changing a `.expect` to a `?`.

`LocalKey::state` and `LocalKeyState` will both be marked as deprecated.

# How We Teach This
[how-we-teach-this]: #how-we-teach-this

This is very similar to `RefCell`'s `try_borrow`, and fits into existing Rust patterns.

As this is a rather obscure edge case, I don't think this will need any documentation other than
the usual rustdoc.

# Drawbacks
[drawbacks]: #drawbacks

- This makes it hard to check if a `LocalKey` can be borrowed without borrowing it.
  However, I haven't encountered and cannot think of a situation where that would be necessary.
  In addition, `RefCell` currently has a `try_borrow` but no way of checking its state.
  If a situation necessitating `state` comes up, the existing `state` method can be kept while still adding `try_with`.
- `try_with` panicking instead of returning an error if the initializer fails may be unexpected to many users.
  However, this is far better than building in a `catch_unwind` (because of the flaws of `catch_unwind`).

# Alternatives
[alternatives]: #alternatives

- Don't deprecate `state` but still add `try_with`.
- Instead of returning a `Result` from `try_with`, pass a `Result` to the closure.
- The error type for the `Result` could be an enum, or just `()`.

# Unresolved questions
[unresolved]: #unresolved-questions

- Is `state` still necessary with `try_with`?
- Should `try_with` return a `Result` or pass a `Result` to the closure?
