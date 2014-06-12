- Start Date: 2014-06-12
- RFC PR #: (leave this empty)
- Rust Issue #: (leave this empty)

# Summary

`unsafe` blocks are one of Rust's most important features. However, the name
"unsafe" doesn't properly communicate the intention of unsafe blocks. I propose
we change the "unsafe" keyword to "trusted," initially deprecating it.

# Motivation

When explaining Rust to someone who doesn't already know it, there's often
confusion around `unsafe`. To most of the readers I've spoken with, `unsafe`
means "This code is not safe." This understanding is incomplete, however.
`unsafe` actually means "This code is not able to be determined to be safe by
the compiler, but I promise you that it is." This is a significant difference.
As indicated in
[numerous](https://twitter.com/Zalambar/status/477198693783724032)
[conversations](https://news.ycombinator.com/item?id=7885502), this causes
confusion.

Furthermore, `unsafe` only [unrestricts certain
behaviors](http://static.rust-lang.org/doc/0.10/rust.html#behavior-considered-unsafe).
`unsafe` Rust code is still significantly safer than C. `unsafe` implies that
_no_ safety checks occur. This is incorrect.

# Detailed design

Replace the "unsafe" keyword with a new keyword, "trusted." For ease of
transition, "unsafe" should be deprecated, and throw a warning on use. "unsafe"
can then be removed before 1.0.

# Drawbacks

This would basically invalidate all current code which uses the `unsafe`
keyword. There's quite a bit of that code. Considering the fix is a simple find
and replace, I don't believe this drawback is important enough to not change
the keyword. In addition, a simple deprecation notice means that the older code
wouldn't be strictly invalid, just throw additional warnings.

# Alternatives

- trust
- unchecked
- wrap_unsafe

# Unresolved questions

I am not 100% sure that "trusted" is the best possible name.
