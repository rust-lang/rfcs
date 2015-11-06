- Feature Name: cfg_sizeof_alignof
- Start Date: 2015-11-06
- RFC PR:
- Rust Issue:

# Summary
[summary]: #summary

Add syntax for the `#[cfg(...)]` attribute to enable conditional compilation
dependent on size and alignment of primitive types used in FFI.

# Motivation
[motivation]: #motivation

In Rust, the programmer is largely shielded from per-platform variation on
size and alignment of data types. On one hand, the layout of data structures
is deliberately left unspecified and safe aliasing of memory is only allowed
through language-sanctioned means, which makes most of the size/offset
tricks impractical. On the other hand, most of the primitive types have
precisely defined bit width. The exceptions are pointers and `isize`/`usize`;
notably, configuration parameter `target_pointer_width` already exists
to support alternative definitions based on the actual pointer size.

When interfacing with foreign libraries, however, alternative definitions or
implementations sometimes need to be provided to address the variation
in the data types used in FFI calling conventions. For example, the developer
of an idiomatic Rust binding interface may wish to use `i64` as a consistently
defined type for a function parameter that represents a `long` parameter in
the corresponding C function. On targets where `long` is 32-bit wide, casting
and additional range checks may need to be provided by the bindings; on
targets where `libc::c_long` is defined as `i64`, conversely, explicit
casting will cause compiler warnings, which would better be suppressed for
this specific configuration.

In the near term, before interop with [C unions][rfcs#877] and
[bit fields][rfcs#314] is addressed, developers of foreign library bindings
need a way to represent the C data types containing unions or bit fields
with stand-in structures matching those in size and alignment. Currently,
this implies choosing a list of supported target platforms and providing
target-specific definitions as appropriate. Conditional checks for size and
alignment parameters of the ABI would help make such definitions more openly
and flexibly portable.

[rfcs#877]: https://github.com/rust-lang/rfcs/issues/877
[rfcs#314]: https://github.com/rust-lang/rfcs/issues/314

# Detailed design
[design]: #detailed-design

Extend `#[cfg(...)]` syntax with parenthesized clauses named `sizeof` and
`alignof`, which can be used to assert size and alignment of C primitive
types, as specified by the target ABI:

```rust
#[cfg(sizeof(c_int = 4))]
const I32_FORMAT: &'static [u8] = b"%d\0";
```

The size and alignment is given in bytes. The names of the C types are
as defined by crate `libc`. Rust types `isize` and `usize` can also be
checked, and `ptr` represents any pointer-based types, providing a
more concise and flexible alternative to `target_pointer_width`.

Multiple types can be checked in a single `sizeof` or `alignof`
clause, and the logical combinators of `cfg` can be applied to
express more complex conditions:

```rust
    #[cfg(any(sizeof(c_uint = 4, ptr = 4),
              sizeof(c_uint = 8, ptr = 8)))]
    #[repr(C)]
    struct UnionTwoUIntsOrPointer {
        // The largest variant on 32-bit or ILP64
        // is two uints, alignment also fits
        dummy: [libc::c_uint; 2]
    }
```

# Drawbacks
[drawbacks]: #drawbacks

None known.

# Alternatives
[alternatives]: #alternatives

The existing practice of target-bound definitions can be continued, thus
often implicitly limiting the set of supported targets for each crate
where such definitions are used. This obscures the intent of conditional
attributes with platform-specific knowledge and may allow errors to sneak in
(e.g. `#[cfg(windows)]` might be mistakenly used to mean 32-bit Windows,
but it happens to work on `x86_64` for all but pointer-based types).

Sizes and alignments could be specified in bits. This would bring mnemonical
mismatch with `sizeof`/`alignof` in C. No platforms with non-8-bit bytes
or sub-byte addressing are expected to be supported by Rust in the foreseeable
future, so there are no practical benefits in bitwise units.

# Unresolved questions
[unresolved]: #unresolved-questions

There are two kinds of alignment that might matter in different
considerations: the minimal allowed alignment and the preferred alignment.
In the understanding of the author, the minimal alignment is the one used in C
structure layout and predominantly the one that matters in FFI.
