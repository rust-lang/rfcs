- Start Date: 2014-09-19
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Rename the `uint` and `int` integer types to make clear that they are not the
default integer types. Optionally add default integer types for “don’t
care”-cases.

# Motivation

Rust’s `uint` type is defined to be the equivalent of C’s `uintptr_t` type.
This is good for storing indices, pointers, etc. On the other hand, using it as
a default integer type does not make much sense, there is e. g. no reason for
the default being 32bit on x32 and 64bit on x86_64.

Rust should be explicit, so fixed-size integer types should be preferred over
the architecture-dependant `uint` so it is clear how overflow is handled.

There are however cases where a default integer type is desired. This includes
code as simple as `println!("{}", 0);`. The current situation is that `int` is
picked, just because it sounds like the default although it is not a good one.

Rust’s default should be fast and reasonably sized on each architecture so that
code dealing only with small integers in a style similar to C is also
similarily fast to C – this is not an issue for local variables which might be
made smaller by LLVM, but function calls with structs containing integers
suffer from that.

# Detailed design

Rename the `uint` and `int` integer types. Several suggestions about
alternative namings have been made, this still needs bikeshedding. Some
proposed names were:

- `uintptr` `intptr` (like C)
- `size` `ssize` (also like C)
- `usize` `isize` (Rustified C)
- `uindex` `index`

Remove fallback to pointer-sized integer types.

Optionally introduce new integer types `uint` and `int` that represent
reasonable defaults regarding speed and size. In case these are introduced the
the integer inference could use these as fallbacks.

# Drawbacks

The `uint` and `int` types will be renamed which is a big backward-incompatible
change. If the new integer types are introduced, there will be more integer
types.

# Alternatives

It would be possible to stay with the current `uint`, `int` integer types. In
this case however, it does not seem to be reasonable to have the integer
inference fall back to pointer-sized integer types, they should rather fall
back to `i32` or a new default type. Additionally the docs, tests and the guide
need to be updated as almost all of these use `int` as if it were a good
“don’t care”-default.

# Unresolved questions

What will the old `uint` and `int` be called? Will there be new, default
integer types `uint`, `int`? What size will they have?
