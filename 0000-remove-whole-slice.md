- Start Date: 2014-11-29
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Remove the `[]` notation for taking a whole slice.

# Motivation

Since we accepted [RFC 235](text/0235-collections-conventions.md) we can take a
slice of a vector or string (e.g., `x`) by writing `&*x`. In the future (#241)
this may become `&x`. It therefore seems unnecessary to have the additional
`x[]` syntax for getting a slice. In [RFC 439](text/0439-cmp-ops-reform.md), we
change the implementation of slice notation to use a first class range. There is
then no obvious way to represent whole slices. Personally, I have found `x[]`
hard to read in code and easily confused with array notation.

To avoid a hackey range fix, remove duplication, and avoid backwards
compatibility risk, we should remove the `[]` notation. (We can always add it
back in later without breaking backwards compatibility, if we decide this was a
mistake).

# Detailed design

`expr[]` would no longer parse as an expression. The corresponding methods in the
`Slice` traits would be removed.

# Drawbacks

Using `&*` might be off-putting/unintuitive for new users (although they'll have
to get used to it for cross-borrowing pretty quickly, and hopefully this will
only be short term until we get deref coercions).

If a collection implements `Slice` but not `Deref`, then there is no nice way to
get a slice representation (presumably the collection would allow `as_slice()`).
I imagine most collections that can be represented as slices should implement
`Deref` though, in order to be consistent with `Vec` and `String`.

# Alternatives

Leave as is.

Replace `[]` with `[..]`, which is at least easier to read, although doesn't
solve the problem with duplication.

# Unresolved questions

What is the impact of this change? We didn't mass convert code in the compiler
or libraries to use `[]`, so the impact should be quite small, although it has
been used to some extent. I don't know how much it is used in other code. It is
quite hard to find and replace, since `[]` is also used for an empty array. It
would be easy to instrument the compiler to gather this info though (and to give
deprecation warnings and/or advice for fixing bustage).
