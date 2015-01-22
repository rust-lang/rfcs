- Start Date: 2015-01-22
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Add a macro for "unreachable" whose meaning differs in debug and release builds.

# Motivation

Sometimes the compiler can't tell that a certain part of the code can't be reached.
In some of these situations it would be handy to be able to mark the spot for optimizations
in release builds but be able to debug in debug builds whether the spot is indeed unreachable.

# Detailed design

The macro would evaluate to either `panic!()` or `intrinsics::unreachable()` depending on build type.
Right now this is determined by the `ndebug` variable. If/when `ndebug` is removed,
the same test as in `debug_assert` would be used.

The macro would always be unsafe (ie. in all builds).

# Drawbacks


# Alternatives

Instead of adding another macro, the existing `unreachable` might be modified.
Here are some reasons why this might potentialy be preferable:

+ Right now `std::unreachable` isn't that extremely useful, it's basically an alias to `panic!()`.
  Is the use case of the current `unreachable!()` really so distinct from other `panic!()` use cases?
+ Having 3 "unreachable" things (1 function and 2 macros) all of which do something different might be confusing.

On the other hand:

+ There is a number of `unreachable!()` uses in rustc right now, about 160-ish cases.
+ `unreachable!()` is probably somewhat more convenient than `panic!("unreachable")`.


# Unresolved questions

Please let me know what you think of the alternative option described above.

If you prefer to add a new macro, please let me know what you think of the name.
In my [original PR](https://github.com/rust-lang/rust/pull/21009) I named the new macro `optimize_unreachable`,
[reem/rust-debug-unreachable](https://github.com/reem/rust-debug-unreachable) uses `debug_unreachable`;
you are also welcome to suggest another name if you want.
