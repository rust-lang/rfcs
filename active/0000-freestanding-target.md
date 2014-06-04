- Start Date: 2014-06-03
- RFC PR #: (leave this empty)
- Rust Issue #: (leave this empty)

# Summary

Add support for the "unknown" OS in target triples, and disable split stack
checking on them by default.

# Motivation

One of Rust's important use cases is embedded, OS, or otherwise "bare metal"
software. At the moment, we still depend on LLVM's split-stack prologue for
stack safety. In certain situations, it is impossible or undesirable to
support what LLVM requires to enable this (on x86, a certain thread-local
storage setup). We also link to some libraries unconditionally, making it
difficult to produce freestanding binaries.

# Detailed design

A target triple consists of three strings separated by a hyphen, with a
possible fourth string at the end preceded by a hyphen. The first is the
architecture, the second is the "vendor", the third is the OS type, and the
optional fourth is environment type. In theory, this specifies precisely what
platform the generated binary will be able to run on. All of this is
determined not by us but by LLVM and other tools. When on bare metal or a
similar environment, there essentially is no OS, and to handle this there is
the concept of "unknown" in the target triple.  When the OS is "unknown" many
features can be assumed to be missing, such as dynamic linking (which requires
surprisingly complicated runtime support), thread-local storage (threads are
inherently an OS concept), presence of any given IO routines, etc.

When the compiler encounters such a target, it will never attempt to use
dynamic linking, will disable emitting the split stack prologue, and will not
link to any support libraries. These targets, of the form `*-unknown-unknown`,
will never be a valid platform for rustc itself to run on, and can only be
used for cross-compiling a crate. Statically linking to other crates will
still be valid.

"Support libraries" includes libc, libm, and compiler-rt. libc and libm
are clearly not available on the unknown OS, and the crate author will need to
provide compiler-rt or a similar library themselves if they use a feature
requiring it. The goal is to have 0 dependencies on the environment, at
compile time and runtime, reducing unnecessary surprises for the freestanding
author and allowing the use of only rustc to compile the crate, rather than
manually assembling and linking LLVM bitcode. Providing linker arguments
manually is probably unavoidable in these cases.

# Drawbacks

All function calls become unsafe in such a target, since guards against stack
overflow are not inserted by the compiler. The crate author must be careful
not to overflow the stack, or set up their own stack safety mechanism.

# Alternatives

We could allow disabling split stacks on a per-crate basis. Then calling
functions from that crate is then unsafe, and we would need to compensate for
this somehow. We would still need to add a way to not link to
libc/libm/copmiler-rt.
