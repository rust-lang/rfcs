- Feature Name: Space-friendly arguments
- Start Date: 2016-02-23
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Add `-C link-arg` and `-C llvm-arg` command line arguments which correspond to `-C link-args` and `-C llvm-args` but takes a single argument which accepts spaces.

# Motivation
[motivation]: #motivation

It is common to pass paths to the linker. Paths may contain spaces and so `-C link-args` is insufficient for this. MSVC's linker also has [atleast one argument which require spaces](https://msdn.microsoft.com/en-us/library/ew0y5khy.aspx).
This would also fix [#30947](https://github.com/rust-lang/rust/issues/30947).
We don't control what arguments the linker or LLVM accepts so we shouldn't limit them to not use spaces.

# Detailed design
[design]: #detailed-design

We add two new arguments `-C link-arg` and `-C llvm-arg` which can be used multiple times and combined with the old `-C link-args` and `-C llvm-args`. The value passed to them represent a single argument to be passed to the linker or LLVM respectively. The order of the arguments should be preserved when passing them on.
This design corresponds to clang's with `-Xlinker <val>` and `-mllvm <val>`.

# Drawbacks
[drawbacks]: #drawbacks

This results in multiple ways to pass on arguments (without spaces) to the linker and LLVM.

# Alternatives
[alternatives]: #alternatives

None.

# Unresolved questions
[unresolved]: #unresolved-questions

None.