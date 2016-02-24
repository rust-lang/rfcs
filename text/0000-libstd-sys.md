- Feature Name: libstd_sys
- Start Date: 2016-02-16
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Refactor the standard library implementation in a backward-compatible way, such
that most platform-specific code resides in submodules of `std::sys`, one
submodule per platform, with the goal of making it easier to port the standard
library to systems much different than POSIX and Windows.

# Motivation
[motivation]: #motivation

Currently, the Rust standard library is organized such that large amounts of
platform-specific code coexists in the same modules as platform-independent
code, and the code for different platforms is interspersed. When adding a new
platform to Rust, one must add or change code in many different modules to
support the new platform.

With the current state of libstd, it is hard to see which parts depend on
external operating-system provided libraries like libc, libpthread, etc. This in
turn makes it difficult to begin work on implementing the standard library
without using these libraries. For example, it is hard to determine whether it
is better for the pc-windows-msvc targets to use libc or whether they should
depend only on Operating System APIs (the Win32 API, or even a smaller subset of
the Win32 API). As another example, it is difficult to estimate the practicality
of porting the full Rust standard library to platforms that are very different
from POSIX and Win32, such as microkernel architectures and even non-POSIX
Rust-based monolithic kernels. It is even difficult to just estimate how much
work would by needed to provide a non-POSIX implementation of the standard
library using Rust code directly invoking Linux syscalls (bypassing libc and
libpthread).

It is expected that if/when this RFC is implemented, the standard library will
be organized in such a way that it is straightforward to understand what work is
needed for a brand new platform or a new approach to an existing platform.
Further, we expect that people porting Rust to new platforms or experimenting
with alternative implementation strategies for existing platforms will have a
much easier time maintaining their work as libstd changes in parallel. That is,
it is expected that such experiments will become both less disruptive to
mainline Rust development and also that mainline Rust development will become
less disruptive to such experiments.

It is NOT expected that there will be a stable interface between the portable
parts of the standard library and the platform-specific parts. The idea is to
*reduce* maintanence of ports in progress, not to *eliminate* it.

It is NOT expected that there will be an official way to plug in an out-of-tree
platform port into libstd. In particular, it is NOT a goal to facilitate
end-users of the production Rust toolchain being able to substitute an
alternative standard library implementation different from the one provided with
the official toolchain.

It is NOT expected that the strategies for implementing the Linux port or the
Windows ports or any other platform will change (e.g. by removing the libc
dependencies). This work may help us evaluate proposals for such implementation
changes in an informed manner, though.

It is NOT expected that the work will be done all at once. Not only should the
work be split into small, easy-to-understand commits, but those commits should
land independently and on their own merits.

# Detailed design
[design]: #detailed-design

This RFC proposes identifying a common abstraction of the interfaces used by
libstd to interact with the underlying operating system. One module at a time
will be audited for platform-dependent code (usually contained in submodules
and functions guarded by `#[cfg(...)]` attributes) and moved to an appropriate
area under `libstd::sys`. The platform-independent code will be adjusted to use
the new interface.

There are two main ways to implement the static interface.

## Trait approach

Define traits for each piece of functionality needed. Platform-independent code
will call into the trait methods and generally be ignorant of the underlying
implementation. Many of these traits will be "static" in that they will contain
no methods that take `self`.

### Advantages

- Promotes exposing a well-defined common interface.
- Porting libstd to a new platform is as simple as implementing a list of
  traits.

### Disadvantages

- Introduces a fair amount of boilerplate into the platform-independent code.
    - Importing the traits into scope or from a `sys::prelude::*`.
    - Explicit associated type specifications may be required, e.g.
      `<imp::Filesystem as sys::Filesystem>::File`

## Module approach

We may instead simply expose the necessary types and methods under a module.
This simplifies code at the call site, but makes it easier for non-generic
interfaces to be accidentally relied upon and break builds for other platforms.
A mock interface may be necessary in order to allow the compiler and tests to
assert that the abstractions have not been breached.

# Drawbacks
[drawbacks]: #drawbacks

The main drawback is that this is a lot of work that will require many changes
(roughly +5,000/-5,000 over 200 files). This may disrupt other ongoing work.
However, the incremental approach should minimize this disruption.

# Alternatives
[alternatives]: #alternatives

The alternative is to do nothing.

# Unresolved questions
[unresolved]: #unresolved-questions

- Additional work may be done to pull `libstd::sys` out into its own `libsys`
library in order to expose a better defined separation of the interfaces to the
underlying OS. This is not trivial as there are currently some circular
dependencies between the platform-dependent code and libstd types. It is likely
best to move to `libstd::sys` first, and then evaluate the necessary work and
advantages of moving to a potential `libsys` afterward.
- `libstd::sys` is already somewhat used for a similar purpose, but is not
currently well-isolated. A lot of that functionality will be moved into a
`sys::common` module or similar but may generate some churn as we incrementally
move code into the same module namespace.
- Bikeshed arguments over `libstd::sys` vs `libstd::platform` or some other
alternative.
