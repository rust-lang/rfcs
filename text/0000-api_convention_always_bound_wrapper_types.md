- Start Date: 2015-01-17
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Make it an API convention that generic types that have an interface
that always requires the same trait bounds on the same type parameters
_should_ have those bounds directly in the type definition. Allow exceptions
from this rule for cases like planned changes or upholding backwards compatibility.

# Motivation

The motivation here is to improve error messages, and to
prevent instances of types that are "useless" due to missing trait bounds.

For example, currently you can end up in this situation:

```rust
use std::sync::Mutex;

struct Foo<T>(Mutex<T>);

impl<T> Foo<T> {
    fn do_stuff(&self) {
        let _lock = self.0.lock();
        // ...
    }
}
```
```
<anon>:7:28: 7:34 error: type `std::sync::mutex::Mutex<T>` does not implement any method in scope named `lock`
<anon>:7         let _lock = self.0.lock();
                                    ^~~~~~
```

The issue here is that all of `Mutex<T>` functionality is only defined if `T: Send`.
But because that is not a bound on the type itself, this will cause confusing
error messages "late" in the process of writing the actual implementation for
the custom type `Foo<T>`, rather than "early" at the point where you define the
type itself.

With the convention proposed here, `Mutex` would be
defined as `struct Mutex<T: Send> { ... }`, which would make that error
appear sooner, and be more clear:

```
mutex-send.rs:3:1: 3:25 error: the trait `core::marker::Send` is not implemented for the type `T`
mutex-send.rs:3 struct Foo<T>(Mutex<T>);
                ^~~~~~~~~~~~~~~~~~~~~~~~
```

To fix it, you would then need to add the bound in the type itself, propagating that
requirement upwards: `struct Foo<T: Send>(Mutex<T>);`.

In combination with a change as proposed in http://smallcultfollowing.com/babysteps/blog/2014/07/06/implied-bounds/
, this would also allow removing redundant trait bounds for the actual implementation.

# Detailed design

## Implementing the convention

The std library needs to be reviewed for types that would fall under this convention,
and be changed accordingly. This would involve changes to stable API,
like in the case of `Mutex`.

## Upholding the convention during software evolution

If at a later point the type grows functionality that does not require the trait bound
after all, it can be removed from the type without breaking backwards compatibility.
It would simply lead to downstream users potentially over-constraining the type variable,
which is something they can fix if they need the new functionality.

In the reverse case, where a change to a type results in all functionality of it
now requiring the same trait bounds where it was not the case before, the author of the
library can pick on of two ways to proceed:

- Add the trait bound to the type, which would be a breaking change
  and causes a new major version.
- Don't add the trait bound to keep backwards-compatibility,
  which would not break any existing downstream uses of the.

In both cases, there would be no actual restriction of the functionality,
because anything depending on such a type would already have a trait bound
on the concrete impls or function making use of it.

This also means that following the convention in `std` would be a conservative
change, because it would not break downstream code to remove the bound again.

# Drawbacks

- Adding trait bounds to type definitions that are not necessary for the
  definition itself technically over-restricts what APIs you are allowed
  to use the type with. This means there could be valid use cases that would
  be blocked off by this convention.

  However, because you could not use the type in any meaningful way due to the missing
  trait bound, the author is not aware of any real use cases there.
- The issue of the bad error message can also be solved by making the compiler
  remember possible matches during method resolution and then propose them like this:
  `error: type Mutex<T> does not implement any method in scope named lock. Maybe
  you meant to restrict T by Send?`

  However, this still has the issue of only detecting the error late.
- Due to the optional nature of this convention
  (not following it might be required to remain backwards compatible), it might be hard
  to define and follow guidelines or lints for this.

# Alternatives

Keep the status quo, and solve the issue by better error messages that
keep track of possible bounds that might be missing.

This would mean users are potentially more likely to get
into situations where they can instantiate a type in a way that
makes it useless because of a missing trait bound, but does not restrict
them in any way.

# Unresolved questions

None so far
