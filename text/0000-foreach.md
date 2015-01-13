- Start Date: 2015-13-01
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

This is a proposal to add .foreach to iterators.

# Motivation

After a chain of transformations on an iterator it might be desirable to have a side effecting
operation that consumes the iterator. Adding .foreach to iterators is proposed, which is a common
idiom in other languages for performing this operation. While the same effect can be achieved with a
for loop, in general is considered that flat is better than nested and leads to more readable code.

# Detailed design

The design is simple, just syntacting suggar around a loop that consumes the iterator as seen in
this PR.

https://github.com/rust-lang/rust/pull/21098

# Drawbacks


# Alternatives


# Unresolved questions

It has also been discussed in reddit and in the forum:


http://www.reddit.com/r/rust/comments/2s5jjs/does_having_foreach_implemented_for_iterators/

http://discuss.rust-lang.org/t/add-foreach-method-to-iterators-for-side-effects/1312/3

