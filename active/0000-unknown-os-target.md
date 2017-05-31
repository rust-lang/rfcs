# Summary

Add support for triple of the form `*-unknown-*` that relies on minimal
dependencies and doesn't support dynamic linking. Targeting newlib for the
dependencies - it comes with libc, libm and libstdc++, can be ported to new
platforms very easily, and should be compatible with most other 'larger'
glibc alternatives.

# Motivation

Rust is a very attractive language for embedded systems; it provides
limited unsafety while still giving full control of memory allocation.
Adding an OsUnknown to rustc permits `#[cfg(target_os = "unknown")]`
attributes which should be enough to get libstd building for embedded
targets.

# Detailed design

Add a new OS to librustc for the unknown target. Mostly duplicating all the
code of OsLinux and naming it OsUnknown. Any code for dynamic linking should
error as OsUnknown can't support dynamic linking.

Modify librustc to ignore `#[crate-type = "dylib"];` attributes for
OsUnknown. If `--crate-type=dylib` is specified on the command line, then an
error should be produced. Ignoring the crate attribute is the easiest
solution, it doesn't look like
`#[cfg(target_os = "unknown")] [crate-type = "dylib"];` is supported
(and would look messy).

Subsequent work would be in libstd to get it to build a rlib and then be
able to link against newlib. Majority of the changes are in rtdeps and libc
(adding lots of target_os = unknown). Should this RFC be for the whole
process or just the OsUnknown in librustc?

Newlib supports POSIX like threads and is/can-be fully reentrant. It also
supports C++ exceptions, although it can be built in a -nano spec that drops
this support to minimise size.

# Alternatives

There was a previous pull request for a "none" OS to support the
arm-none-eabi toolchain for ARM-Cortex-M0 procesors. There was an issue
raised with this that .so was used as the dynamic lib extension (but dynamic
linking isn't supported).

Could probably get libstd to build using
`--target *-linux-* --cfg no-dynamic --cfg with-newlib` (because OsUnknown
is mostly just OsLinux).

# Unresolved questions

What is the best way to deal with linking Rust applications without
exception support. Can utilise weak linkage in the fail macro, or build a
custom libstd with `--cfg no-unwind`.

I'm not sure how much work is required to get 'rt' to build against newlib,
nor how compiler-rt will work (for one, it isn't able to work out the target
endianness when bult without `__linux__` and similar defines). 
