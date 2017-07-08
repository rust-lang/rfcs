- Feature Name: Destructure structs implementing Drop
- Start Date: 2017-07-08
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Allow destructuring of structs that implement Drop.

# Motivation
[motivation]: #motivation

Currently destructuring of structs that implement Drop, requires to read non-copy fields via 
`ptr::read(&struct.field)` followed by `mem::forget(struct)`.

This leaves more room for error, than there would have to be: 
1.  forgetting to read all fields, possibly creating a memory leak,
2.  acidentally working with `&struct` instead of `struct`, leading to a double free.
     (The fields are copied, but mem::forget only forgets the reference.)

Allowing to destructure these types would ensure that:
1.  unused fields are dropped,
2.  only owned structs can be destructured.

# Detailed design
[design]: #detailed-design

As 
previously shown destructuring of structs with destructures may create unsoundness
[[1]](https://github.com/rust-lang/rust/issues/3147)
[[2]](https://github.com/rust-lang/rust/issues/26940).

One possible solution would be to make this an `unsafe` operation and only allow it inside an unsafe `block`.
Another possiblity would be to restrict it to modules that are allowed to impement the type in question.

Either restriction would prevent the issue of [[2]](https://github.com/rust-lang/rust/issues/26940).

# How We Teach This
[how-we-teach-this]: #how-we-teach-this

# Drawbacks
[drawbacks]: #drawbacks

Why should we *not* do this?

# Alternatives
[alternatives]: #alternatives

The trivial alternative is to keep it as is, and keep using `ptr::read` & `mem::forget`.
This could possibly be automated with a macro.

# Unresolved questions
[unresolved]: #unresolved-questions

Which restrictions are required to make this sound?
