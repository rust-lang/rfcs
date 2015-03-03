- Start Date: 2015-03-03
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

The goal of this RFC is to add a function to get the current entry's name without the full path.
It has been initialized by @hugoduncan in this [rust-issue](https://github.com/rust-lang/rust/issues/22926).

# Motivation

The [DirEntry](https://github.com/rust-lang/rust/issues/22926) structure can be get from the iterators
[WalkDir](http://doc.rust-lang.org/std/fs/struct.WalkDir.html) and [ReadDir](http://doc.rust-lang.org/std/fs/struct.ReadDir.html) returned by the `walk_dir` and the `read_dir` functions.

It misses a simple function to return the entry's name in addition of the `path` function. It could allow to have
an ever more precise control over directories iterations by simplifying operations. For the moment, we have to
parse the returned path to get the item's name. That's not convenient and that's the aim of this RFC.

# Detailed design

We could store the item's path and name instead of only the full path at the opposite of the current [unix](https://github.com/rust-lang/rust/blob/master/src/libstd/sys/unix/fs2.rs)'s and
[windows](https://github.com/rust-lang/rust/blob/master/src/libstd/sys/windows/fs2.rs)'
current implementation.

Then we can add the `get_name` function easily by returning a copy of the name stored in the `DirEntry` structure.

# Drawbacks

Well, it's still possible to get the item's name without this method...

# Alternatives

We could also implement the Deref trait which would return the item's name. However, it might get a little hard for
a user to understand easily what it does returned while the `get_name` function is very explicit.
