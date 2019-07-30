- Feature Name: `no_entry`
- Start Date: 2019-07-23
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Add a new top-level attribute `no_entry`, to omit the platform entry point
symbol, allowing the user to write their own.

# Motivation
[motivation]: #motivation

For some low-level systems-programming use cases, a user needs to have full
control of a process from the very first instruction, by specifying the
entry-point symbol. Examples include kernels or firmware where the entry-point
symbol may represent initial boot code, applications running under an operating
system but with unusual startup requirements, or programs creating or running
in sandbox environments.

The target toolchain will commonly supply an entry-point symbol (e.g.
`_start`) by default. As a result, attempting to define an entry-point symbol
typically results in a link error. Addressing this currently requires linking
manually with an toolchain-specific argument like `-nostartfiles`.

This proposal introduces a top-level attribute `no_entry`, which will cause the
Rust toolchain to omit the entry-point symbol. Specifying `#![no_entry]` allows
(and requires) the user to supply such a symbol.

(Please note that this is entirely distinct from the Rust-supplied startup hook
`#[start]`.)

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Specifying the attribute `#![no_entry]` at the top level of a Rust binary crate
will cause Rust to omit the entry-point symbol when compiling and linking the
binary. When linking, Rust will pass any necessary target-specific arguments to
the toolchain to omit the target's entry-point code, such as `-nostartfiles`.

Binaries built with this attribute set will typically not link unless the user
supplies their own entry-point symbol.

Libraries cannot use this attribute; attempting to do so will produce a
compile-time error stating that `no_entry` only applies to binaries.

A binary built with this attribute set could supply the entry-point symbol
itself, or have the entry-point symbol supplied by a library crate or by a
non-Rust library.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

Until stabilized, the `no_entry` attribute will require a feature-gate named
`no_entry`.

The `no_entry` attribute may be specified as part of a `cfg_attr` declaration,
to omit the entry point in some configurations but not others.

Specifying the `no_entry` attribute on anything other than the top-level module
of a binary crate will produce an error (`warning: crate-level attribute should
be in the root module`).

Declaring an entry-point symbol will typically require a declaration with
`#[no_mangle] pub extern "C"`.

To implement `no_entry`, the compiler can have a default linker argument based
on the type of linker in use, such as `-nostartfiles` for GCC or clang/LLD.
Specific targets which require more specific options can provide those options
via the target configuration structures.

Some targets may be able to support invoking the C library without running the
startup code provided by the C library; other targets may not support this, or
have limitations on such support. Targets that do not support this should
document the target-specific limitations of `no_entry` for their target, may
require `no_std` in addition to `no_entry`, or may not support `no_entry` at
all.

On targets that already don't provide an entry point, the compiler will
silently accept `no_entry`. On targets that cannot support `no_entry`, the
compiler will emit a compile-time error.

# Drawbacks
[drawbacks]: #drawbacks

Rust has several existing attributes related to program startup, and this would
require further careful documentation to distinguish them. However, no existing
attribute serves the function that `no_entry` does.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

Today, people do this either by linking their binary manually, or by specifying
non-portable linker arguments in `.cargo/config`. Introducing the `no_entry`
attribute substantially simplifies creating such programs, and avoids
target-specific build steps.

The name `no_entry` was selected specifically to minimize conflict and
confusion with other Rust features such as `start`, `no_main`, and `main`, none
of which relate to the entry-point symbol.

We could potentially integrate this mechanism with specification of an
alternate entry point. However, this would make it difficult to flexibly
provide the entry point from a library or other mechanism.

We could make this a rustc command-line option, rather than an attribute.
However, that would separate the configuration required to build a crate from
the crate itself. We have substantial precedent for including such
configuration directly in the crate as an attribute.

# Prior art
[prior-art]: #prior-art

We currently have the `no_main` attribute to omit a C `main` function (not the
entry point symbol), the `main` attribute to specify a *Rust* `main` function
other than `fn main()`, and the `start` attribute to specify a Rust-specific
portable startup function that runs after some existing Rust startup code.

We do not currently have any mechanisms to address handling of entry-point
symbols.

Existing programming languages rely entirely on the toolchain for this
functionality. Introducing `no_entry` makes it possible to gain control from
the initial binary entry point without toolchain-specific code, consistent with
other Rust efforts to provide some uniformity across targets and toolchains.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

Can we use common code to implement this for all targets using a given
toolchain family (`LinkerFlavor`), and minimize target-specific implementation
code?

# Future possibilities
[future-possibilities]: #future-possibilities

We should simplify the declaration of an entry-point symbol, such as by
providing an `#[entry]` attribute. The symbol itself would still have a
non-portable signature, but having #[entry] would allow using exclusively Rust
cfg to handle portability, rather than also needing linker arguments. Rust
could also (optionally) detect and warn about entry points with an unexpected
signature for the target.  This would also make it easier for Rust library
crates to provide entry points.  (This differs from #[start], which provides a
semi-portable entry point that still has Rust code run before it.)

I'd also like to add an attribute to pass `-nodefaultlibs` to the linker.
Unlike `no_entry`, this would typically require `no_std` on many targets, as
std often depends on the C library.
