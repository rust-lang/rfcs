- Start Date: 2014-11-17
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Introduce a way to "freeze" the location of a macro expansion in an inexpensive way
and to fetch the appropriate debug info at a later point (for instance from `DWARF` data).

The RFC proposes the following new functionality:

* `debug_location!()` which returns a debug location (`DebugLocationMarker`)
* `inspect_debug_location(loc: DebugLocationMarker)` which returns debug information
  for a debug location.

# Motivation

It has been brought up that `file!`, `line!` and friends are producing significant
executable bloat and are in many situations entirely unnecessary because separate debug
info is available.  While those macros are fine in situations where they are constantly
taken (for instance error logging libraries can benefit from these macros) they are
causing worse overall performance for situations where the error branch is never taken
or error handling never needs the location information.

The wah `debug_location!()` could work, is by recording the instruction pointer and
then later using `DWARF` information to find the debug info.  However it is implemented
as a macro which would allow extra information to be frozen where the expansion happens
to aid this.

The idea would be that an error handling library could emit `debug_location!()` every
where it propagates errors and then generate a traceback when needed.  This would be
a lot more efficient than baking the location info into every error propagation
manually.

# Detailed design

There are a few parts to the proposal.

## The Debug Location Macro

The `debug_location!()` macro expands by the compiler into something that holds enough
information to get a `DebugLocationMarker`.  It would be valid for this to expand into
a dummy marker that cannot actually generate any debug information in release builds
or in versions of Rust that run on platforms that do not have debug info.

For instance on x86 the assumption would be that `DebugLocationMarker` looks like this:

```rust
#[deriving(Clone, Eq, PartialEq)]
struct DebugLocationMarker {
    ip: libc::uintptr_t,
}
```

The expansion of `debug_location()` would directly read the instruction pointer and
store it in the debug location marker.

On other platforms the expansion and marker might look different.  The requirements
for the `DebugLocationMarker` are `Copy`, `Eq` and `PartialEq`.

## The Debug Info

the `inspect_debug_location` function returns a `DebugInfo` object that has as much
debug information as available for the given location:

```rust
struct DebugInfo {
    /// the program counter if available
    pub pc: Option<uint>,
    /// the file if available
    pub file: Option<String>,
    /// the line number if available.
    pub line: Option<uint>,
    /// the column if available.  Chances are, it's not available ever, but even
    /// if it's not possible to get this from DWARF data it might still be a good
    /// idea to have it in the structure in case it gets created by other things.
    pub column: Option<uint>,
    /// the name of the function if available.
    pub function: Option<String>,
    /// the path of the rust module if available.
    pub module_path: Option<String>,
}
```

## Refactoring in the Runtime

The runtime already deals with the debug info but does not expose a good API for it.
In the process of implementing this it might be possible to refactor the runtime a
bit so that the backtrace from a task failure uses the same underlying types as the
proposal here.

# Drawbacks

None.

# Alternatives

As an alternative the debug info could become a trait and allow lazy computation of
the debug info.  It might also be a possibility to provide extra functionality to
locate local variables.
