- Feature Name: libc_types
- Start Date: 2016-11-03
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Move the basic type definitions of the `libc` crate (`c_int`, `c_ulong`, etc) into a separate `libc_types` crate which does not require linking to the platform C library.

# Motivation
[motivation]: #motivation

Sometimes it is desirable to call C code through FFI or use C data structures in an environment without a C library. This is specified in the C standard as a freestanding environment. From the [GCC documentataion](https://gcc.gnu.org/onlinedocs/gcc/Standards.html):

> The ISO C standard defines (in clause 4) two classes of conforming implementation. A conforming hosted implementation supports the whole standard including all the library facilities; a conforming freestanding implementation is only required to provide certain library facilities: those in <float.h>, <limits.h>, <stdarg.h>, and <stddef.h>; since AMD1, also those in <iso646.h>; since C99, also those in <stdbool.h> and <stdint.h>; and since C11, also those in <stdalign.h> and <stdnoreturn.h>. In addition, complex types, added in C99, are not required for freestanding implementations.
>
> The standard also defines two environments for programs, a freestanding environment, required of all implementations and which may not have library facilities beyond those required of freestanding implementations, where the handling of program startup and termination are implementation-defined; and a hosted environment, which is not required, in which all the library facilities are provided and startup is through a function int main (void) or int main (int, char *[]). An OS kernel is an example of a program running in a freestanding environment; a program using the facilities of an operating system is an example of a program running in a hosted environment.

The obvious use case for such a crate would be kernels and other bare-metal code which need to link to existing C libraries. Although such code can simply use raw Rust types (`i32` instead of `c_int` for example), this is unergonomic.

A more interesting case is that of bindings for C libraries which can work in a freestanding environment. Bindings for such libraries are typically defined using types from the `libc` crate, which prevents them from being used in a freestanding environment without a C library.

Finally, a separate `libc_types` crate would allow Rust on Windows to avoid linking to the MS CRT entirely. This would make Rust executables more portable since they would not require a user to install a Visual Studio redistributable package.

Relevant discussions on [internals](https://internals.rust-lang.org/t/solve-std-os-raw-c-void/3268) and on [Github](https://github.com/rust-lang/rust/issues/31536).

# Detailed design
[design]: #detailed-design

The following types will be moved to a separate `libc_types` crate:

```rust
pub enum c_void;

pub type int8_t;
pub type int16_t;
pub type int32_t;
pub type int64_t;
pub type uint8_t;
pub type uint16_t;
pub type uint32_t;
pub type uint64_t;

pub type c_schar;
pub type c_uchar;
pub type c_short;
pub type c_ushort;
pub type c_int;
pub type c_uint;
pub type c_float;
pub type c_double;
pub type c_longlong;
pub type c_ulonglong;
pub type intmax_t;
pub type uintmax_t;

pub type size_t;
pub type ptrdiff_t;
pub type intptr_t;
pub type uintptr_t;
pub type ssize_t;

pub type c_long;
pub type c_ulong;
```

To preserve backward compatibility, these types will be re-exported by the `libc` crate. This is not a breaking change since the `c_void` type still only comes from a single source, so there will not be conflicting definitions. Thus only a minor version bump is required, which avoids extensive breakage across the ecosystem similar to what happened when the `libc` version was bumped to 0.2.

# Drawbacks
[drawbacks]: #drawbacks

- Adds an additional crate to the standard library.

# Alternatives
[alternatives]: #alternatives

- Do nothing. Freestanding code will have to use standard rust types and write their own bindings for C libraries.

# Unresolved questions
[unresolved]: #unresolved-questions

- The exact crate name is subject to the usual bikeshedding.
