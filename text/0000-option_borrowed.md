- Feature Name: `option_borrowed`
- Start Date: 2016-11-14
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

The standard library provides the `Option::<&T>::cloned` method for `T: Clone`
as the standard way to convert from `Option<&T>` to `Option<T>`. This RFC
proposes the addition of `Option::<&T>::borrowed` for `T: Borrow<U>` to convert
from `Option<&T>` to `Option<&U>`.

# Motivation
[motivation]: #motivation

How to convert from `Option<String>` to `Option<&str>` is sometimes asked on
Stack Overflow [1] [2]. This use case is also common in Servo, along other
cases that would also be covered by such a method.

[1] http://stackoverflow.com/q/31233938
[2] http://stackoverflow.com/q/34974732

# Detailed design
[design]: #detailed-design

This implementation will be added to the `core::option` module:

```rust
use core::borrow::Borrow;

impl<'a, T> Option<&'a T> {
    fn borrowed<U>(self) -> Option<&'a U> where T: Borrow<U> {
        self.map(T::borrow)
    }
}
```

# Drawbacks
[drawbacks]: #drawbacks

None.

# Alternatives
[alternatives]: #alternatives

We could instead add a `dereferenced` method using the Deref trait, but that's
a mouthful and is less flexible than using `Borrow`.

# Unresolved questions
[unresolved]: #unresolved-questions

None.
