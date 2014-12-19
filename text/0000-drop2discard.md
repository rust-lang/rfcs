- Start Date: 2014-12-19
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Rename `std::mem::drop` to `std::mem::discard`.

# Motivation

Among learners, there is frequent confusion surrounding the difference between `std::mem::drop` and `Drop#drop`. The former is a stdlib function that is used to take ownership of a value and let it immediately go out of scope, while the latter is Rust's mechanism for destructors. It's very easy for a beginner to conflate the two, given that causing a value to go out of scope with `std::mem::drop` will run its destructor. And even advanced users are forced to quantify which `drop` they are talking about in casual discussion. Finally, UFCS muddying the distinction between methods and functions will only make talking about these two functions more difficult.

See [this reddit thread](http://www.reddit.com/r/rust/comments/2ov7e2/a_beginners_thoughts_on_programming_languages/cmqt3lm) for one of many instances of a beginner being confused by this.

To relieve us of this conflation, I propose that we rename `std::mem::drop`. As a replacement I propose `std::mem::discard`. See the section below for alternatives.

# Detailed design

This change should be a straightforward find-and-replace.

# Drawbacks

Because beginners are unworthy of learning Rust easily and must be punished.

# Alternatives

I considered `take` and `consume` before `discard`. `take` was rejected because of its usage on iterators, and `consume` was rejected because of its widespread usage on buffers. In contrast, `discard` is not used anywhere in the stdlib. Given that our goal is to reduce mental name collisions, this seemed the best alternative.

# Unresolved questions

Are all NP-problems actually P-problems?
