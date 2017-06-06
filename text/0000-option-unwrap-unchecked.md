- Feature Name: option_unwrap_unchecked
- Start Date: 2015-4-28
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

This RFC adds a method to the `Option` enum which allows it to be
unsafely unwrapped without checking for `None`.

# Motivation

In cases where a developer can determine that an `Option` is `Some(v)`
but the compiler cannot, this method allows a developer to unsafely
opt-in to an optimization which will avoid a conditional check. In my
own uses I find cases where I know that an `Option` is `Some(v)`. I
present a case from the `libcollections linked_list.rs`:

    fn next(&mut self) -> Option<&'a A> {
        if self.nelem == 0 {
            return None;
        }
        self.head.as_ref().map(|head| {
            self.nelem -= 1;
            self.head = &head.next;
            &head.value
        })
    }

The check for `nelem == 0` already ensures that `self.head.as_ref()`
cannot be `None` but the `map` method is unlikely to optimize out the
check. Here, an unsafe method which could unwrap the `Option` would
allow the compiler to optimize. I expect that this can be used in many
more cases throughout both the standard library and other code. The
outcome is likely a minor performance improvement, but one that should
be possible for developers to make.

# Detailed design

I choose to name this method `unwrap_unchecked()` (commence
bike-shedding). It is implemented quite simply by using the
`unreachable` intrinsic to indicate that the `Option` cannot be `None`
and then calling `unwrap()`. I expect that the compiler can them
optimize out the conditional. A prototype implement can be found
[here](https://github.com/rust-lang/rust/pull/24905).

# Drawbacks

The biggest drawback appears to be that it adds yet another function
to the `Option` enum and it may not be worth the additional cognitive
load it puts on developers to understand when and why they should use
this method.

# Alternatives

Rather than a method on `Option` this can be implemented as a
free-standing method (perhaps in an external library) or instead
developers could just use the `unreachable` intrinsic without a method:

    match x {
        Some(x) => {
           ...
        },
        None => unsafe { intrinsics::unreachable() };
    }

I feel that this is less desirable as it causes right shift in the
code and has poor ergonomics. Forcing developers down this path will
cause them to avoid the simple optimization to keep the code cleaner.

# Unresolved questions

None
